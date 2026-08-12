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
use axum::extract::{Multipart, Path, Query, State};
use axum::http::header;
use axum::response::Response;
use axum::Json;
use serde::Deserialize;
use std::time::Duration;
use uuid::Uuid;

use crate::error::AppError;
use crate::handlers::auth::{append_log, user_tag, ClientIp};
use crate::middleware::auth::{require_permission, AppState, AuthUser};
use crate::models::employee::{Permission, PermissionInfo, PermissionModule};
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct SettingsBody {
    pub chat_upload_limit: Option<String>,
    pub login_theme: Option<String>,
    pub site_title: Option<String>,
    pub login_site_title: Option<String>,
    pub login_site_icon: Option<String>,
    pub site_icon: Option<String>,
    pub login_max_failures: Option<String>,
    pub login_lock_window_secs: Option<String>,
    /// 默认语言包：system（跟随系统/浏览器） | en-US | zh-CN
    pub default_language: Option<String>,
}

/// 允许内联渲染的图片扩展名白名单（头像、聊天图片预览）。
/// 刻意排除 svg/html/xml 等可携带脚本的类型，防止经 /uploads 静态路径触发存储型 XSS（F-10）。
/// 注意：非此列表的扩展名仍可通过 /api/upload/file 上传（任意文件），
/// 但 /uploads 静态访问一律强制 Content-Disposition: attachment（下载而非内联渲染）。
/// chat.rs 校验 file_url 时复用（F-10）。
pub const ALLOWED_UPLOAD_EXTS: [&str; 7] = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico"];

async fn save_setting(
    pool: &sqlx::MySqlPool,
    current: &std::collections::HashMap<String, String>,
    key: &str,
    value: Option<String>,
    changes: &mut Vec<String>,
    label: &str,
) -> Result<(), AppError> {
    if let Some(value) = value {
        if current.get(key).map_or(true, |v| v != &value) {
            sqlx::query(
                "INSERT INTO system_settings (setting_key, setting_value) VALUES (?, ?) \
                 ON DUPLICATE KEY UPDATE setting_value = ?",
            )
            .bind(key)
            .bind(&value)
            .bind(&value)
            .execute(pool)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to save settings: {}", e)))?;
            changes.push(format!("{}: {}", label, value));
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub lines: Option<usize>,
}

pub(crate) fn is_allowed_extension(ext: &str) -> bool {
    ALLOWED_UPLOAD_EXTS.iter().any(|e| *e == ext)
}

/// 通用上传扩展名校验（/api/upload/file 任意文件接口）：
/// 允许任意合法的 ASCII 字母数字扩展名；拒绝无扩展名、超长或含特殊字符（防路径混淆）。
/// 安全由静态目录守卫兜底：非图片扩展名一律强制下载，浏览器不会执行其内容。
pub fn is_uploadable_extension(ext: &str) -> bool {
    !ext.is_empty() && ext.len() <= 16 && ext.chars().all(|c| c.is_ascii_alphanumeric())
}

/// 轻量魔数校验，防止伪造扩展名（例如把 HTML 改名为 .png 上传）。
fn has_valid_magic(data: &[u8], ext: &str) -> bool {
    match ext {
        "png" => data.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        "jpg" | "jpeg" => data.starts_with(&[0xFF, 0xD8, 0xFF]),
        "gif" => data.starts_with(b"GIF8"),
        "webp" => data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WEBP",
        "bmp" => data.starts_with(b"BM"),
        "ico" => data.starts_with(&[0x00, 0x00, 0x01, 0x00]),
        _ => false,
    }
}

pub async fn health(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "system:settings")?;

    let db_ok = tokio::time::timeout(Duration::from_secs(3), async {
        sqlx::query("SELECT 1")
            .execute(&state.pool)
            .await
            .is_ok()
    })
    .await
    .unwrap_or(false);

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "server": "running",
        "database": if db_ok { "connected" } else { "disconnected" },
        "version": env!("CARGO_PKG_VERSION"),
    }))))
}

