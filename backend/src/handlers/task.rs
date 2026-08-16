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

//! 任务模块：员工创建/完成个人任务，task:view_all 管理员可查看全员任务。
//! 与财务模块相互独立（仅共享 employees 基础表）。

use axum::extract::{Path, Query, State};
use axum::Json;
use uuid::Uuid;

use crate::error::AppError;
use crate::handlers::auth::{append_log, user_tag, ClientIp};
use crate::middleware::auth::{require_permission, AppState, AuthUser};
use crate::models::task::{NewTask, TaskQuery, TaskRow, UpdateTask};
use crate::utils::response::ApiResponse;

/// 校验负责人存在，返回 assignee_id。
async fn validate_assignee(
    pool: &sqlx::MySqlPool,
    assignee_id: Option<&str>,
    fallback_id: &str,
) -> Result<String, AppError> {
    let aid = assignee_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback_id.to_string());
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM employees WHERE id = ?")
        .bind(&aid)
        .fetch_one(pool)
        .await?;
    if n == 0 {
        return Err(AppError::BadRequest("指定的负责人不存在".to_string()));
    }
    Ok(aid)
}

/// 任务列表：无 task:view_all 仅见本人（本人创建或本人负责）；有则可按 scope/assignee 过滤全员。
pub async fn list_tasks(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<TaskQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let can_view_all = auth.permissions.iter().any(|p| p == "task:view_all");

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT t.id, t.title, t.description, t.assignee_id, a.name AS assignee_name, \
         t.creator_id, c.name AS creator_name, t.status, t.due_date, t.completed_at, t.created_at \
         FROM tasks t \
         JOIN employees a ON a.id = t.assignee_id \
         JOIN employees c ON c.id = t.creator_id \
         WHERE 1 = 1",
    );

    if !can_view_all {
        // 仅本人相关：创建者或负责人。
        qb.push(" AND (t.creator_id = ").push_bind(&auth.id);
        qb.push(" OR t.assignee_id = ").push_bind(&auth.id).push(")");
    } else {
        if query.scope.as_deref() == Some("mine") {
            qb.push(" AND (t.creator_id = ").push_bind(&auth.id);
            qb.push(" OR t.assignee_id = ").push_bind(&auth.id).push(")");
        } else if let Some(assignee) = &query.assignee_id {
            if !assignee.is_empty() {
                qb.push(" AND t.assignee_id = ").push_bind(assignee);
            }
        }
    }
    if let Some(status) = &query.status {
        if !status.is_empty() {
            qb.push(" AND t.status = ").push_bind(status);
        }
    }
    qb.push(" ORDER BY t.status ASC, t.due_date ASC, t.created_at DESC LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind((page - 1) * page_size);

    let items: Vec<TaskRow> = qb.build_query_as().fetch_all(&state.pool).await?;

    // 计数（同条件）。
    let mut cqb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT COUNT(*) FROM tasks t WHERE 1 = 1",
    );
    if !can_view_all {
        cqb.push(" AND (t.creator_id = ").push_bind(&auth.id);
        cqb.push(" OR t.assignee_id = ").push_bind(&auth.id).push(")");
    } else {
        if query.scope.as_deref() == Some("mine") {
            cqb.push(" AND (t.creator_id = ").push_bind(&auth.id);
            cqb.push(" OR t.assignee_id = ").push_bind(&auth.id).push(")");
        } else if let Some(assignee) = &query.assignee_id {
            if !assignee.is_empty() {
                cqb.push(" AND t.assignee_id = ").push_bind(assignee);
            }
        }
    }
    if let Some(status) = &query.status {
        if !status.is_empty() {
            cqb.push(" AND t.status = ").push_bind(status);
        }
    }
    let total: i64 = cqb.build_query_scalar().fetch_one(&state.pool).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
        "can_view_all": can_view_all,
    }))))
}

/// 任务统计：总任务 / 未完成 / 已完成 / 逾期未完成（本人维度；管理员可传全员）。
pub async fn task_stats(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let can_view_all = auth.permissions.iter().any(|p| p == "task:view_all");

    // 本人可见条件（带括号，保证与 status 条件的 AND/OR 优先级正确）。
    let mine = "(creator_id = ? OR assignee_id = ?)";
    let base_clause = if can_view_all { "" } else { mine };

    let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM tasks WHERE 1 = 1 {}", base_clause))
        .bind(&auth.id)
        .bind(&auth.id)
        .fetch_one(&state.pool)
        .await?;
    let todo: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM tasks WHERE status = 'todo' AND 1 = 1 {}",
        base_clause
    ))
    .bind(&auth.id)
    .bind(&auth.id)
    .fetch_one(&state.pool)
    .await?;
    let done: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM tasks WHERE status = 'done' AND 1 = 1 {}",
        base_clause
    ))
    .bind(&auth.id)
    .bind(&auth.id)
    .fetch_one(&state.pool)
    .await?;
    let overdue: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM tasks WHERE status = 'todo' AND due_date IS NOT NULL \
         AND due_date < CURDATE() AND 1 = 1 {}",
        base_clause
    ))
    .bind(&auth.id)
    .bind(&auth.id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "total": total,
        "todo": todo,
        "done": done,
        "overdue": overdue,
        "can_view_all": can_view_all,
    }))))
}

