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

use std::net::SocketAddr;
use axum::extract::{ConnectInfo, FromRequestParts, State};
use axum::http::request::Parts;
use axum::Json;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::AppError;
use crate::middleware::auth::{AuthUser, AppState, JwtClaims};
use crate::models::employee::{
    ChangePasswordRequest, Employee, FirstLoginRequest, LoginRequest, LoginResponse,
    LoginUserInfo, PrecheckRequest, PrecheckResponse, RegisterRequest,
};
use crate::services::auth::{hash_password, validate_password_strength, verify_password};
use crate::utils::jwt::{create_refresh_token, create_token, validate_token};
use crate::utils::response::ApiResponse;
use crate::utils::trusted_proxy;

/// 真实客户端 IP 提取器（FromRequestParts，不消费 body，可与其他 body 型 extractor 共存）。
/// 直连场景以 TCP 对端地址为准；经可信反向代理部署时（对端命中 TRUSTED_PROXIES
/// 白名单）信任 X-Real-IP / X-Forwarded-For，否则一律忽略转发头（防伪造绕过限流）。
#[derive(Debug, Clone)]
pub struct ClientIp(pub String);

#[async_trait::async_trait]
impl FromRequestParts<AppState> for ClientIp {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0);
        let ip =
            trusted_proxy::resolve_client_ip(peer, &parts.headers, &state.config.trusted_proxies);
        Ok(ClientIp(ip))
    }
}

/// 从请求 Cookie 头中提取指定名称的 cookie 值。
fn get_cookie_value(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|cookie_header| {
            cookie_header.split(';').find_map(|part| {
                let part = part.trim();
                part.strip_prefix(&format!("{name}=")).map(|v| v.to_string())
            })
        })
}

/// 构造 HttpOnly + SameSite=Strict 的会话 Cookie 字符串。
fn make_cookie(name: &str, value: &str, max_age_secs: i64, secure: bool) -> String {
    let secure_flag = if secure { "; Secure" } else { "" };
    format!("{name}={value}; HttpOnly; SameSite=Strict; Path=/; Max-Age={max_age_secs}{secure_flag}")
}

pub fn user_tag(name: &str, username: &str) -> String {
    if name.is_empty() {
        username.to_string()
    } else {
        format!("{} ({})", name, username)
    }
}

pub fn append_log(log_file: &str, msg: &str, ip: &str) {
    if let Ok(line) = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
    {
        let ts = chrono::DateTime::from_timestamp(line.as_secs() as i64, 0)
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "unknown".to_string());
        // 防日志注入：剥离消息中的控制字符（\n、\r、\0 等），
        // 防止用户可控内容（如聊天消息）伪造审计日志行、污染日志结构。
        let safe_msg: String = msg.chars().filter(|c| !c.is_control()).collect();
        // 无论何种业务日志，统一在末尾追加访问 IP，保证可审计。
        let entry = format!("[{}] {} | IP: {}\n", ts, safe_msg.trim(), ip);
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .map(|f| {
                use std::io::Write;
                let _ = writeln!(&f, "{}", entry.trim());
            });
    }
}

pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<ApiResponse<LoginUserInfo>>, AppError> {
    // F-07: 首个管理员注册开关 + 事务内 FOR UPDATE 锁，防止并发抢注出多个 admin。
    // 全新部署时 system_settings.registration_open 默认 '1'；注册成功后置 '0' 关闭。
    let mut tx = state.pool.begin().await?;

    let reg_open: Option<String> = sqlx::query_scalar(
        "SELECT setting_value FROM system_settings WHERE setting_key = 'registration_open' FOR UPDATE",
    )
    .fetch_optional(&mut *tx)
    .await?;

    if reg_open.as_deref() != Some("1") {
        return Err(AppError::Forbidden);
    }

    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM employees")
        .fetch_one(&mut *tx)
        .await?;

    if exists > 0 {
        return Err(AppError::Forbidden);
    }

    let username = body.username.trim().to_string();
    if username.is_empty() || username.chars().count() > 64 {
        return Err(AppError::ValidationError("用户名不能为空且不超过 64 个字符".to_string()));
    }

    let username_exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM employees WHERE username = ?",
    )
    .bind(&username)
    .fetch_one(&mut *tx)
    .await?;

    if username_exists > 0 {
        return Err(AppError::Conflict);
    }

    validate_password_strength(&body.password)?;

    let id = Uuid::new_v4().to_string();
    // 敏感字段静态加密后落库（首个管理员注册时填写的邮箱，密钥 FIELD_ENC_KEY）。
    let email = match body.email {
        Some(e) if !e.trim().is_empty() => {
            Some(crate::utils::crypto::encrypt_field(e.trim(), &state.config.field_enc_key)?)
        }
        _ => None,
    };
    let password = hash_password(&body.password, state.config.bcrypt_cost)?;

    sqlx::query(
        "INSERT INTO employees (id, username, password, name, email) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&username)
    .bind(&password)
    .bind(&body.name)
    .bind(&email)
    .execute(&mut *tx)
    .await?;

    // 首个管理员绑定 super_admin 内置角色（固定 id，拥有全部权限）。
    sqlx::query(
        "INSERT IGNORE INTO employee_roles (employee_id, role_id) VALUES (?, '00000000-0000-0000-0000-000000000001')",
    )
    .bind(&id)
    .execute(&mut *tx)
    .await?;

    // 关闭注册通道，首个管理员注册后系统不再接受注册。
    sqlx::query(
        "UPDATE system_settings SET setting_value = '0' WHERE setting_key = 'registration_open'",
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let grants = crate::services::permission::resolve_effective_grants(&state.pool, &id).await?;
    let permissions = crate::services::permission::permission_codes(&grants);

    let user_info = LoginUserInfo {
        id,
        username,
        name: body.name,
        permissions,
        avatar: None,
        must_change_password: false,
    };

    Ok(Json(ApiResponse::created(user_info)))
}

pub async fn login(
    State(state): State<AppState>,
    client_ip: ClientIp,
    Json(body): Json<LoginRequest>,
) -> Result<Response, AppError> {
    let ip = client_ip.0;

    // F-02: 登录失败节流（真实 IP + 用户名双维度）。已锁定 → 429 + Retry-After。
    {
        let mut throttle = state.login_throttle.lock().await;
        throttle.check(&ip, &body.username)?;
    }

    let employee = sqlx::query_as::<_, crate::models::employee::Employee>(
        "SELECT * FROM employees WHERE username = ?",
    )
    .bind(&body.username)
    .fetch_optional(&state.pool)
    .await?;

    let Some(employee) = employee else {
        // F-01: 对不存在的用户名执行一次等开销的 bcrypt 假校验，
        // 使「用户名不存在」与「密码错误」两条失败路径耗时一致，消除时序侧信道枚举。
        let _ = verify_password(&body.password, &state.login_dummy_hash);
        let mut throttle = state.login_throttle.lock().await;
        throttle.record_failure(&ip, &body.username);
        return Err(AppError::InvalidCredentials);
    };

    let valid = verify_password(&body.password, &employee.password);
    if !valid {
        let mut throttle = state.login_throttle.lock().await;
        throttle.record_failure(&ip, &body.username);
        return Err(AppError::InvalidCredentials);
    }

    // F-13: 校验账号状态。status=0（禁用/离职）拒绝登录，且统一返回与密码错误一致的
    // 错误消息（避免向攻击者泄露「账号存在但被禁用」的枚举信号）。
    if employee.status != 1 {
        append_log(
            &state.config.log_file,
            &format!("User {} ({}) login rejected: account disabled", employee.name, employee.username),
            &ip,
        );
        return Err(AppError::InvalidCredentials);
    }

    // 登录成功：清除该 IP 与该用户名的失败计数，避免历史失败累计误锁。
    {
        let mut throttle = state.login_throttle.lock().await;
        throttle.clear(&ip, &body.username);
    }

    append_log(
        &state.config.log_file,
        &format!("User {} ({}) logged in successfully", employee.name, employee.username),
        &ip,
    );

    issue_session(&state, &employee, None).await
}

/// 为已认证员工签发完整会话（access + refresh 令牌、双 Cookie、LoginResponse）。
/// login / first_login / refresh 共用，保证三种入口的会话结构一致。
///
/// `session_id`：login / first_login 传 None → 生成新会话并覆盖 employees.active_session
/// （单设备登录：旧设备令牌立即失效）；refresh 传当前会话 id → 复用同一会话，不覆盖。
async fn issue_session(
    state: &AppState,
    employee: &Employee,
    session_id: Option<&str>,
) -> Result<Response, AppError> {
    // F-08: 取数据库最新 pwd_version（first_login 改密后已递增，旧令牌全部失效）。
    // 同时取 perm_version：权限相关变更后递增，令牌内权限快照据此失效重算。
    let (pwd_version, perm_version): (i64, i64) = sqlx::query_as(
        "SELECT pwd_version, perm_version FROM employees WHERE id = ?",
    )
    .bind(&employee.id)
    .fetch_one(&state.pool)
    .await?;

    let grants =
        crate::services::permission::resolve_effective_grants(&state.pool, &employee.id).await?;
    let permissions = crate::services::permission::permission_codes(&grants);

    let session_id = match session_id {
        Some(sid) if !sid.is_empty() => sid.to_string(),
        _ => {
            let sid = Uuid::new_v4().to_string();
            sqlx::query("UPDATE employees SET active_session = ? WHERE id = ?")
                .bind(&sid)
                .bind(&employee.id)
                .execute(&state.pool)
                .await?;
            sid
        }
    };

    let token = create_token(
        &employee.id,
        &employee.username,
        &employee.name,
        &permissions,
        &grants,
        perm_version,
        pwd_version,
        &session_id,
        &state.config.jwt_secret,
        state.config.token_expire_minutes,
    )?;

    let refresh_token = create_refresh_token(
        &employee.id,
        &employee.username,
        &employee.name,
        perm_version,
        pwd_version,
        &session_id,
        &state.config.jwt_secret,
        state.config.refresh_token_expire_days,
    )?;

    let user_info = LoginUserInfo {
        id: employee.id.clone(),
        username: employee.username.clone(),
        name: employee.name.clone(),
        permissions,
        avatar: employee.avatar.clone(),
        must_change_password: employee.must_change_password == 1,
    };

    let max_age = state.config.token_expire_minutes * 60;
    let refresh_max_age = state.config.refresh_token_expire_days * 86400;

    let cookie_value = make_cookie("manner_token", &token, max_age, state.config.cookie_secure);
    let refresh_cookie_value = make_cookie(
        "manner_refresh",
        &refresh_token,
        refresh_max_age,
        state.config.cookie_secure,
    );
    // F7: 双提交 CSRF 令牌 Cookie（非 HttpOnly，前端 JS 读取后随写请求回传 X-CSRF-Token）。
    // SameSite=Strict + CSRF 校验双保险；Bearer 认证（API 客户端）不依赖 Cookie，不受影响。
    let csrf_value = Uuid::new_v4().to_string();
    let secure_flag = if state.config.cookie_secure { "; Secure" } else { "" };
    let csrf_cookie_value =
        format!("manner_csrf={csrf_value}; SameSite=Strict; Path=/; Max-Age={refresh_max_age}{secure_flag}");

    let response = LoginResponse {
        token,
        expires_in: max_age,
        user: user_info,
    };

    let mut resp = (StatusCode::OK, Json(ApiResponse::ok(response))).into_response();
    if let Ok(hv) = header::HeaderValue::from_str(&cookie_value) {
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
    if let Ok(hv) = header::HeaderValue::from_str(&refresh_cookie_value) {
        resp.headers_mut().append(header::SET_COOKIE, hv);
    }
    if let Ok(hv) = header::HeaderValue::from_str(&csrf_cookie_value) {
        resp.headers_mut().append(header::SET_COOKIE, hv);
    }
    Ok(resp)
}

pub async fn logout(
    State(state): State<AppState>,
    claims: JwtClaims,
    req: axum::extract::Request,
) -> Result<Response, AppError> {
    let exp = DateTime::from_timestamp(claims.0.exp as i64, 0)
        .map(|dt| dt.naive_utc())
        .unwrap_or_else(|| Utc::now().naive_utc());

    sqlx::query("INSERT INTO token_blacklist (id, jti, expires_at) VALUES (?, ?, ?)")
        .bind(Uuid::new_v4().to_string())
        .bind(&claims.0.jti)
        .bind(exp)
        .execute(&state.pool)
        .await?;

    // 一并拉黑 refresh 令牌：登出后刷新续期必须同样失败。
    if let Some(refresh_token) = get_cookie_value(req.headers(), "manner_refresh") {
        if let Ok(refresh_claims) = validate_token(&refresh_token, &state.config.jwt_secret) {
            let refresh_exp = DateTime::from_timestamp(refresh_claims.exp as i64, 0)
                .map(|dt| dt.naive_utc())
                .unwrap_or_else(|| Utc::now().naive_utc());
            let _ = sqlx::query("INSERT INTO token_blacklist (id, jti, expires_at) VALUES (?, ?, ?)")
                .bind(Uuid::new_v4().to_string())
                .bind(&refresh_claims.jti)
                .bind(refresh_exp)
                .execute(&state.pool)
                .await;
        }
    }

    let clear_cookie = "manner_token=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0";
    let clear_refresh_cookie = "manner_refresh=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0";
    let clear_csrf_cookie = "manner_csrf=; SameSite=Strict; Path=/; Max-Age=0";
    let mut resp = (StatusCode::OK, Json(ApiResponse::<()>::ok_msg("ok"))).into_response();
    if let Ok(hv) = header::HeaderValue::from_str(clear_cookie) {
        resp.headers_mut().insert(header::SET_COOKIE, hv);
    }
    if let Ok(hv) = header::HeaderValue::from_str(clear_refresh_cookie) {
        resp.headers_mut().append(header::SET_COOKIE, hv);
    }
    if let Ok(hv) = header::HeaderValue::from_str(clear_csrf_cookie) {
        resp.headers_mut().append(header::SET_COOKIE, hv);
    }
    Ok(resp)
}

pub async fn change_password(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let stored = sqlx::query_scalar::<_, String>(
        "SELECT password FROM employees WHERE id = ?",
    )
    .bind(&auth.id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let valid = verify_password(&body.old_password, &stored);
    if !valid {
        return Err(AppError::OldPasswordMismatch);
    }

    validate_password_strength(&body.new_password)?;

    let new_password = hash_password(&body.new_password, state.config.bcrypt_cost)?;

    // F-08: pwd_version 递增使所有旧 token 立即失效；F-02: 清除首登强制改密标记。
    sqlx::query(
        "UPDATE employees SET password = ?, pwd_version = pwd_version + 1, must_change_password = 0 WHERE id = ?",
    )
    .bind(&new_password)
    .bind(&auth.id)
    .execute(&state.pool)
    .await?;

    Ok(Json(ApiResponse::ok_msg("ok")))
}

pub async fn get_preferences(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let prefs: Option<String> = sqlx::query_scalar(
        "SELECT preferences FROM employees WHERE id = ?",
    )
    .bind(&auth.id)
    .fetch_one(&state.pool)
    .await?;

    let data = prefs
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::json!({}));

    Ok(Json(ApiResponse::ok(data)))
}

pub async fn update_preferences(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // F-11: preferences 白名单 schema 校验——只接受已知字段与合法取值，
    // 未知字段与非法值一律丢弃，防止任意 JSON 入库成为潜在 XSS 载体。
    let prefs = body.get("preferences").cloned().unwrap_or(serde_json::json!({}));
    let obj = match prefs {
        serde_json::Value::Object(m) => m,
        _ => return Err(AppError::BadRequest("preferences 必须是 JSON 对象".to_string())),
    };

    let mut clean = serde_json::Map::new();
    for (key, value) in obj {
        let ok = match (key.as_str(), &value) {
            ("theme", serde_json::Value::String(s)) => {
                s == "light" || s == "dark" || s == "system"
            }
            ("timezoneMode", serde_json::Value::String(s)) => {
                s == "system" || s == "manual"
            }
            ("timezoneOffset", serde_json::Value::Number(_)) => true,
            ("newConvPosition", serde_json::Value::String(s)) => {
                s == "first" || s == "last"
            }
            _ => false,
        };
        if ok {
            clean.insert(key, value);
        }
    }

    let prefs_str = serde_json::Value::Object(clean).to_string();

    sqlx::query("UPDATE employees SET preferences = ? WHERE id = ?")
        .bind(&prefs_str)
        .bind(&auth.id)
        .execute(&state.pool)
        .await?;

    Ok(Json(ApiResponse::ok_msg("ok")))
}

pub async fn me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let employee = sqlx::query_as::<_, crate::models::employee::Employee>(
        "SELECT * FROM employees WHERE id = ?",
    )
    .bind(&auth.id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "id": employee.id,
        "username": employee.username,
        "name": employee.name,
        "email": crate::utils::crypto::mask_field(employee.email),
        "title": employee.title,
        "phone": crate::utils::crypto::mask_field(employee.phone),
        "avatar": employee.avatar,
        "permissions": auth.permissions,
    }))))
}