fn parse_size_limit(value: &str) -> Option<usize> {
    if value == "禁止" {
        return Some(0);
    }
    if value == "无限制" {
        return None;
    }
    // F-24: 长度 <2 时 `value.len() - 2` 在 debug 下 usize 下溢、release 下 split_at(mid>len) panic，
    // 统一在此短路返回 None（非法格式），避免配置驱动的请求级崩溃。
    if value.len() < 2 {
        return None;
    }
    let (_num_str, unit) = value.split_at(value.len() - 2);
    let unit = if unit == "KB" || unit == "MB" || unit == "GB" || unit == "TB" {
        unit
    } else if value.len() >= 1 {
        let last = &value[value.len() - 1..];
        if last == "B" {
            last
        } else {
            return None;
        }
    } else {
        return None;
    };
    let num_str = &value[..value.len() - unit.len()];
    let num: usize = num_str.parse().ok()?;
    let multiplier = match unit {
        "B" => 1,
        "KB" => 1024,
        "MB" => 1024 * 1024,
        "GB" => 1024 * 1024 * 1024,
        "TB" => 1024usize.pow(4),
        _ => return None,
    };
    Some(num * multiplier)
}

/// 通用上传保存逻辑。
/// - image_only=true：仅允许 ALLOWED_UPLOAD_EXTS 图片（头像/内联预览用），图片一律做魔数校验；
/// - image_only=false：允许任意合法扩展名（聊天文件），其中图片类仍做魔数校验，其余类型不做
///   （其安全由静态目录守卫强制下载保证）。
async fn save_upload(
    state: &AppState,
    auth: &AuthUser,
    multipart: &mut Multipart,
    image_only: bool,
    ip: &str,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let limit: Option<String> = sqlx::query_scalar(
        "SELECT setting_value FROM system_settings WHERE setting_key = 'chat_upload_limit'",
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to read settings: {}", e)))?;

    let max_bytes = limit.as_deref().and_then(parse_size_limit);

    if max_bytes == Some(0) {
        append_log(&state.config.log_file, &format!("User {} upload failed: uploads disabled by administrator", user_tag(&auth.name, &auth.username)), ip);
        return Err(AppError::BadRequest("文件上传已被管理员禁止".to_string()));
    }

    let upload_dir = std::path::Path::new(&state.config.upload_dir);
    tokio::fs::create_dir_all(upload_dir).await.map_err(|e| {
        AppError::Internal(anyhow::anyhow!("Failed to create upload dir: {}", e))
    })?;

    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "file".to_string());

        let ext_raw = std::path::Path::new(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();
        let ext = ext_raw.to_ascii_lowercase();

        let ext_ok = if image_only {
            is_allowed_extension(&ext)
        } else {
            is_uploadable_extension(&ext)
        };
        if !ext_ok {
            append_log(&state.config.log_file, &format!("User {} upload of {} failed: unsupported file type ({})", user_tag(&auth.name, &auth.username), file_name, ext), ip);
            return Err(AppError::BadRequest(format!(
                "不支持的文件类型: {}", ext
            )));
        }

        let new_name = format!("{}.{}", Uuid::new_v4(), ext);
        // F-24: 聊天文件（非头像等图片）统一存入 chat/ 子目录，静态 /uploads 不服务该目录，
        // 只能经 /api/chat/file 鉴权接口（会话成员校验）访问，杜绝 URL 直链泄露他人文件。
        let sub_dir = if image_only { "" } else { "chat" };
        let path = upload_dir.join(sub_dir).join(&new_name);
        if !sub_dir.is_empty() {
            tokio::fs::create_dir_all(upload_dir.join(sub_dir))
                .await
                .map_err(|e| {
                    AppError::Internal(anyhow::anyhow!("Failed to create upload sub dir: {}", e))
                })?;
        }

        let data = field.bytes().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Failed to read upload data: {}", e))
        })?;

        // 图片类一律校验魔数（防伪装扩展名，例如把 HTML 改名 .png）；
        // 非图片类型不校验（其安全由静态目录守卫强制下载兜底）。
        if is_allowed_extension(&ext) && !has_valid_magic(&data, &ext) {
            append_log(&state.config.log_file, &format!("User {} upload of {} failed: file content does not match declared type", user_tag(&auth.name, &auth.username), file_name), ip);
            return Err(AppError::BadRequest(
                "文件内容与声明类型不符".to_string(),
            ));
        }

        if let Some(max) = max_bytes {
            if data.len() > max {
                let limit_display = limit.clone().unwrap_or_else(|| max.to_string());
                append_log(&state.config.log_file, &format!("User {} upload of {} failed: file size exceeds limit ({})", user_tag(&auth.name, &auth.username), file_name, limit_display), ip);
                return Err(AppError::BadRequest(format!(
                    "文件大小超过限制 ({})", limit_display
                )));
            }
        }

        tokio::fs::write(&path, &data).await.map_err(|e| {
            append_log(&state.config.log_file, &format!("User {} upload of {} failed: write error - {}", user_tag(&auth.name, &auth.username), file_name, e), ip);
            AppError::Internal(anyhow::anyhow!("Failed to write upload file: {}", e))
        })?;

        append_log(&state.config.log_file, &format!("User {} uploaded file successfully: {} ({} bytes, {})", user_tag(&auth.name, &auth.username), file_name, data.len(), new_name), ip);
        let url = if image_only {
            format!("/uploads/{}", new_name)
        } else {
            format!("/uploads/chat/{}", new_name)
        };
        return Ok(Json(ApiResponse::ok(url)));
    }

    Err(AppError::Internal(anyhow::anyhow!("No file uploaded")))
}

