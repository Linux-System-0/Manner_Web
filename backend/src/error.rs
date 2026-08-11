use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::utils::response::ApiResponse;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("用户名或密码错误")]
    InvalidCredentials,

    #[error("Token 无效或已过期")]
    Unauthorized,

    #[error("Token 已被注销")]
    TokenRevoked,

    #[error("该用户已在其他设备登录")]
    SessionExpired,

    #[error("无权限访问")]
    Forbidden,

    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    ValidationError(String),

    #[error("资源不存在")]
    NotFound,

    #[error("用户名已存在")]
    Conflict,

    #[error("旧密码错误")]
    OldPasswordMismatch,

    #[error("请求过于频繁，请稍后再试")]
    TooManyRequests { retry_after_secs: u64 },

    #[error("服务器内部错误")]
    Internal(#[from] anyhow::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!(error = ?err, "Database error");
        AppError::Internal(anyhow::anyhow!(err))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, 40001),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, 40002),
            AppError::TokenRevoked => (StatusCode::UNAUTHORIZED, 40003),
            AppError::SessionExpired => (StatusCode::UNAUTHORIZED, 40010),
            AppError::Forbidden => (StatusCode::FORBIDDEN, 40004),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, 40000),
            AppError::ValidationError(_) => (StatusCode::BAD_REQUEST, 40005),
            AppError::NotFound => (StatusCode::NOT_FOUND, 40006),
            AppError::Conflict => (StatusCode::CONFLICT, 40007),
            AppError::OldPasswordMismatch => (StatusCode::BAD_REQUEST, 40008),
            AppError::TooManyRequests { .. } => (StatusCode::TOO_MANY_REQUESTS, 40009),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, 50000),
        };
        let message = match &self {
            AppError::TooManyRequests { retry_after_secs } => {
                format!("登录失败次数过多，请在 {} 秒后重试", retry_after_secs)
            }
            _ => self.to_string(),
        };
        let mut response =
            (status, Json(ApiResponse::<()>::error(code, &message))).into_response();
        // 429 响应携带 Retry-After 头，告知客户端解锁时间。
        if let AppError::TooManyRequests { retry_after_secs } = &self {
            if let Ok(v) = axum::http::HeaderValue::from_str(&retry_after_secs.to_string()) {
                response
                    .headers_mut()
                    .insert(axum::http::header::RETRY_AFTER, v);
            }
        }
        response
    }
}
