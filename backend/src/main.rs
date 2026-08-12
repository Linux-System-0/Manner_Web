// Manner_Web - 可以在 Linux 系统上运行的企业管理系统
// Copyright (C) 2026 Linux-System-0(Github) / 一架在Linux上起飞的A320(Bilibili) <ls0_1@qq.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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

    // 存量「员工级直接授权」→ 角色授权迁移（employee_permissions 表存在时执行，幂等）。
    match migrate_direct_permissions(&pool).await {
        Ok(()) => tracing::info!("直接授权迁移完成（employee_permissions 已移除）"),
        Err(e) => tracing::error!(error = ?e, "直接授权迁移失败"),
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

/// 存量「员工级直接授权」→ 角色授权迁移（一次性、幂等）：
/// - 持有全部权限的员工绑定 super_admin 内置角色；
/// - 其余持有部分权限的员工生成私有角色（role id 取员工 id）承载其权限；
/// - 全员 perm_version 递增强制下次请求重算有效授权；
/// - 迁移完成后 DROP employee_permissions 表（下次启动表不存在则跳过）。
async fn migrate_direct_permissions(pool: &sqlx::MySqlPool) -> Result<(), anyhow::Error> {
    let has_table: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM information_schema.TABLES \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'employee_permissions'",
    )
    .fetch_one(pool)
    .await?;
    if has_table == 0 {
        return Ok(());
    }

    // super_admin 判定基准：持有「重构前全部权限码」的存量员工视为原管理员。
    // 本次重构新增 role:manage，旧管理员不可能持有它，须将其排除在基准外。
    let total_legacy: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM permissions WHERE code != 'role:manage'")
            .fetch_one(pool)
            .await?;

    if total_legacy > 0 {
        // 1) 持有全部存量权限 → super_admin
        sqlx::query(
            "INSERT IGNORE INTO employee_roles (employee_id, role_id) \
             SELECT ep.employee_id, '00000000-0000-0000-0000-000000000001' \
             FROM employee_permissions ep \
             GROUP BY ep.employee_id \
             HAVING COUNT(DISTINCT ep.permission_id) = ?",
        )
        .bind(total_legacy)
        .execute(pool)
        .await?;

        // 2) 其余持有部分权限 → 私有角色
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT ep.employee_id, e.username \
             FROM employee_permissions ep \
             JOIN employees e ON e.id = ep.employee_id \
             GROUP BY ep.employee_id, e.username \
             HAVING COUNT(DISTINCT ep.permission_id) < ?",
        )
        .bind(total_legacy)
        .fetch_all(pool)
        .await?;

        for (emp_id, username) in rows {
            sqlx::query(
                "INSERT IGNORE INTO roles (id, name, is_system, scope_type, description) \
                 VALUES (?, ?, 0, 'all', '存量直接授权迁移生成的私有角色')",
            )
            .bind(&emp_id)
            .bind(format!("直接授权-{}", username))
            .execute(pool)
            .await?;

            sqlx::query(
                "INSERT IGNORE INTO role_permissions (role_id, permission_id) \
                 SELECT ?, permission_id FROM employee_permissions WHERE employee_id = ?",
            )
            .bind(&emp_id)
            .bind(&emp_id)
            .execute(pool)
            .await?;

            sqlx::query(
                "INSERT IGNORE INTO employee_roles (employee_id, role_id) VALUES (?, ?)",
            )
            .bind(&emp_id)
            .bind(&emp_id)
            .execute(pool)
            .await?;
        }
    }

    // 3) 全员 perm_version 递增，强制下次请求重算有效授权（即时生效）。
    sqlx::query("UPDATE employees SET perm_version = perm_version + 1")
        .execute(pool)
        .await?;

    // 4) 移除旧的直接授权表。
    sqlx::query("DROP TABLE employee_permissions")
        .execute(pool)
        .await?;

    Ok(())
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
