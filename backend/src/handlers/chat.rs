use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::Response;
use axum::Json;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::handlers::auth::{append_log, user_tag, ClientIp};
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
    /// 当前请求认证用户的 id（服务端身份，前端据此判断「自己/对方」，避免与前端本地状态不一致）
    pub my_id: String,
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
    /// 当前请求认证用户的 id（服务端身份，前端据此判断消息是否「自己发送」）
    pub my_id: String,
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

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub member_ids: Vec<String>,
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
            my_id: auth.id.clone(),
        });
    }

    Ok(Json(ApiResponse::ok(result)))
}

/// 获取或创建与指定员工的单聊会话（同一对用户只保留一个会话，供「点击姓名发起聊天」使用）。
pub async fn get_or_create_direct_conversation(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(peer_id): Path<String>,
) -> Result<Json<ApiResponse<ConversationResponse>>, AppError> {
    if peer_id == auth.id {
        return Err(AppError::BadRequest("不能和自己聊天".to_string()));
    }
    let peer_exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM employees WHERE id = ?")
        .bind(&peer_id)
        .fetch_one(&state.pool)
        .await?;
    if peer_exists == 0 {
        return Err(AppError::NotFound);
    }

    // 复用已存在的单聊会话；同一对用户只保留一个（按创建时间取最早，避免历史重复会话）。
    let conv_id: Option<String> = sqlx::query_scalar(
        "SELECT c.id FROM conversations c
         WHERE c.type = 'single'
           AND EXISTS (SELECT 1 FROM conversation_participants cp WHERE cp.conversation_id = c.id AND cp.employee_id = ?)
           AND EXISTS (SELECT 1 FROM conversation_participants cp WHERE cp.conversation_id = c.id AND cp.employee_id = ?)
         ORDER BY c.created_at ASC LIMIT 1",
    )
    .bind(&auth.id)
    .bind(&peer_id)
    .fetch_optional(&state.pool)
    .await?;

    let conv_id = match conv_id {
        Some(id) => id,
        None => {
            let conv_id = Uuid::new_v4().to_string();
            let mut tx = state.pool.begin().await?;
            sqlx::query(
                "INSERT INTO conversations (id, type, name, created_by) VALUES (?, 'single', NULL, ?)",
            )
            .bind(&conv_id)
            .bind(&auth.id)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "INSERT INTO conversation_participants (conversation_id, employee_id, role) VALUES (?, ?, 'member'), (?, ?, 'member')",
            )
            .bind(&conv_id)
            .bind(&auth.id)
            .bind(&conv_id)
            .bind(&peer_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            conv_id
        }
    };

    let participants = get_participants(&state.pool, &conv_id).await;
    let my_info = participants.iter().find(|p| p.id == auth.id);
    let my_role = my_info.map(|p| p.role.clone().unwrap_or_default()).unwrap_or_default();
    let my_nickname = my_info.and_then(|p| p.nickname.clone());
    let my_group_note: Option<String> = sqlx::query_scalar(
        "SELECT group_note FROM conversation_participants WHERE conversation_id = ? AND employee_id = ?",
    )
    .bind(&conv_id)
    .bind(&auth.id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let last_message: Option<String> = sqlx::query_scalar(
        "SELECT m.content FROM messages m WHERE m.conversation_id = ? ORDER BY m.created_at DESC LIMIT 1",
    )
    .bind(&conv_id)
    .fetch_optional(&state.pool)
    .await?;
    let last_time: Option<NaiveDateTime> = sqlx::query_scalar(
        "SELECT m.created_at FROM messages m WHERE m.conversation_id = ? ORDER BY m.created_at DESC LIMIT 1",
    )
    .bind(&conv_id)
    .fetch_optional(&state.pool)
    .await?;

    Ok(Json(ApiResponse::ok(ConversationResponse {
        id: conv_id,
        r#type: "single".to_string(),
        name: None,
        created_by: Some(auth.id.clone()),
        created_at: chrono::Utc::now().naive_utc(),
        last_message,
        last_time,
        participants,
        my_role,
        my_nickname,
        my_group_note,
        my_id: auth.id,
    })))
}

pub async fn create_group_conversation(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateGroupRequest>,
) -> Result<Json<ApiResponse<ConversationResponse>>, AppError> {
    require_permission(&auth.permissions, "chat:group_create")?;

    // 群名校验：trim 后非空、≤128 字符（conversations.name VARCHAR(128)）
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("群名不能为空".to_string()));
    }
    if name.chars().count() > 128 {
        return Err(AppError::BadRequest("群名不能超过 128 个字符".to_string()));
    }

    // 成员去重、剔除创建者自己（创建者自动加入）；数量上限 100 防滥用
    let mut member_ids: Vec<String> = Vec::new();
    for id in &body.member_ids {
        if id == &auth.id || member_ids.contains(id) {
            continue;
        }
        member_ids.push(id.clone());
    }
    if member_ids.len() > 100 {
        return Err(AppError::BadRequest("群聊成员数量不能超过 100 人".to_string()));
    }

    // 校验所有成员在 employees 表中存在（动态 IN 查询，防脏数据）
    if !member_ids.is_empty() {
        let sql = format!(
            "SELECT id FROM employees WHERE id IN ({})",
            vec!["?"; member_ids.len()].join(",")
        );
        let mut query = sqlx::query_scalar::<_, String>(&sql);
        for id in &member_ids {
            query = query.bind(id);
        }
        let found: Vec<String> = query.fetch_all(&state.pool).await?;
        if found.len() != member_ids.len() {
            return Err(AppError::BadRequest("包含无效成员".to_string()));
        }
    }

    // 事务：创建会话 + 写入参与者（创建者为 admin，其余为 member）
    let conv_id = Uuid::new_v4().to_string();
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO conversations (id, type, name, created_by) VALUES (?, 'group', ?, ?)",
    )
    .bind(&conv_id)
    .bind(name)
    .bind(&auth.id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO conversation_participants (conversation_id, employee_id, role) VALUES (?, ?, 'admin')",
    )
    .bind(&conv_id)
    .bind(&auth.id)
    .execute(&mut *tx)
    .await?;
    for id in &member_ids {
        sqlx::query(
            "INSERT INTO conversation_participants (conversation_id, employee_id, role) VALUES (?, ?, 'member')",
        )
        .bind(&conv_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    let participants = get_participants(&state.pool, &conv_id).await;

    Ok(Json(ApiResponse::ok(ConversationResponse {
        id: conv_id,
        r#type: "group".to_string(),
        name: Some(name.to_string()),
        created_by: Some(auth.id.clone()),
        created_at: chrono::Utc::now().naive_utc(),
        last_message: None,
        last_time: None,
        participants,
        my_role: "admin".to_string(),
        my_nickname: None,
        my_group_note: None,
        my_id: auth.id,
    })))
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
            my_id: auth.id.clone(),
        });
    }

    Ok(Json(ApiResponse::ok(result)))
}

