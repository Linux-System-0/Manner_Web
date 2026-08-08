pub mod auth;
pub mod chat;
pub mod employee;
pub mod system;

use axum::Router;
use axum::body::Body;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::Method;
use axum::http::StatusCode;
use axum::http::header::{HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{Request};
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{delete, get, post, put};
use std::collections::HashSet;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;

use crate::middleware::auth::AppState;
use crate::error::AppError;
use axum::middleware;

/// F-04/F-24/F-25: /uploads 静态目录访问控制。
/// - 静态服务仅限图片白名单扩展名（头像等 <img> 加载场景，需登录——见 build_router 中
///   叠加的认证中间件；未登录一律 401）；
/// - /uploads/chat/ 子目录（聊天文件，含聊天图片）一律 404：只能经 /api/chat/file
///   鉴权接口（会话成员校验）访问，杜绝 URL 直链泄露他人聊天/文件；
/// - 根目录非图片扩展名一律 404（聊天文件已隔离至 chat/ 子目录，根目录不应再出现非图片）；
/// - 无扩展名：一律 403（防路径混淆与探测）；
/// - F-25: 路径任一级组件为软链接 → 404（防 uploads/root -> / 之类符号链接逃逸，
///   读取服务器任意文件；tower-http ServeDir 0.5.2 无内置 symlink 禁用且 Rust
///   File::open/metadata 默认跟随符号链接，必须在此主动拦截）。
async fn uploads_extension_guard(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, axum::response::Response> {
    // F-23: 本守卫只作用于 /uploads 静态目录，其余路径交回正常路由匹配
    // （未注册 → 401 fallback，已注册 → 认证中间件 401），避免响应码差分泄露路由信息。
    if !req.uri().path().starts_with("/uploads") {
        return Ok(next.run(req).await);
    }
    let path = req.uri().path();
    // F-24: 聊天文件目录不提供静态访问
    if path.starts_with("/uploads/chat/") {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("not found"))
            .unwrap());
    }
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    if ext.is_empty() {
        return Ok(Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("forbidden"))
            .unwrap());
    }
    // F-25: 逐级检查目标路径组件，任何一级为软链接即 404
    let rel = path.trim_start_matches("/uploads").trim_start_matches('/');
    let mut current = std::path::PathBuf::from(&state.config.upload_dir);
    for comp in rel.split('/').filter(|s| !s.is_empty()) {
        current.push(comp);
        match tokio::fs::symlink_metadata(&current).await {
            Ok(m) if m.file_type().is_symlink() => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("not found"))
                    .unwrap());
            }
            Ok(_) => {}
            Err(_) => {
                // 路径不存在：直接 404（与 ServeDir 行为一致）
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("not found"))
                    .unwrap());
            }
        }
    }
    // F-25b: 引用校验——静态文件必须被员工头像(employees.avatar)或消息
    // (messages.file_url) 引用，否则 404。硬链接在文件系统层面与普通文件
    // 无法区分（symlink_metadata 不会标记硬链接），此校验从业务侧杜绝
    // "未被任何记录引用的文件"被静态读取（含硬链接逃逸指向的系统文件）。
    let rel_url = format!("/uploads/{}", rel);
    let avatar_refs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM employees WHERE avatar = ?")
        .bind(&rel_url)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    let msg_refs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE file_url = ?")
        .bind(&rel_url)
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    if avatar_refs + msg_refs == 0 {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("not found"))
            .unwrap());
    }
    // F-04: 非图片扩展名强制 Content-Disposition: attachment（下载而非内联渲染）。
    // 即使上传了 html/svg 等可携带脚本的内容，浏览器也不会执行
    // （配合全局 X-Content-Type-Options: nosniff 双重兜底）。图片保持内联（头像等）。
    let mut resp = next.run(req).await;
    if !system::ALLOWED_UPLOAD_EXTS.contains(&ext.as_str()) {
        resp.headers_mut().insert(
            CONTENT_DISPOSITION,
            HeaderValue::from_static("attachment"),
        );
    }
    Ok(resp)
}

