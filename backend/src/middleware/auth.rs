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

use axum::body::Body;
use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use sqlx::MySqlPool;

use crate::error::AppError;
use crate::utils::jwt::{validate_token, Claims};

use async_trait::async_trait;

#[derive(Clone)]
pub struct AppState {
    pub pool: MySqlPool,
    pub config: crate::config::Config,
    /// 登录时对「用户名不存在」路径执行等开销假校验所用的固定 bcrypt 哈希，
    /// 由主程序启动时按 BCRYPT_COST 生成，用于消除用户枚举时序侧信道（F-01）。
    pub login_dummy_hash: String,
    /// 登录失败节流器（真实 IP + 用户名双维度，F-02）。
    pub login_throttle: crate::middleware::ratelimit::SharedLoginThrottle,
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    // F15: token 来源双通道——优先 Authorization: Bearer（API 客户端），
    // 回退到 HttpOnly Cookie `manner_token`（浏览器会话）。Cookie 由 login 下发、
    // logout 清除，前端不再将 JWT 落入 localStorage（消除 XSS 窃取面）。
    let mut via_cookie = false;
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|t| t.to_string())
        .or_else(|| {
            via_cookie = true;
            req.headers()
                .get("cookie")
                .and_then(|h| h.to_str().ok())
                .and_then(|cookie_header| {
                    cookie_header.split(';').find_map(|part| {
                        let part = part.trim();
                        part.strip_prefix("manner_token=").map(|v| v.to_string())
                    })
                })
        })
        .ok_or(AppError::Unauthorized)?;

    let claims = validate_token(&token, &state.config.jwt_secret)?;

    // 仅 access 令牌可通过鉴权中间件；refresh 令牌只允许 /auth/refresh 使用，
    // 防止 refresh 令牌被当作访问令牌调用业务接口（身份混淆防护）。
    if claims.typ == "refresh" {
        return Err(AppError::Unauthorized);
    }

    let is_blacklisted = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM token_blacklist WHERE jti = ?",
    )
    .bind(&claims.jti)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
        > 0;

    if is_blacklisted {
        return Err(AppError::TokenRevoked);
    }

    // F7: 双提交 CSRF 校验——仅对「Cookie 认证 + 写方法」强制要求
    // `X-CSRF-Token` 头与 `manner_csrf` Cookie 一致。
    // Bearer 认证（API 客户端，令牌本就在请求头中）不受 CSRF 约束。
    if via_cookie {
        let method = req.method().clone();
        if method == axum::http::Method::POST
            || method == axum::http::Method::PUT
            || method == axum::http::Method::DELETE
            || method == axum::http::Method::PATCH
        {
            let cookie_csrf = req
                .headers()
                .get("cookie")
                .and_then(|h| h.to_str().ok())
                .and_then(|cookie_header| {
                    cookie_header.split(';').find_map(|part| {
                        let part = part.trim();
                        part.strip_prefix("manner_csrf=").map(|v| v.to_string())
                    })
                })
                .unwrap_or_default();
            let header_csrf = req
                .headers()
                .get("x-csrf-token")
                .and_then(|h| h.to_str().ok())
                .map(|v| v.trim().to_string())
                .unwrap_or_default();
            if cookie_csrf.is_empty() || header_csrf.is_empty() || cookie_csrf != header_csrf {
                return Err(AppError::Forbidden);
            }
        }
    }

    // F-08: 校验 token 版本号与数据库一致。改密/重置密码后 pwd_version 递增，
    // 旧 token 即使未过期也会在此被拒绝，实现「改密即全端失效」。
    // F-13: 同时校验账号状态。status=0（禁用/离职）后，其所有已签发 token 立即失效。
    // 单设备登录：同一账号在别处重新登录会覆盖 employees.active_session，
    // 本会话 sid 与之一致才放行，否则视为「已在其他设备登录」被踢下线。
    let account: Option<(i64, i8, Option<String>, i64)> = sqlx::query_as(
        "SELECT pwd_version, status, active_session, perm_version FROM employees WHERE id = ?",
    )
    .bind(&claims.sub)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    let Some((stored_pwd_version, stored_status, active_session, stored_perm_version)) = account else {
        return Err(AppError::Unauthorized);
    };

    if stored_pwd_version != claims.pwd_version || stored_status != 1 {
        return Err(AppError::Unauthorized);
    }

    if let Some(active) = active_session {
        if !active.is_empty() && active != claims.sid {
            return Err(AppError::SessionExpired);
        }
    }

    // 权限即时生效：令牌内 perm_version 与库不一致（角色/权限/部门归属等变更后递增），
    // 或令牌内无授权快照（旧格式/无权限）时，从库重算有效授权并替换注入。
    let (permissions, grants) =
        if claims.perm_version == stored_perm_version && !claims.grants.is_empty() {
            (claims.permissions.clone(), claims.grants.clone())
        } else {
            let fresh = crate::services::permission::resolve_effective_grants(&state.pool, &claims.sub)
                .await
                .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
            let codes = crate::services::permission::permission_codes(&fresh);
            (codes, fresh)
        };

    let auth_user = AuthUser {
        id: claims.sub.clone(),
        username: claims.username.clone(),
        name: claims.name.clone(),
        permissions,
        grants,
        jti: claims.jti.clone(),
    };

    req.extensions_mut().insert(claims);
    req.extensions_mut().insert(auth_user);
    Ok(next.run(req).await)
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub grants: Vec<crate::services::permission::Grant>,
    #[allow(dead_code)]
    pub jti: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthUser>()
            .cloned()
            .ok_or(AppError::Unauthorized)
    }
}

pub struct JwtClaims(pub Claims);

#[async_trait]
impl<S> FromRequestParts<S> for JwtClaims {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<Claims>()
            .ok_or(AppError::Unauthorized)?;
        Ok(JwtClaims(claims.clone()))
    }
}

pub fn require_permission(permissions: &[String], required: &str) -> Result<(), AppError> {
    if permissions.iter().any(|p| p == required) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