pub async fn send_message(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(conv_id): Path<String>,
    Json(body): Json<SendMessageRequest>,
) -> Result<Json<ApiResponse<MessageResponse>>, AppError> {
    let ip = client_ip.0;
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

    // 消息类型白名单（防任意字符串污染 type 列）
    if msg_type != "text" && msg_type != "file" {
        return Err(AppError::BadRequest("不支持的消息类型".to_string()));
    }
    // 消息内容长度上限（防单条消息撑爆 TEXT 列与内存）
    if let Some(ref content) = body.content {
        if content.chars().count() > 20_000 {
            return Err(AppError::BadRequest("消息内容过长".to_string()));
        }
    }

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
        // F-6: 文件归属校验——文件若已被其他会话引用，发送者必须是相关会话的成员；
        // 未被任何会话引用的（新上传）文件必须真实存在于上传目录。
        // 防止「知晓 UUID 即可跨会话引用/传播他人文件」与「引用不存在的伪造路径」。
        let referencing: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT conversation_id FROM messages WHERE file_url = ?",
        )
        .bind(url)
        .fetch_all(&state.pool)
        .await?;
        if !referencing.is_empty() {
            let mut allowed = false;
            for cid in &referencing {
                let n: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM conversation_participants WHERE conversation_id = ? AND employee_id = ?",
                )
                .bind(cid)
                .bind(&auth.id)
                .fetch_one(&state.pool)
                .await?;
                if n > 0 {
                    allowed = true;
                    break;
                }
            }
            if !allowed {
                return Err(AppError::Forbidden);
            }
        } else {
            // 新上传文件：校验真实存在（chat/ 子目录优先，根目录兜底存量文件）。
            // 与 get_chat_file 一致：路径任一级为软链接视为不存在（防符号链接逃逸引用）。
            let name = url.trim_start_matches("/uploads/");
            let chat_path =
                std::path::Path::new(&state.config.upload_dir).join("chat").join(name);
            let root_path = std::path::Path::new(&state.config.upload_dir).join(name);
            let mut exists = false;
            for cand in [&chat_path, &root_path] {
                match tokio::fs::symlink_metadata(cand).await {
                    Ok(m) if !m.file_type().is_symlink() => {
                        exists = true;
                        break;
                    }
                    _ => {}
                }
            }
            if !exists {
                return Err(AppError::BadRequest(
                    "文件不存在，请重新上传".to_string(),
                ));
            }
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
    append_log(
        &state.config.log_file,
        &format!(
            "User {} sent a message in conversation {}: {}",
            user_tag(&auth.name, &auth.username),
            conv_id,
            msg_preview
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok(MessageResponse {
        id: msg_id,
        conversation_id: conv_id,
        sender_id: auth.id.clone(),
        sender_name,
        sender_avatar,
        r#type: msg_type.to_string(),
        content: body.content.clone(),
        file_url: body.file_url.clone(),
        file_name: body.file_name.clone(),
        created_at: chrono::Utc::now().naive_utc(),
        my_id: auth.id,
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
        .unwrap_or_else(|| format!("[deleted: {}]", id))
}

pub async fn block_user(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Json(body): Json<BlockRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let ip = client_ip.0;
    if body.blocked_id == auth.id {
        return Err(AppError::BadRequest("不能拉黑自己".to_string()));
    }
    // 防拉黑保护：目标员工有效权限含 chat:protect_block 则不可拉黑（角色/部门角色派生）。
    let target_grants =
        crate::services::permission::resolve_effective_grants(&state.pool, &body.blocked_id).await?;
    if crate::services::permission::has_permission(&target_grants, "chat:protect_block") {
        return Err(AppError::BadRequest("该用户受保护，无法拉黑".to_string()));
    }
    sqlx::query("INSERT IGNORE INTO blocked_users (blocker_id, blocked_id) VALUES (?, ?)")
        .bind(&auth.id)
        .bind(&body.blocked_id)
        .execute(&state.pool)
        .await?;

    let target = resolve_employee_display(&state.pool, &body.blocked_id).await;
    append_log(
        &state.config.log_file,
        &format!(
            "User {} blocked user {}",
            user_tag(&auth.name, &auth.username),
            target
        ),
        &ip,
    );
    Ok(Json(ApiResponse::ok_msg("已拉黑")))
}

pub async fn unblock_user(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(blocked_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let ip = client_ip.0;
    sqlx::query("DELETE FROM blocked_users WHERE blocker_id = ? AND blocked_id = ?")
        .bind(&auth.id)
        .bind(&blocked_id)
        .execute(&state.pool)
        .await?;

    let target = resolve_employee_display(&state.pool, &blocked_id).await;
    append_log(
        &state.config.log_file,
        &format!(
            "User {} unblocked {}",
            user_tag(&auth.name, &auth.username),
            target
        ),
        &ip,
    );
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
    // 聊天名单按 employee:view 数据范围过滤：持有该权限的用户仅看到范围内员工；
    // 未持有 employee:view 的用户不受限制（聊天功能保持可用）。
    let scope =
        crate::services::permission::build_scope(&state.pool, &auth.grants, "employee:view", &auth.id)
            .await?;

    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT id, name, NULL AS role, NULL AS nickname, avatar FROM employees e WHERE e.id != ",
    );
    qb.push_bind(&auth.id);
    if let Some(scope) = &scope {
        crate::services::permission::apply_scope_filter(&mut qb, scope, "e");
    }
    qb.push(" ORDER BY name");

    let rows: Vec<ParticipantInfo> = qb.build_query_as().fetch_all(&state.pool).await?;

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
    // F-9: 与创建群聊一致的群名校验（trim 后非空、≤128 字符，与列宽一致）。
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("群名不能为空".to_string()));
    }
    if name.chars().count() > 128 {
        return Err(AppError::BadRequest("群名不能超过 128 个字符".to_string()));
    }
    sqlx::query("UPDATE conversations SET name = ? WHERE id = ?")
        .bind(name)
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