pub fn build_router(state: AppState) -> Router {
    // F-07: register 移入匿名路由——全新部署时首个管理员通过注册创建（registration_open 开关防并发）。
    let public_routes = Router::new()
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/precheck", post(auth::precheck))
        .route("/api/auth/first-login", post(auth::first_login))
        .route("/api/auth/refresh", post(auth::refresh))
        .route("/api/system/login-page", get(system::get_login_page_settings));

    let protected_routes = Router::new()
        .route("/api/system/health", get(system::health))
        .route("/api/system/settings", get(system::get_settings))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/password", put(auth::change_password))
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/preferences", get(auth::get_preferences))
        .route("/api/auth/preferences", put(auth::update_preferences))
        .route("/api/employees", get(employee::list_employees))
        .route("/api/employees", post(employee::create_employee))
        .route("/api/employees/:id", get(employee::get_employee))
        .route("/api/employees/:id", put(employee::update_employee))
        .route("/api/employees/:id", delete(employee::delete_employee))
        .route("/api/employees/:id/password", put(employee::reset_password))
        .route("/api/employees/:id/permissions", put(employee::update_employee_permissions))
        .route("/api/permissions", get(system::list_permissions))
        .route("/api/upload", post(system::upload).layer(DefaultBodyLimit::max(104_857_600)))
        .route("/api/upload/file", post(system::upload_file).layer(DefaultBodyLimit::max(104_857_600)))
        .route("/api/system/logs", get(system::logs))
        .route("/api/system/settings", put(system::update_settings))
        .route("/api/chat/conversations", get(chat::list_conversations))
        .route("/api/chat/conversations/:id/messages", get(chat::get_messages))
        .route("/api/chat/conversations/:id/messages", post(chat::send_message))
        .route("/api/chat/conversations/:id/name", put(chat::update_group_name))
        .route("/api/chat/conversations/:id/participants", post(chat::add_participant))
        .route("/api/chat/conversations/:id/participants/:target_id", put(chat::update_participant))
        .route("/api/chat/conversations/:id/participants/:target_id", delete(chat::remove_participant))
        .route("/api/chat/conversations/:id/disband", delete(chat::disband_group))
        .route("/api/chat/block", post(chat::block_user))
        .route("/api/chat/block/:id", delete(chat::unblock_user))
        .route("/api/chat/blocked", get(chat::list_blocked))
        .route("/api/chat/employees", get(chat::list_employees_for_chat))
        .route("/api/chat/file/:name", get(chat::get_chat_file))
        .route("/api/employees/:id/protect-block", put(chat::update_protect_block))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::auth::auth_middleware,
        ));

    let cors_origins: HashSet<HeaderValue> = state
        .config
        .cors_allowed_origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(cors_origins))
        .allow_credentials(true)
        .allow_methods(AllowMethods::list([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ]))
        .allow_headers(AllowHeaders::list([CONTENT_TYPE, ACCEPT, AUTHORIZATION]));

    let upload_dir = state.config.upload_dir.clone();

    let uploads_router = Router::new()
        .nest_service("/uploads", ServeDir::new(&upload_dir))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            uploads_extension_guard,
        ))
        // F-24: /uploads 静态目录一律要求登录（Authorization Bearer 或 HttpOnly Cookie）。
        // 聊天附件等敏感文件不再匿名可下载；头像等 <img> 同源加载自动携带 Cookie 不受影响。
        .layer(middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::auth::auth_middleware,
        ));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(uploads_router)
        // F-23: 未注册路径统一回落 401，与「匿名访问一律 401」基线一致，
        // 消除 401（路由已注册）vs 403/404（未注册）的响应码差分信号。
        .fallback(|| async { AppError::Unauthorized })
        .layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn(
            crate::middleware::security::request_logging_middleware,
        ))
        .layer(axum::middleware::from_fn(
            crate::middleware::security::harden_response_middleware,
        ))
        .with_state(state)
}