/// 图片上传（头像等）：仅允许图片白名单 + 魔数校验。
pub async fn upload(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let ip = client_ip.0;
    save_upload(&state, &auth, &mut multipart, true, &ip).await
}

/// 任意文件上传（聊天文件）：允许任意合法扩展名，图片类仍做魔数校验。
pub async fn upload_file(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let ip = client_ip.0;
    // F-17: 聊天文件上传需要 chat:upload 权限码（admin 角色默认拥有，
    // 普通角色需管理员在角色管理中授予），避免任意认证用户滥用上传面。
    require_permission(&auth.permissions, "chat:upload")?;
    save_upload(&state, &auth, &mut multipart, false, &ip).await
}

pub async fn update_settings(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Json(body): Json<SettingsBody>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let ip = client_ip.0;
    require_permission(&auth.permissions, "system:settings")?;

    let current: std::collections::HashMap<String, String> = sqlx::query_as::<_, (String, String)>(
        "SELECT setting_key, setting_value FROM system_settings",
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .collect();

    let mut changes: Vec<String> = Vec::new();

    // F-24: chat_upload_limit 入库前格式校验（"数字+单位" | "无限制" | "禁止"），
    // 防止非法配置值（长度 <2、负数、纯字母等）落入 DB 触发 parse_size_limit panic 或语义异常。
    if let Some(ref v) = body.chat_upload_limit {
        let valid = v == "无限制" || v == "禁止" || parse_size_limit(v).is_some();
        if !valid {
            return Err(AppError::ValidationError(
                "上传限制格式不正确（如 10MB / 无限制 / 禁止）".to_string(),
            ));
        }
    }

    save_setting(&state.pool, &current, "chat_upload_limit", body.chat_upload_limit.clone(), &mut changes, "upload limit").await?;
    save_setting(&state.pool, &current, "login_theme", body.login_theme.clone(), &mut changes, "login theme").await?;
    save_setting(&state.pool, &current, "site_title", body.site_title.clone(), &mut changes, "post-login site title").await?;
    save_setting(&state.pool, &current, "login_site_title", body.login_site_title.clone(), &mut changes, "login page site title").await?;
    save_setting(&state.pool, &current, "login_site_icon", body.login_site_icon.clone(), &mut changes, "login page site icon").await?;
    save_setting(&state.pool, &current, "site_icon", body.site_icon.clone(), &mut changes, "post-login site icon").await?;

    // 登录限流参数:数值校验(1~100 次 / 1~86400 秒),合法才入库。
    if let Some(ref v) = body.login_max_failures {
        let n: usize = v
            .parse()
            .map_err(|_| AppError::ValidationError("登录失败次数上限必须是正整数".to_string()))?;
        if !(1..=100).contains(&n) {
            return Err(AppError::ValidationError(
                "登录失败次数上限需在 1~100 之间".to_string(),
            ));
        }
    }
    if let Some(ref v) = body.login_lock_window_secs {
        let n: u64 = v
            .parse()
            .map_err(|_| AppError::ValidationError("锁定窗口必须是正整数(秒)".to_string()))?;
        if !(1..=86400).contains(&n) {
            return Err(AppError::ValidationError(
                "锁定窗口需在 1~86400 秒之间".to_string(),
            ));
        }
    }
    save_setting(&state.pool, &current, "login_max_failures", body.login_max_failures.clone(), &mut changes, "max login failures").await?;
    save_setting(&state.pool, &current, "login_lock_window_secs", body.login_lock_window_secs.clone(), &mut changes, "login lock window (secs)").await?;

    // 默认语言包：system（跟随系统）或合法 BCP-47 语言代码（如 en-US / zh-CN / ja-JP），
    // 仅校验格式防止任意字符串入库；前端语言包目录（locales/*.json）决定可用列表，
    // 后端不做语言白名单限制（添加语言包无需改后端）。
    if let Some(ref v) = body.default_language {
        let valid = v == "system"
            || (v.len() >= 2
                && v.len() <= 35
                && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        if !valid {
            return Err(AppError::ValidationError(
                "默认语言包仅支持 system 或合法语言代码（如 en-US / zh-CN）".to_string(),
            ));
        }
    }
    save_setting(&state.pool, &current, "default_language", body.default_language.clone(), &mut changes, "default language").await?;

    // 限流参数变更后立即同步到内存节流器(无需重启生效)。
    if body.login_max_failures.is_some() || body.login_lock_window_secs.is_some() {
        let max: usize = sqlx::query_scalar::<_, String>(
            "SELECT setting_value FROM system_settings WHERE setting_key = 'login_max_failures'",
        )
        .fetch_one(&state.pool)
        .await?
        .parse()
        .unwrap_or(5);
        let win: u64 = sqlx::query_scalar::<_, String>(
            "SELECT setting_value FROM system_settings WHERE setting_key = 'login_lock_window_secs'",
        )
        .fetch_one(&state.pool)
        .await?
        .parse()
        .unwrap_or(900);
        state.login_throttle.lock().await.update_limits(max, win);
    }

    if !changes.is_empty() {
        append_log(&state.config.log_file, &format!("User {} modified system settings: {}", user_tag(&auth.name, &auth.username), changes.join(", ")), &ip);
    }
    Ok(Json(ApiResponse::ok_msg("保存成功")))
}

pub async fn get_settings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "system:settings")?;

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT setting_key, setting_value FROM system_settings",
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let mut map = serde_json::Map::new();
    for (key, value) in rows {
        map.insert(key, serde_json::Value::String(value));
    }
    Ok(Json(ApiResponse::ok(serde_json::Value::Object(map))))
}