/// 登录第一步：按用户名预检是否处于「首次登录待设置密码」状态。
///
/// 安全说明：
/// - 用户名不存在时返回 must_change=false 且执行一次与 BCRYPT_COST 同开销的假校验
///   （F-01 时序侧信道防护），避免「查库快慢差异」泄露用户名是否存在；
/// - 该接口只暴露两种状态（待激活 / 其他），不暴露「账号存在」这一信息；
/// - must_change 仅表示该用户名当前被标记为首次登录（创建员工/重置密码时置 1），
///   凭用户名无法完成任何操作——激活仍要求初始密码正确（first_login 校验）。
pub async fn precheck(
    State(state): State<AppState>,
    client_ip: ClientIp,
    Json(body): Json<PrecheckRequest>,
) -> Result<Json<ApiResponse<PrecheckResponse>>, AppError> {
    let ip = client_ip.0;

    // F-21: 节流（与登录一致：真实 IP + 用户名双维度），防止接口被用于批量账号枚举。
    {
        let mut throttle = state.login_throttle.lock().await;
        throttle.check(&ip, &body.username)?;
    }

    // F-21: 等开销假校验移到 SQL 之前，所有分支统一执行——
    // 消除「存在（直接查库返回） vs 不存在（假校验慢）」的时序差。
    let _ = verify_password(&body.username, &state.login_dummy_hash);

    let must_change: bool = sqlx::query_scalar(
        "SELECT must_change_password FROM employees WHERE username = ?",
    )
    .bind(&body.username)
    .fetch_optional(&state.pool)
    .await?
    .map(|v: i8| v == 1)
    .unwrap_or(false);

    Ok(Json(ApiResponse::ok(PrecheckResponse { must_change })))
}