/// 创建任务。
pub async fn create_task(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Json(body): Json<NewTask>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "task:create")?;
    let ip = client_ip.0;

    let title = body.title.trim().to_string();
    if title.is_empty() || title.chars().count() > 128 {
        return Err(AppError::ValidationError(
            "任务标题不能为空且不能超过 128 字符".to_string(),
        ));
    }
    if let Some(desc) = &body.description {
        if desc.chars().count() > 512 {
            return Err(AppError::ValidationError(
                "任务说明不能超过 512 字符".to_string(),
            ));
        }
    }
    let assignee_id = validate_assignee(&state.pool, body.assignee_id.as_deref(), &auth.id).await?;

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO tasks (id, title, description, assignee_id, creator_id, status, due_date) \
         VALUES (?, ?, ?, ?, ?, 'todo', ?)",
    )
    .bind(&id)
    .bind(&title)
    .bind(&body.description)
    .bind(&assignee_id)
    .bind(&auth.id)
    .bind(body.due_date)
    .execute(&state.pool)
    .await?;

    append_log(
        &state.config.log_file,
        &format!(
            "User {} created task {} (assignee {})",
            user_tag(&auth.name, &auth.username),
            title,
            assignee_id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::created(serde_json::json!({ "id": id }))))
}

/// 更新任务：本人（创建者或负责人）可改本人任务；task:manage 可改任意。
/// status 传 done/todo 即标记完成/未完成（完成时记录 completed_at）。
pub async fn update_task(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateTask>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let ip = client_ip.0;
    let can_manage = auth.permissions.iter().any(|p| p == "task:manage");

    let current: Option<(String, String)> = sqlx::query_as(
        "SELECT creator_id, assignee_id FROM tasks WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((creator_id, assignee_id)) = current else {
        return Err(AppError::NotFound);
    };
    if !can_manage && auth.id != creator_id && auth.id != assignee_id {
        return Err(AppError::Forbidden);
    }

    if let Some(ref title) = body.title {
        let title = title.trim();
        if title.is_empty() || title.chars().count() > 128 {
            return Err(AppError::ValidationError(
                "任务标题不能为空且不能超过 128 字符".to_string(),
            ));
        }
    }
    if let Some(desc) = &body.description {
        if desc.as_ref().map_or(false, |d| d.chars().count() > 512) {
            return Err(AppError::ValidationError(
                "任务说明不能超过 512 字符".to_string(),
            ));
        }
    }
    let new_assignee = match &body.assignee_id {
        Some(aid) => Some(validate_assignee(&state.pool, Some(aid), &auth.id).await?),
        None => None,
    };
    if let Some(status) = &body.status {
        if status != "todo" && status != "done" {
            return Err(AppError::ValidationError(
                "status 必须是 todo 或 done".to_string(),
            ));
        }
    }

    let mut tx = state.pool.begin().await?;
    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> =
        sqlx::QueryBuilder::new("UPDATE tasks SET ");
    let mut has_fields = false;
    if let Some(title) = &body.title {
        qb.push("title = ").push_bind(title.trim());
        has_fields = true;
    }
    if let Some(desc) = &body.description {
        if has_fields {
            qb.push(", ");
        }
        qb.push("description = ").push_bind(desc);
        has_fields = true;
    }
    if let Some(aid) = new_assignee {
        if has_fields {
            qb.push(", ");
        }
        qb.push("assignee_id = ").push_bind(aid);
        has_fields = true;
    }
    if let Some(due) = &body.due_date {
        if has_fields {
            qb.push(", ");
        }
        qb.push("due_date = ").push_bind(due);
        has_fields = true;
    }
    if let Some(status) = &body.status {
        if has_fields {
            qb.push(", ");
        }
        // 标记完成记录完成时间；未完成清除。
        qb.push("status = ").push_bind(status);
        if status == "done" {
            qb.push(", completed_at = NOW()");
        } else {
            qb.push(", completed_at = NULL");
        }
        has_fields = true;
    }
    if has_fields {
        qb.push(" WHERE id = ").push_bind(&id);
        qb.build().execute(&mut *tx).await?;
    }
    tx.commit().await?;

    append_log(
        &state.config.log_file,
        &format!(
            "User {} updated task {}",
            user_tag(&auth.name, &auth.username),
            id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("保存成功")))
}

/// 删除任务：本人（创建者或负责人）可删本人任务；task:manage 可删任意。
pub async fn delete_task(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let ip = client_ip.0;
    let can_manage = auth.permissions.iter().any(|p| p == "task:manage");

    let current: Option<(String, String)> = sqlx::query_as(
        "SELECT creator_id, assignee_id FROM tasks WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((creator_id, assignee_id)) = current else {
        return Err(AppError::NotFound);
    };
    if !can_manage && auth.id != creator_id && auth.id != assignee_id {
        return Err(AppError::Forbidden);
    }

    let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    append_log(
        &state.config.log_file,
        &format!(
            "User {} deleted task {}",
            user_tag(&auth.name, &auth.username),
            id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("删除成功")))
}