/// 登录页使用的公开只读配置（无需身份认证）：
/// 仅暴露登录页标题与主题，不包含其他敏感系统设置。
pub async fn get_login_page_settings(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT setting_key, setting_value FROM system_settings \
         WHERE setting_key IN ('login_site_title','login_theme','site_title',\
         'login_site_icon','site_icon','default_language')",
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let mut map = serde_json::Map::new();
    for (key, value) in rows {
        map.insert(key, serde_json::Value::String(value));
    }

    // 注册入口仅在「注册开关开启」且「系统尚无任何账号」时开放，
    // 与 /api/auth/register 的判定逻辑保持一致，避免前端误显入口。
    let reg_open = sqlx::query_scalar::<_, String>(
        "SELECT setting_value FROM system_settings WHERE setting_key = 'registration_open'",
    )
    .fetch_optional(&state.pool)
    .await
    .unwrap_or_default();
    let employee_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM employees")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(1);
    map.insert(
        "registration_open".to_string(),
        serde_json::Value::Bool(reg_open.as_deref() == Some("1") && employee_count == 0),
    );

    Ok(Json(ApiResponse::ok(serde_json::Value::Object(map))))
}

/// 公开图标服务：按 key 返回登录页（login）/ 登录后（site）的网站图标文件字节。
/// 未配置时返回 404。安全要点：
/// - 仅允许从 system_settings 中读取配置的路径（不可任意指定文件）；
/// - 仅允许扩展名白名单（与静态目录守卫一致），且校验魔数防伪造；
/// - 仅限 upload_dir 根目录文件，禁止子目录/路径穿越。
pub async fn get_site_icon(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Response, AppError> {
    let setting_key = match key.as_str() {
        "login" => "login_site_icon",
        "site" => "site_icon",
        _ => return Err(AppError::NotFound),
    };

    let url: Option<String> = sqlx::query_scalar(
        "SELECT setting_value FROM system_settings WHERE setting_key = ?",
    )
    .bind(setting_key)
    .fetch_optional(&state.pool)
    .await?;

    let Some(url) = url else {
        return Err(AppError::NotFound);
    };

    // 仅接受 /uploads/<file> 形式（上传接口产物），防止任意文件读取。
    let Some(rel) = url.strip_prefix("/uploads/") else {
        return Err(AppError::NotFound);
    };
    if rel.is_empty() || rel.contains('/') || rel.contains('\\') || rel.contains("..") {
        return Err(AppError::NotFound);
    }

    let ext = std::path::Path::new(rel)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    if !is_allowed_extension(&ext) {
        return Err(AppError::NotFound);
    }

    let path = std::path::Path::new(&state.config.upload_dir).join(rel);
    // F-25: 软链接逃逸防护——目标为符号链接时拒绝读取。
    if let Ok(m) = tokio::fs::symlink_metadata(&path).await {
        if m.file_type().is_symlink() {
            return Err(AppError::NotFound);
        }
    }

    let data = tokio::fs::read(&path).await.map_err(|_| AppError::NotFound)?;

    // 魔数校验：防止把非图片内容伪装成图标（与上传侧一致）。
    if !has_valid_magic(&data, &ext) {
        return Err(AppError::NotFound);
    }

    let content_type = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    };

    Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "public, max-age=300")
        .body(Body::from(data))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to build icon response: {}", e)))
}

