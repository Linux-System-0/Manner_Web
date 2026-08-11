mod config;
mod db;
mod error;
mod handlers;
mod middleware;
mod models;
mod services;
mod utils;

use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let cfg = config::Config::from_env();

    init_logging(&cfg.log_level);

    for warning in cfg.security_warnings() {
        tracing::warn!(warning = %warning, "安全配置提醒");
    }

    let pool = db::init_pool(&cfg.database_url).await;

    run_migrations(&pool).await;

    // 敏感字段静态加密迁移：存量明文 → 密文（幂等，仅处理无 enc:v1: 前缀的字段）。
    match crate::utils::crypto::migrate_sensitive_fields(&pool, &cfg.field_enc_key).await {
        Ok(n) => tracing::info!(migrated = n, "敏感字段加密迁移完成"),
        Err(e) => tracing::error!(error = ?e, "敏感字段加密迁移失败"),
    }

    // F-01: 预生成与 BCRYPT_COST 同开销的假哈希，供登录「用户名不存在」分支做等时校验。
    let login_dummy_hash = crate::services::auth::hash_password(
        &uuid::Uuid::new_v4().to_string(),
        cfg.bcrypt_cost,
    )
    .expect("failed to generate dummy bcrypt hash");

    // F-02: 登录失败节流器（真实 IP + 用户名双维度）。
    // 限流参数优先取 system_settings 中的配置（可在系统设置界面调整），未配置时回退环境变量默认值。
    let mut login_max_failures = cfg.login_max_failures;
    let mut login_lock_window_secs = cfg.login_lock_window_secs;
    if let Ok(Some(v)) = sqlx::query_scalar::<_, String>(
        "SELECT setting_value FROM system_settings WHERE setting_key = 'login_max_failures'",
    )
    .fetch_optional(&pool)
    .await
    {
        if let Ok(n) = v.parse::<usize>() {
            login_max_failures = n;
        }
    }
    if let Ok(Some(v)) = sqlx::query_scalar::<_, String>(
        "SELECT setting_value FROM system_settings WHERE setting_key = 'login_lock_window_secs'",
    )
    .fetch_optional(&pool)
    .await
    {
        if let Ok(n) = v.parse::<u64>() {
            login_lock_window_secs = n;
        }
    }

    let login_throttle = std::sync::Arc::new(tokio::sync::Mutex::new(
        crate::middleware::ratelimit::LoginThrottle::new(
            login_max_failures,
            login_lock_window_secs,
        ),
    ));

    let state = middleware::auth::AppState {
        pool,
        config: cfg.clone(),
        login_dummy_hash,
        login_throttle,
    };

    let app = handlers::build_router(state);
    let make_service = crate::middleware::strip_allow::StripAllowMakeService::new(
        app.into_make_service_with_connect_info::<SocketAddr>(),
    );

    let addr: SocketAddr = format!("{}:{}", cfg.server_host, cfg.server_port)
        .parse()
        .expect("Invalid server address");

    tracing::info!("Server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, make_service).await.unwrap();
}

fn init_logging(level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    if std::env::var("PROFILE").unwrap_or_default() == "production" {
        let file_appender = tracing_appender::rolling::daily("logs", "manner.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_writer(non_blocking)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .init();
    }
}

async fn run_migrations(pool: &sqlx::MySqlPool) {
    let sql = include_str!("../sql/init.sql");
    for statement in sql.split(';') {
        let trimmed = statement.trim();
        if !trimmed.is_empty() && !trimmed.starts_with("CREATE DATABASE") && !trimmed.starts_with("USE") {
            if let Err(e) = sqlx::query(trimmed).execute(pool).await {
                tracing::warn!(error = ?e, statement = %trimmed.chars().take(80).collect::<String>(), "Migration statement");
            }
        }
    }
    if let Err(e) = sqlx::query("SELECT 1").execute(pool).await {
        tracing::error!("Database connection failed after migration: {:?}", e);
    }
    tracing::info!("Database migrations completed");
}
