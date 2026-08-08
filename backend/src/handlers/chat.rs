use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::Response;
use axum::Json;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::handlers::auth::{append_log, user_tag};
use crate::middleware::auth::{require_permission, AppState, AuthUser};
use crate::utils::response::ApiResponse;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ConversationRow {
    pub id: String,
    pub conv_type: String,
    pub name: Option<String>,
    pub created_by: Option<String>,
    pub created_at: NaiveDateTime,
    pub last_message: Option<String>,
    pub last_time: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct ConversationResponse {
    pub id: String,
    pub r#type: String,
    pub name: Option<String>,
    pub created_by: Option<String>,
    pub created_at: NaiveDateTime,
    pub last_message: Option<String>,
    pub last_time: Option<NaiveDateTime>,
    pub participants: Vec<ParticipantInfo>,
    pub my_role: String,
    pub my_nickname: Option<String>,
    pub my_group_note: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ParticipantInfo {
    pub id: String,
    pub name: String,
    pub role: Option<String>,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MessageRow {
    pub id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub msg_type: String,
    pub content: Option<String>,
    pub file_url: Option<String>,
    pub file_name: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_avatar: Option<String>,
    pub r#type: String,
    pub content: Option<String>,
    pub file_url: Option<String>,
    pub file_name: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: Option<String>,
    pub msg_type: Option<String>,
    pub file_url: Option<String>,
    pub file_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BlockRequest {
    pub blocked_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateParticipantRequest {
    pub role: Option<String>,
    pub nickname: Option<String>,
    pub group_note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupNameRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AddParticipantRequest {
    pub employee_id: String,
}

async fn get_participants(pool: &sqlx::MySqlPool, conv_id: &str) -> Vec<ParticipantInfo> {
    sqlx::query_as(
        "SELECT e.id, e.name, cp.role, cp.nickname, e.avatar
         FROM conversation_participants cp
         JOIN employees e ON cp.employee_id = e.id
         WHERE cp.conversation_id = ?",
    )
    .bind(conv_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

async fn is_admin(pool: &sqlx::MySqlPool, conv_id: &str, user_id: &str) -> bool {
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM conversation_participants WHERE conversation_id = ? AND employee_id = ?",
    )
    .bind(conv_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    role.as_deref() == Some("admin")
}

fn count_admins(participants: &[ParticipantInfo]) -> usize {
    participants.iter().filter(|p| p.role.as_deref() == Some("admin")).count()
}

pub async fn list_conversations(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApiResponse<Vec<ConversationResponse>>>, AppError> {
    let rows: Vec<ConversationRow> = sqlx::query_as(
        "SELECT c.id, c.type AS conv_type, c.name, c.created_by, c.created_at,
         (SELECT m.content FROM messages m WHERE m.conversation_id = c.id ORDER BY m.created_at DESC LIMIT 1) AS last_message,
         (SELECT m.created_at FROM messages m WHERE m.conversation_id = c.id ORDER BY m.created_at DESC LIMIT 1) AS last_time
         FROM conversations c
         WHERE c.id IN (SELECT cp.conversation_id FROM conversation_participants cp WHERE cp.employee_id = ?)
         ORDER BY last_time DESC, c.created_at DESC",
    )
    .bind(&auth.id)
    .fetch_all(&state.pool)
    .await?;

    let mut result = Vec::new();
    for row in rows {
        let participants = get_participants(&state.pool, &row.id).await;
        let my_info = participants.iter().find(|p| p.id == auth.id);
        let my_role = my_info.map(|p| p.role.clone().unwrap_or_default()).unwrap_or_default();
        let my_nickname = my_info.and_then(|p| p.nickname.clone());
        let my_group_note: Option<String> = sqlx::query_scalar(
            "SELECT group_note FROM conversation_participants WHERE conversation_id = ? AND employee_id = ?",
        )
        .bind(&row.id)
        .bind(&auth.id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten();

        result.push(ConversationResponse {
            id: row.id,
            r#type: row.conv_type,
            name: row.name,
            created_by: row.created_by,
            created_at: row.created_at,
            last_message: row.last_message,
            last_time: row.last_time,
            participants,
            my_role,
            my_nickname,
            my_group_note,
        });
    }

    Ok(Json(ApiResponse::ok(result)))
}



pub async fn get_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(conv_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<MessageResponse>>>, AppError> {
    let is_member: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM conversation_participants WHERE conversation_id = ? AND employee_id = ?",
    )
    .bind(&conv_id)
    .bind(&auth.id)
    .fetch_one(&state.pool)
    .await?;

    if is_member == 0 {
        return Err(AppError::Forbidden);
    }

    // 单聊备注：仅在自己视角生效，对方发来的消息显示"我"为其设置的备注
    let is_single = sqlx::query_scalar::<_, String>("SELECT type FROM conversations WHERE id = ?")
        .bind(&conv_id)
        .fetch_optional(&state.pool)
        .await?
        .as_deref()
        == Some("single");
    let my_group_note: Option<String> = if is_single {
        sqlx::query_scalar("SELECT group_note FROM conversation_participants WHERE conversation_id = ? AND employee_id = ?")
            .bind(&conv_id)
            .bind(&auth.id)
            .fetch_optional(&state.pool)
            .await?
            .flatten()
    } else {
        None
    };

    let rows: Vec<MessageRow> = sqlx::query_as(
        "SELECT id, conversation_id, sender_id, type AS msg_type, content, file_url, file_name, created_at
         FROM messages WHERE conversation_id = ? ORDER BY created_at ASC LIMIT 200",
    )
    .bind(&conv_id)
    .fetch_all(&state.pool)
    .await?;

    let mut result = Vec::new();
    for row in rows {
        let sender_info: (String, Option<String>) = sqlx::query_as(
            "SELECT COALESCE(cp.nickname, e.name) AS name, e.avatar FROM employees e
             LEFT JOIN conversation_participants cp ON cp.conversation_id = ? AND cp.employee_id = e.id
             WHERE e.id = ?",
        )
        .bind(&conv_id)
        .bind(&row.sender_id)
        .fetch_one(&state.pool)
        .await?;
        let (sender_name, sender_avatar) = (sender_info.0, sender_info.1);
        let sender_name = if is_single && row.sender_id != auth.id {
            match &my_group_note {
                Some(note) if !note.is_empty() => note.clone(),
                _ => sender_name,
            }
        } else {
            sender_name
        };

        result.push(MessageResponse {
            id: row.id,
            conversation_id: row.conversation_id,
            sender_id: row.sender_id.clone(),
            sender_name,
            sender_avatar,
            r#type: row.msg_type,
            content: row.content,
            file_url: row.file_url,
            file_name: row.file_name,
            created_at: row.created_at,
        });
    }

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn send_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(conv_id): Path<String>,
    Json(body): Json<SendMessageRequest>,
) -> Result<Json<ApiResponse<MessageResponse>>, AppError> {
    let is_member: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM conversation_participants WHERE conversation_id = ? AND employee_id = ?",
    )
    .bind(&conv_id)
    .bind(&auth.id)
    .fetch_one(&state.pool)
    .await?;

    if is_member == 0 {
        return Err(AppError::Forbidden);
    }

    let conv_type: String = sqlx::query_scalar("SELECT type FROM conversations WHERE id = ?")
        .bind(&conv_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    if conv_type == "single" {
        let other_id: Option<String> = sqlx::query_scalar(
            "SELECT employee_id FROM conversation_participants WHERE conversation_id = ? AND employee_id != ?",
        )
        .bind(&conv_id)
        .bind(&auth.id)
        .fetch_optional(&state.pool)
        .await?;

        if let Some(other) = other_id {
            let blocked: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM blocked_users WHERE blocker_id = ? AND blocked_id = ?",
            )
            .bind(&other)
            .bind(&auth.id)
            .fetch_one(&state.pool)
            .await?;
            if blocked > 0 {
                return Err(AppError::BadRequest("对方已拉黑你".to_string()));
            }
        }
    }

    let msg_id = Uuid::new_v4().to_string();
    let msg_type = body.msg_type.as_deref().unwrap_or("text");

    // F-10: 文件消息必须携带 file_url，且 file_url 必须指向本站 /uploads 下、扩展名合法的文件
    // （任意文件扩展名，与 /api/upload/file 上传白名单一致）。
    // 防止用户可控任意链接（钓鱼跳转、javascript: 伪协议等）通过文件消息传播。
    if msg_type == "file" && body.file_url.is_none() {
        return Err(AppError::BadRequest("文件消息必须携带文件链接".to_string()));
    }
    if let Some(ref url) = body.file_url {
        let is_local_upload = url.starts_with("/uploads/")
            && {
                let name = url.trim_start_matches("/uploads/");
                let ext = std::path::Path::new(name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_ascii_lowercase())
                    .unwrap_or_default();
                crate::handlers::system::is_uploadable_extension(&ext)
            };
        if !is_local_upload {
            return Err(AppError::BadRequest(
                "文件链接必须指向本站上传的文件".to_string(),
            ));
        }
    }
    if let Some(ref name) = body.file_name {
        if name.chars().count() > 256 {
            return Err(AppError::BadRequest("文件名过长".to_string()));
        }
    }

    sqlx::query(
        "INSERT INTO messages (id, conversation_id, sender_id, type, content, file_url, file_name) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&msg_id)
    .bind(&conv_id)
    .bind(&auth.id)
    .bind(msg_type)
    .bind(&body.content)
    .bind(&body.file_url)
    .bind(&body.file_name)
    .execute(&state.pool)
    .await?;

    let sender_info: (String, Option<String>) = sqlx::query_as(
        "SELECT COALESCE(cp.nickname, e.name) AS name, e.avatar FROM employees e
         LEFT JOIN conversation_participants cp ON cp.conversation_id = ? AND cp.employee_id = e.id
         WHERE e.id = ?",
    )
    .bind(&conv_id)
    .bind(&auth.id)
    .fetch_one(&state.pool)
    .await?;
    let (sender_name, sender_avatar) = (sender_info.0, sender_info.1);

    let msg_preview = body.content.as_deref().unwrap_or("").chars().take(50).collect::<String>();
    append_log(&state.config.log_file, &format!("用户 {} 在会话 {} 中发送了消息: {}", user_tag(&auth.name, &auth.username), conv_id, msg_preview));

    Ok(Json(ApiResponse::ok(MessageResponse {
        id: msg_id,
        conversation_id: conv_id,
        sender_id: auth.id,
        sender_name,
        sender_avatar,
        r#type: msg_type.to_string(),
        content: body.content.clone(),
        file_url: body.file_url.clone(),
        file_name: body.file_name.clone(),
        created_at: chrono::Utc::now().naive_utc(),
    })))
}

async fn resolve_employee_display(pool: &sqlx::Pool<sqlx::MySql>, id: &str) -> String {
    #[derive(sqlx::FromRow)]
    struct Emp { name: String, username: String }
    sqlx::query_as::<_, Emp>("SELECT name, username FROM employees WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|e| format!("{} ({})", e.name, e.username))
        .unwrap_or_else(|| format!("[已删除: {}]", id))
}

pub async fn block_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<BlockRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if body.blocked_id == auth.id {
        return Err(AppError::BadRequest("不能拉黑自己".to_string()));
    }
    let protected: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM employees WHERE id = ? AND protect_block = 1",
    )
    .bind(&body.blocked_id)
    .fetch_one(&state.pool)
    .await?;
    if protected > 0 {
        return Err(AppError::BadRequest("该用户受保护，无法拉黑".to_string()));
    }
    sqlx::query("INSERT IGNORE INTO blocked_users (blocker_id, blocked_id) VALUES (?, ?)")
        .bind(&auth.id)
        .bind(&body.blocked_id)
        .execute(&state.pool)
        .await?;

    let target = resolve_employee_display(&state.pool, &body.blocked_id).await;
    append_log(&state.config.log_file, &format!("用户 {} 拉黑了用户 {}", user_tag(&auth.name, &auth.username), target));
    Ok(Json(ApiResponse::ok_msg("已拉黑")))
}

pub async fn unblock_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(blocked_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    sqlx::query("DELETE FROM blocked_users WHERE blocker_id = ? AND blocked_id = ?")
        .bind(&auth.id)
        .bind(&blocked_id)
        .execute(&state.pool)
        .await?;

    let target = resolve_employee_display(&state.pool, &blocked_id).await;
    append_log(&state.config.log_file, &format!("用户 {} 取消了拉黑 {}", user_tag(&auth.name, &auth.username), target));
    Ok(Json(ApiResponse::ok_msg("已取消拉黑")))
}

pub async fn list_blocked(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApiResponse<Vec<ParticipantInfo>>>, AppError> {
    let rows: Vec<ParticipantInfo> = sqlx::query_as(
        "SELECT e.id,
                COALESCE(
                    (SELECT cp.nickname FROM conversation_participants cp
                     WHERE cp.employee_id = e.id
                     AND cp.conversation_id IN (
                         SELECT c.id FROM conversations c
                         WHERE c.type = 'single'
                         AND EXISTS (SELECT 1 FROM conversation_participants WHERE conversation_id = c.id AND employee_id = bu.blocker_id)
                         AND EXISTS (SELECT 1 FROM conversation_participants WHERE conversation_id = c.id AND employee_id = e.id)
                     )
                     LIMIT 1),
                    e.name
                ) AS name,
                NULL AS role,
                NULL AS nickname,
                e.avatar
         FROM blocked_users bu
         JOIN employees e ON bu.blocked_id = e.id
         WHERE bu.blocker_id = ?",
    )
    .bind(&auth.id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(ApiResponse::ok(rows)))
}

pub async fn list_employees_for_chat(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApiResponse<Vec<ParticipantInfo>>>, AppError> {
    // 部门数据范围机制已移除：聊天名单对所有登录用户可见（用于发起会话/拉人）。
    let rows: Vec<ParticipantInfo> = sqlx::query_as(
        "SELECT id, name, NULL AS role, NULL AS nickname, avatar FROM employees WHERE id != ? ORDER BY name",
    )
    .bind(&auth.id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(ApiResponse::ok(rows)))
}

pub async fn update_participant(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((conv_id, target_id)): Path<(String, String)>,
    Json(body): Json<UpdateParticipantRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let conv_type: Option<String> = sqlx::query_scalar(
        "SELECT type FROM conversations WHERE id = ?",
    )
    .bind(&conv_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let is_group = conv_type.as_deref() == Some("group");

    if is_group {
        if !is_admin(&state.pool, &conv_id, &auth.id).await && auth.id != target_id {
            return Err(AppError::Forbidden);
        }
    } else {
        // F-09/F-19: 单聊会话仅允许修改自己的昵称/备注，禁止修改对方昵称与角色字段；
        // 且操作者必须是该会话成员（防对非本会话提交写操作）。
        let is_member: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM conversation_participants WHERE conversation_id = ? AND employee_id = ?",
        )
        .bind(&conv_id)
        .bind(&auth.id)
        .fetch_one(&state.pool)
        .await?;
        if is_member == 0 {
            return Err(AppError::Forbidden);
        }
        if body.role.is_some() {
            return Err(AppError::Forbidden);
        }
        if body.nickname.is_some() && target_id != auth.id {
            return Err(AppError::Forbidden);
        }
    }

    if let Some(role) = &body.role {
        if target_id == auth.id {
            return Err(AppError::Forbidden);
        }
        if role == "admin" || role == "member" {
            let participants = get_participants(&state.pool, &conv_id).await;
            if role == "member" && count_admins(&participants) <= 1 && auth.id != target_id {
                return Err(AppError::BadRequest("群聊至少需要一名管理员".to_string()));
            }
            sqlx::query("UPDATE conversation_participants SET role = ? WHERE conversation_id = ? AND employee_id = ?")
                .bind(role)
                .bind(&conv_id)
                .bind(&target_id)
                .execute(&state.pool)
                .await?;
        }
    }

    if let Some(nickname) = &body.nickname {
        // 昵称仅允许修改自己（群聊与单聊一致）。
        sqlx::query("UPDATE conversation_participants SET nickname = ? WHERE conversation_id = ? AND employee_id = ?")
            .bind(nickname)
            .bind(&conv_id)
            .bind(&auth.id)
            .execute(&state.pool)
            .await?;
    }

    if let Some(group_note) = &body.group_note {
        sqlx::query("UPDATE conversation_participants SET group_note = ? WHERE conversation_id = ? AND employee_id = ?")
            .bind(group_note)
            .bind(&conv_id)
            .bind(&auth.id)
            .execute(&state.pool)
            .await?;
    }

    Ok(Json(ApiResponse::ok_msg("已更新")))
}

pub async fn update_group_name(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(conv_id): Path<String>,
    Json(body): Json<UpdateGroupNameRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if !is_admin(&state.pool, &conv_id, &auth.id).await {
        return Err(AppError::Forbidden);
    }
    sqlx::query("UPDATE conversations SET name = ? WHERE id = ?")
        .bind(&body.name)
        .bind(&conv_id)
        .execute(&state.pool)
        .await?;

    Ok(Json(ApiResponse::ok_msg("已更新")))
}

pub async fn add_participant(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(conv_id): Path<String>,
    Json(body): Json<AddParticipantRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if !is_admin(&state.pool, &conv_id, &auth.id).await {
        return Err(AppError::Forbidden);
    }

    // F-12: 校验目标员工存在，避免向会话写入不存在的参与者（脏数据）。
    let emp_exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM employees WHERE id = ?",
    )
    .bind(&body.employee_id)
    .fetch_one(&state.pool)
    .await?;
    if emp_exists == 0 {
        return Err(AppError::NotFound);
    }

    sqlx::query(
        "INSERT IGNORE INTO conversation_participants (conversation_id, employee_id, role) VALUES (?, ?, 'member')",
    )
    .bind(&conv_id)
    .bind(&body.employee_id)
    .execute(&state.pool)
    .await?;

    Ok(Json(ApiResponse::ok_msg("已添加")))
}

pub async fn remove_participant(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((conv_id, target_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if !is_admin(&state.pool, &conv_id, &auth.id).await {
        return Err(AppError::Forbidden);
    }
    let participants = get_participants(&state.pool, &conv_id).await;
    if target_id == auth.id {
        return Err(AppError::BadRequest("不能移除自己".to_string()));
    }
    if count_admins(&participants) <= 1 && participants.iter().any(|p| p.id == target_id && p.role.as_deref() == Some("admin")) {
        return Err(AppError::BadRequest("群聊至少需要一名管理员".to_string()));
    }
    sqlx::query("DELETE FROM conversation_participants WHERE conversation_id = ? AND employee_id = ?")
        .bind(&conv_id)
        .bind(&target_id)
        .execute(&state.pool)
        .await?;

    Ok(Json(ApiResponse::ok_msg("已移除")))
}

pub async fn disband_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(conv_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    if !is_admin(&state.pool, &conv_id, &auth.id).await {
        return Err(AppError::Forbidden);
    }
    sqlx::query("DELETE FROM conversations WHERE id = ?")
        .bind(&conv_id)
        .execute(&state.pool)
        .await?;

    Ok(Json(ApiResponse::ok_msg("群聊已解散")))
}

pub async fn update_protect_block(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(emp_id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "chat:protect_block")?;

    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM employees WHERE id = ?")
        .bind(&emp_id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }

    let value = body.get("protect_block").and_then(|v| v.as_i64()).unwrap_or(0) as i8;
    sqlx::query("UPDATE employees SET protect_block = ? WHERE id = ?")
        .bind(value)
        .bind(&emp_id)
        .execute(&state.pool)
        .await?;
    Ok(Json(ApiResponse::ok_msg("已更新")))
}

/// F-24: 聊天文件下载接口（替代 /uploads 静态直链）。
/// 三重校验：文件名白名单格式 → 文件必须被某条消息引用（属于某个会话）→
/// 当前用户是相关会话成员。防止任意登录用户（或未登录用户）通过猜测/泄露的
/// /uploads/<uuid>.<ext> 直链下载他人聊天文件。
pub async fn get_chat_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(name): Path<String>,
) -> Result<Response, AppError> {
    // 1) 文件名白名单：ASCII 字母数字 + 点 + 连字符（UUID 文件名含 "-"），禁止 ".."、
    //    前导点、超长（防路径遍历/编码绕过）。扩展名单独白名单校验。
    if name.is_empty()
        || name.len() > 64
        || name.starts_with('.')
        || name.contains("..")
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err(AppError::NotFound);
    }
    let ext = std::path::Path::new(&name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    if !crate::handlers::system::is_uploadable_extension(&ext) {
        return Err(AppError::NotFound);
    }

    // 2) 文件必须被某条消息引用（即有会话归属），否则无人可访问（上传后未发送的文件不可读）。
    //    兼容新格式 /uploads/chat/<name> 与存量格式 /uploads/<name>。
    let file_url_candidates = [
        format!("/uploads/chat/{}", name),
        format!("/uploads/{}", name),
    ];
    let mut conv_ids: Vec<String> = Vec::new();
    for file_url in &file_url_candidates {
        let hits: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT conversation_id FROM messages WHERE file_url = ?",
        )
        .bind(file_url)
        .fetch_all(&state.pool)
        .await?;
        conv_ids.extend(hits);
    }
    conv_ids.sort();
    conv_ids.dedup();

    // 3) 当前用户必须是任一相关会话的成员
    let mut allowed = false;
    for cid in &conv_ids {
        let is_member: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM conversation_participants WHERE conversation_id = ? AND employee_id = ?",
        )
        .bind(cid)
        .bind(&auth.id)
        .fetch_one(&state.pool)
        .await?;
        if is_member > 0 {
            allowed = true;
            break;
        }
    }
    if !allowed {
        return Err(AppError::Forbidden);
    }

    // 4) 读文件并流式返回；图片内联，其余强制下载（与静态目录守卫行为一致）。
    //    优先 chat/ 子目录（新文件），缺失则回退根目录（存量文件）。
    //    F-25: 读取前检查两处候选路径，任一为软链接即 404（防符号链接逃逸读取任意文件）。
    let chat_path = std::path::Path::new(&state.config.upload_dir).join("chat").join(&name);
    let root_path = std::path::Path::new(&state.config.upload_dir).join(&name);
    for cand in [&chat_path, &root_path] {
        if let Ok(m) = tokio::fs::symlink_metadata(cand).await {
            if m.file_type().is_symlink() {
                return Err(AppError::NotFound);
            }
        }
    }
    let data = match tokio::fs::read(&chat_path).await {
        Ok(d) => d,
        Err(_) => tokio::fs::read(&root_path).await.map_err(|_| AppError::NotFound)?,
    };

    let content_type = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "txt" => "text/plain; charset=utf-8",
        "md" => "text/markdown; charset=utf-8",
        "log" => "text/plain; charset=utf-8",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogg" => "audio/ogg",
        "mov" => "video/quicktime",
        _ => "application/octet-stream",
    };

    let mut builder = Response::builder().header(header::CONTENT_TYPE, content_type);
    if !crate::handlers::system::ALLOWED_UPLOAD_EXTS.contains(&ext.as_str()) {
        builder = builder.header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", name),
        );
    }
    builder
        .body(Body::from(data))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("构建文件响应失败: {}", e)))
}