pub async fn logs(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<LogQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "system:settings")?;

    let log_path = std::path::Path::new(&state.config.log_file);
    let max_lines = query.lines.unwrap_or(200);

    if !log_path.exists() {
        return Ok(Json(ApiResponse::ok(serde_json::json!({
            "lines": [],
            "total": 0,
            "file": "",
        }))));
    }

    let content = tokio::fs::read_to_string(log_path).await.map_err(|e| {
        AppError::Internal(anyhow::anyhow!("Failed to read log file: {}", e))
    })?;

    let all_lines: Vec<&str> = content.lines().collect();
    let total = all_lines.len();
    let start = if total > max_lines { total - max_lines } else { 0 };
    let lines: Vec<&str> = all_lines[start..].to_vec();

    // F-12: 仅暴露日志文件名，不泄露服务器绝对路径。
    let file_display = log_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "lines": lines,
        "total": total,
        "file": file_display,
    }))))
}

/// 权限字典列表（按模块分组）。供角色管理界面（role:manage）勾选权限使用。
pub async fn list_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "role:manage")?;

    let permissions: Vec<Permission> = sqlx::query_as(
        "SELECT id, code, name, module FROM permissions ORDER BY module, id",
    )
    .fetch_all(&state.pool)
    .await?;

    let mut modules: Vec<PermissionModule> = Vec::new();
    let mut current_module: Option<String> = None;
    let mut current_permissions: Vec<PermissionInfo> = Vec::new();
    let mut module_name = String::new();

    for perm in permissions {
        if current_module.as_ref() != Some(&perm.module) {
            if let Some(module) = current_module.take() {
                modules.push(PermissionModule {
                    module,
                    module_name: module_name.clone(),
                    permissions: std::mem::take(&mut current_permissions),
                });
            }
            current_module = Some(perm.module.clone());
            module_name = match perm.module.as_str() {
                "employee" => "员工管理".to_string(),
                "department" => "部门管理".to_string(),
                "system" => "系统设置".to_string(),
                "chat" => "聊天".to_string(),
                "role" => "角色管理".to_string(),
                _ => perm.module.clone(),
            };
        }
        current_permissions.push(PermissionInfo {
            code: perm.code,
            name: perm.name,
        });
    }

    if let Some(module) = current_module {
        modules.push(PermissionModule {
            module,
            module_name,
            permissions: current_permissions,
        });
    }

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "modules": modules,
    }))))
}
