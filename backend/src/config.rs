use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub bcrypt_cost: u32,
    pub token_expire_minutes: i64,
    /// refresh 续期令牌有效期（天）。过期后需重新登录。
    pub refresh_token_expire_days: i64,
    pub log_level: String,
    pub server_host: String,
    pub server_port: u16,
    pub upload_dir: String,
    pub log_file: String,
    pub cors_allowed_origins: Vec<String>,
    pub login_max_failures: usize,
    pub login_lock_window_secs: u64,
    /// F15: 登录 Cookie 是否携带 Secure 标志（生产 HTTPS 时设为 true）。
    pub cookie_secure: bool,
}

pub const DEFAULT_JWT_SECRET: &str = "your-super-secret-key-change-in-production";

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            jwt_secret: {
                let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
                if secret == DEFAULT_JWT_SECRET {
                    panic!("JWT_SECRET 仍为公开默认值，拒绝启动：请设置强随机密钥（建议 >=32 字符）");
                }
                secret
            },
            bcrypt_cost: env::var("BCRYPT_COST")
                .unwrap_or_else(|_| "12".into())
                .parse()
                .expect("BCRYPT_COST must be a number"),
            token_expire_minutes: env::var("TOKEN_EXPIRE_MINUTES")
                .unwrap_or_else(|_| "30".into())
                .parse()
                .expect("TOKEN_EXPIRE_MINUTES must be a number"),
            refresh_token_expire_days: env::var("REFRESH_TOKEN_EXPIRE_DAYS")
                .unwrap_or_else(|_| "7".into())
                .parse()
                .expect("REFRESH_TOKEN_EXPIRE_DAYS must be a number"),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "debug".into()),
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".into())
                .parse()
                .expect("SERVER_PORT must be a number"),
            upload_dir: env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".into()),
            log_file: env::var("LOG_FILE").unwrap_or_else(|_| "./logs/manner.log".into()),
            cors_allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:5173,http://127.0.0.1:5173".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            login_max_failures: env::var("LOGIN_MAX_FAILURES")
                .unwrap_or_else(|_| "5".into())
                .parse()
                .expect("LOGIN_MAX_FAILURES must be a number"),
            login_lock_window_secs: env::var("LOGIN_LOCK_WINDOW_SECS")
                .unwrap_or_else(|_| "900".into())
                .parse()
                .expect("LOGIN_LOCK_WINDOW_SECS must be a number"),
            cookie_secure: env::var("COOKIE_SECURE")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        }
    }

    /// 安全加固项校验，返回需要提醒的配置问题列表。
    pub fn security_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.jwt_secret == DEFAULT_JWT_SECRET {
            warnings.push("JWT_SECRET 仍为默认值，请改为强随机密钥".to_string());
        }
        if self.jwt_secret.len() < 32 {
            warnings.push("JWT_SECRET 长度过短（建议至少 32 字符随机值）".to_string());
        }
        if self.server_host == "0.0.0.0" {
            warnings.push("SERVER_HOST 绑定 0.0.0.0，服务对所有网卡暴露；如非必要请改为 127.0.0.1".to_string());
        }
        if self.cors_allowed_origins.contains(&"*".to_string()) {
            warnings.push("CORS_ALLOWED_ORIGINS 含通配符 *，请改为显式 Origin 白名单".to_string());
        }
        // F-23 遗留:非本机绑定（生产部署）时 Cookie 必须带 Secure，否则会话明文传输。
        if self.cookie_secure == false
            && self.server_host != "127.0.0.1"
            && self.server_host != "localhost"
        {
            warnings.push(
                "COOKIE_SECURE 未开启且服务非本机绑定：生产 HTTPS 部署必须设置 COOKIE_SECURE=true".to_string(),
            );
        }
        warnings
    }
}