/// 首次登录激活：验证初始密码 → 设置新密码 → 自动签发会话（即「设置登录密码后重新登录」）。
///
/// 仅允许 must_change_password=1 的账号走此流程；初始密码错误/非激活账号统一返回
/// InvalidCredentials，并复用登录节流（F-02，真实 IP + 用户名双维度）防暴力尝试。
pub async fn first_login(
    State(state): State<AppState>,
    client_ip: ClientIp,
    Json(body): Json<FirstLoginRequest>,
) -> Result<Response, AppError> {
    let ip = client_ip.0;

    // F-02: 节流（真实 IP + 用户名组合，防暴力/枚举），已锁定 → 429 + Retry-After。
    {
        let mut throttle = state.login_throttle.lock().await;
        throttle.check(&ip, &body.username)?;
    }

    let employee = sqlx::query_as::<_, Employee>(
        "SELECT * FROM employees WHERE username = ?",
    )
    .bind(&body.username)
    .fetch_optional(&state.pool)
    .await?;

    // F-3: 所有提前返回分支统一执行等开销 bcrypt 假校验（与下方真实校验一致），
    // 消除「待激活账号（慢）vs 非激活/不存在（快）」的时序侧信道枚举。
    let Some(mut employee) = employee else {
        let _ = verify_password(&body.initial_password, &state.login_dummy_hash);
        let mut throttle = state.login_throttle.lock().await;
        throttle.record_failure(&ip, &body.username);
        return Err(AppError::InvalidCredentials);
    };

    // 仅「首次登录待设置密码」账号可走此流程（统一错误，不泄露账号状态）。
    if employee.must_change_password != 1 {
        let _ = verify_password(&body.initial_password, &state.login_dummy_hash);
        let mut throttle = state.login_throttle.lock().await;
        throttle.record_failure(&ip, &body.username);
        return Err(AppError::InvalidCredentials);
    }

    // F-13: 禁用账号不允许激活（统一错误，不泄露禁用状态）。
    if employee.status != 1 {
        let _ = verify_password(&body.initial_password, &state.login_dummy_hash);
        let mut throttle = state.login_throttle.lock().await;
        throttle.record_failure(&ip, &body.username);
        return Err(AppError::InvalidCredentials);
    }

    // F-20: 改密前必须校验初始密码——否则任何知道用户名的人可接管待激活账号
    // （与 login 一致的失败路径：计数节流 + 统一 InvalidCredentials，不泄露账号状态）。
    let valid = verify_password(&body.initial_password, &employee.password);
    if !valid {
        let mut throttle = state.login_throttle.lock().await;
        throttle.record_failure(&ip, &body.username);
        return Err(AppError::InvalidCredentials);
    }

    // 新密码强度：与注册/修改密码/重置密码一致。
    validate_password_strength(&body.new_password)?;

    let new_password = hash_password(&body.new_password, state.config.bcrypt_cost)?;

    // F-08: 更新密码 + 清除首登标记 + pwd_version 递增（此前签发的任何令牌立即失效）。
    sqlx::query(
        "UPDATE employees SET password = ?, must_change_password = 0, pwd_version = pwd_version + 1 WHERE id = ?",
    )
    .bind(&new_password)
    .bind(&employee.id)
    .execute(&state.pool)
    .await?;

    employee.must_change_password = 0;

    // 激活成功：清除节流计数。
    {
        let mut throttle = state.login_throttle.lock().await;
        throttle.clear(&ip, &body.username);
    }

    append_log(
        &state.config.log_file,
        &format!("User {} ({}) set password on first login successfully", employee.name, employee.username),
        &ip,
    );

    issue_session(&state, &employee, None).await
}

