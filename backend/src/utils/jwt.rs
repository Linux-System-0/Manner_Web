use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    #[serde(default)]
    pub name: String,
    pub permissions: Vec<String>,
    pub exp: usize,
    pub jti: String,
    /// F-08: token 版本号。改密/重置密码后递增，服务端校验与 employees.pwd_version 不一致即拒绝，
    /// 使旧 token 在改密后立即失效（旧 token 无此字段时 serde default 为 0）。
    #[serde(default)]
    pub pwd_version: i64,
    /// 令牌用途："access"（访问令牌，中间件放行）/"refresh"（续期令牌，仅 /auth/refresh 可用）。
    /// 旧 token 无此字段时 serde default 为空字符串，中间件视为 access 放行（向后兼容）。
    #[serde(default)]
    pub typ: String,
}

pub fn create_token(
    employee_id: &str,
    username: &str,
    name: &str,
    permissions: &[String],
    pwd_version: i64,
    secret: &str,
    expire_minutes: i64,
) -> Result<String, AppError> {
    let now = Utc::now();
    let exp = (now + chrono::Duration::minutes(expire_minutes)).timestamp() as usize;
    let jti = Uuid::new_v4().to_string();

    let claims = Claims {
        sub: employee_id.to_string(),
        username: username.to_string(),
        name: name.to_string(),
        permissions: permissions.to_vec(),
        exp,
        jti,
        pwd_version,
        typ: "access".to_string(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT encode error: {}", e)))?;

    Ok(token)
}

/// 签发 refresh 续期令牌。
///
/// - 与 access token 同密钥、同 Claims 结构，但 typ="refresh" 且有效期按「天」计算；
/// - 携带 pwd_version：用户改密/重置密码后 pwd_version 递增，旧 refresh 令牌立即失效；
/// - 不携带 permissions（签发新 access 时由服务端重新查询，避免权限快照过期）。
pub fn create_refresh_token(
    employee_id: &str,
    username: &str,
    name: &str,
    pwd_version: i64,
    secret: &str,
    expire_days: i64,
) -> Result<String, AppError> {
    let now = Utc::now();
    let exp = (now + chrono::Duration::days(expire_days)).timestamp() as usize;
    let jti = Uuid::new_v4().to_string();

    let claims = Claims {
        sub: employee_id.to_string(),
        username: username.to_string(),
        name: name.to_string(),
        permissions: Vec::new(),
        exp,
        jti,
        pwd_version,
        typ: "refresh".to_string(),
    };

    let token = jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT encode error: {}", e)))?;

    Ok(token)
}

pub fn validate_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::Unauthorized)?;

    Ok(token_data.claims)
}
