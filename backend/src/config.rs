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
    /// 员工敏感字段静态加密密钥（SHA-256 派生为 32 字节 AES-256 密钥）。
    /// 缺失/过短时拒绝启动或告警，见 security_warnings()。
    pub field_enc_key: [u8; 32],
    /// 可信反向代理 IP 白名单（单 IP 或 CIDR，逗号分隔）。
    /// 仅当对端 IP 命中白名单时后端才信任 X-Real-IP / X-Forwarded-For，
    /// 用于登录限流与审计日志解析真实客户端 IP；直连部署留空即可。
    pub trusted_proxies: crate::utils::trusted_proxy::TrustedProxies,
}

pub const DEFAULT_JWT_SECRET: &str = "your-super-secret-key-change-in-production";

/// .env.example 中的示例值。直接照搬示例配置会被拒绝启动，防止误用公开的示例凭据。
pub const EXAMPLE_DATABASE_URL: &str = "mysql://manner:Change_Me_123@127.0.0.1:3306/manner_web";
pub const EXAMPLE_JWT_SECRET: &str =
    "CHANGE_ME_this_is_an_example_jwt_secret_please_generate_a_64_char_random_one";
pub const EXAMPLE_FIELD_ENC_KEY: &str = "CHANGE_ME_example_field_enc_key_please_use_random";

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: {
                let url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
                if url == EXAMPLE_DATABASE_URL {
                    panic!("DATABASE_URL 仍为 .env.example 示例值，拒绝启动：请配置真实数据库连接串");
                }
                url
            },
            jwt_secret: {
                let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
                if secret == DEFAULT_JWT_SECRET || secret == EXAMPLE_JWT_SECRET {
                    panic!("JWT_SECRET 仍为公开默认值/示例值，拒绝启动：请设置强随机密钥（建议 >=32 字符）");
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
            field_enc_key: {
                let key = env::var("FIELD_ENC_KEY")
                    .expect("FIELD_ENC_KEY must be set（员工敏感字段加密密钥，建议 ≥32 字符强随机值）");
                if key == EXAMPLE_FIELD_ENC_KEY {
                    panic!("FIELD_ENC_KEY 仍为 .env.example 示例值，拒绝启动：请设置强随机密钥");
                }
                crate::utils::crypto::derive_key(&key)
            },
            trusted_proxies: crate::utils::trusted_proxy::TrustedProxies::parse(
                &env::var("TRUSTED_PROXIES").unwrap_or_default(),
            ),
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
        if self.field_enc_key == [0u8; 32] {
            warnings.push("FIELD_ENC_KEY 派生为空密钥，请设置强随机值（建议 ≥32 字符）".to_string());
        }
        warnings
    }
}