/// 静默续期：用 refresh 令牌换取全新 access + refresh 会话（旋转式双 Cookie）。
///
/// - 校验 refresh 令牌签名、类型（必须 typ=refresh）、黑名单、用户状态与 pwd_version；
/// - 改密/重置密码后 pwd_version 递增 → 旧 refresh 令牌立即失效，续期自动失败
///   （配合前端 401 静默重放，失败后强制回到登录页）；
/// - 成功则返回与 login 相同的 LoginResponse，并覆盖两个 Cookie。
pub async fn refresh(
    State(state): State<AppState>,
    req: axum::extract::Request,
) -> Result<Response, AppError> {
    let Some(refresh_token) = get_cookie_value(req.headers(), "manner_refresh") else {
        return Err(AppError::Unauthorized);
    };

    let claims = validate_token(&refresh_token, &state.config.jwt_secret)?;

    // 仅接受 refresh 类型的令牌，防止 access 令牌被当作续期凭据。
    if claims.typ != "refresh" {
        return Err(AppError::Unauthorized);
    }

    let is_blacklisted: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM token_blacklist WHERE jti = ?",
    )
    .bind(&claims.jti)
    .fetch_one(&state.pool)
    .await?;

    if is_blacklisted > 0 {
        return Err(AppError::TokenRevoked);
    }

    // F-08 + F-13: 用户必须存在、启用且 pwd_version 与令牌一致。
    let account: Option<(i64, i8, Option<String>)> = sqlx::query_as(
        "SELECT pwd_version, status, active_session FROM employees WHERE id = ?",
    )
    .bind(&claims.sub)
    .fetch_optional(&state.pool)
    .await?;

    let Some((stored_pwd_version, stored_status, active_session)) = account else {
        return Err(AppError::Unauthorized);
    };

    if stored_pwd_version != claims.pwd_version || stored_status != 1 {
        return Err(AppError::Unauthorized);
    }

    // 单设备登录：若该账号已在别处重新登录（active_session 已更新），本会话续期被拒绝。
    if let Some(active) = active_session {
        if !active.is_empty() && active != claims.sid {
            return Err(AppError::SessionExpired);
        }
    }

    let employee = sqlx::query_as::<_, Employee>(
        "SELECT * FROM employees WHERE id = ?",
    )
    .bind(&claims.sub)
    .fetch_one(&state.pool)
    .await?;

    // F-22: refresh 令牌轮换。旧 jti 立即入黑名单，旧令牌不可再次续期；
    // 已被轮换的旧令牌再次使用会命中上方 is_blacklisted 检查，返回 TokenRevoked。
    let exp = DateTime::from_timestamp(claims.exp as i64, 0)
        .map(|dt| dt.naive_utc())
        .unwrap_or(Utc::now().naive_utc());
    sqlx::query("INSERT INTO token_blacklist (id, jti, expires_at) VALUES (?, ?, ?)")
        .bind(Uuid::new_v4().to_string())
        .bind(&claims.jti)
        .bind(exp)
        .execute(&state.pool)
        .await?;

    issue_session(&state, &employee, Some(&claims.sid)).await
}
