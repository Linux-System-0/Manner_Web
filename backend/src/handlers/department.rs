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

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::error::AppError;
use crate::handlers::auth::{append_log, user_tag, ClientIp};
use crate::middleware::auth::{require_permission, AppState, AuthUser};
use crate::models::department::{
    DepartmentListRow, DepartmentMemberRow, NewDepartment, UpdateDepartment,
    UpdateEmployeeDepartmentsRequest,
};
use crate::utils::response::ApiResponse;

/// 校验负责人 id 列表：去重后全部须指向存在的员工，返回去重后的列表。
async fn validate_leaders(
    pool: &sqlx::MySqlPool,
    leader_ids: &[String],
) -> Result<Vec<String>, AppError> {
    let mut unique: Vec<String> = leader_ids.to_vec();
    unique.sort();
    unique.dedup();
    for lid in &unique {
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM employees WHERE id = ?")
            .bind(lid)
            .fetch_one(pool)
            .await?;
        if exists == 0 {
            return Err(AppError::BadRequest("指定的部门负责人不存在".to_string()));
        }
    }
    Ok(unique)
}

/// 校验 parent_id 是否指向存在的部门。
async fn validate_parent(
    pool: &sqlx::MySqlPool,
    parent_id: &str,
    exclude_id: Option<&str>,
) -> Result<(), AppError> {
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE id = ?")
        .bind(parent_id)
        .fetch_one(pool)
        .await?;
    if exists == 0 {
        return Err(AppError::BadRequest("指定的上级部门不存在".to_string()));
    }
    if let Some(exclude) = exclude_id {
        if exclude == parent_id {
            return Err(AppError::BadRequest("上级部门不能是自身".to_string()));
        }
        // 防环：新父部门不能是当前部门的后代。
        let mut current = Some(parent_id.to_string());
        while let Some(cur) = current {
            if cur == exclude {
                return Err(AppError::BadRequest(
                    "上级部门不能是其子部门（形成循环）".to_string(),
                ));
            }
            // 注意：parent_id 为 NULL 时须以 Option<Option<String>> 解码，
            // 否则 String 解码器对 NULL 单元格报 UnexpectedNull。
            let parent: Option<Option<String>> =
                sqlx::query_scalar::<_, Option<String>>(
                    "SELECT parent_id FROM departments WHERE id = ?",
                )
                .bind(&cur)
                .fetch_optional(pool)
                .await?;
            current = parent.flatten();
        }
    }
    Ok(())
}

/// 部门列表（含负责人姓名与成员数；前端自行构建树）。
pub async fn list_departments(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "department:list")?;

    let rows: Vec<DepartmentListRow> = sqlx::query_as(
        "SELECT d.id, d.name, d.parent_id, d.sort_order, \
         (SELECT GROUP_CONCAT(e.name ORDER BY e.name SEPARATOR '、') \
          FROM department_leaders dl JOIN employees e ON e.id = dl.employee_id \
          WHERE dl.department_id = d.id) AS leader_names, \
         (SELECT GROUP_CONCAT(r.name ORDER BY r.created_at SEPARATOR '、') \
          FROM department_roles dr JOIN roles r ON r.id = dr.role_id \
          WHERE dr.department_id = d.id) AS role_names, \
         (SELECT COUNT(*) FROM employee_departments ed WHERE ed.department_id = d.id) AS member_count \
         FROM departments d \
         ORDER BY d.sort_order, d.created_at",
    )
    .fetch_all(&state.pool)
    .await?;

    // 负责人 id 列表（供前端编辑回显）。
    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id": row.id,
                "name": row.name,
                "parent_id": row.parent_id,
                "leader_names": row.leader_names,
                "role_names": row.role_names,
                "member_count": row.member_count,
                "sort_order": row.sort_order,
            })
        })
        .collect();
    let ids: Vec<String> = items
        .iter()
        .map(|it| it["id"].as_str().unwrap_or_default().to_string())
        .collect();

    // 批量查负责人 id：一次查询按部门分组。
    let mut leader_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    if !ids.is_empty() {
        let placeholders = vec!["?"; ids.len()].join(",");
        let sql = format!(
            "SELECT department_id, employee_id FROM department_leaders WHERE department_id IN ({})",
            placeholders
        );
        let mut q = sqlx::query_as::<_, (String, String)>(&sql);
        for id in &ids {
            q = q.bind(id);
        }
        let pairs: Vec<(String, String)> = q.fetch_all(&state.pool).await?;
        for (dept_id, emp_id) in pairs {
            leader_map.entry(dept_id).or_default().push(emp_id);
        }
    }

    let items: Vec<serde_json::Value> = items
        .into_iter()
        .map(|mut it| {
            let id = it["id"].as_str().unwrap_or_default().to_string();
            it["leader_ids"] =
                serde_json::Value::Array(leader_map.get(&id).cloned().unwrap_or_default()
                    .into_iter().map(serde_json::Value::String).collect());
            it
        })
        .collect();

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "items": items,
        "total": items.len(),
    }))))
}

/// 部门成员列表（含负责人标记）。
pub async fn list_department_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "department:view")?;

    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }

    let members: Vec<DepartmentMemberRow> = sqlx::query_as(
        "SELECT e.id, e.username, e.name, e.title, e.avatar, e.status, \
         CASE WHEN dl.employee_id IS NOT NULL THEN 1 ELSE 0 END AS is_leader \
         FROM employee_departments ed \
         INNER JOIN employees e ON e.id = ed.employee_id \
         LEFT JOIN department_leaders dl ON dl.department_id = ed.department_id AND dl.employee_id = e.id \
         WHERE ed.department_id = ? \
         ORDER BY is_leader DESC, e.created_at",
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "items": members,
        "total": members.len(),
    }))))
}

/// 创建部门。
pub async fn create_department(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Json(body): Json<NewDepartment>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "department:create")?;

    let ip = client_ip.0;
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::ValidationError("部门名称不能为空".to_string()));
    }
    if name.chars().count() > 64 {
        return Err(AppError::ValidationError(
            "部门名称不能超过 64 个字符".to_string(),
        ));
    }

    if let Some(ref parent) = body.parent_id {
        validate_parent(&state.pool, parent, None).await?;
    }
    let leaders = validate_leaders(&state.pool, &body.leader_ids).await?;

    let id = Uuid::new_v4().to_string();

    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO departments (id, name, parent_id, sort_order) VALUES (?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(&body.parent_id)
    .bind(body.sort_order.unwrap_or(0))
    .execute(&mut *tx)
    .await?;

    for lid in &leaders {
        sqlx::query(
            "INSERT INTO department_leaders (department_id, employee_id) VALUES (?, ?)",
        )
        .bind(&id)
        .bind(lid)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    append_log(
        &state.config.log_file,
        &format!("User {} created department {}", user_tag(&auth.name, &auth.username), name),
        &ip,
    );

    Ok(Json(ApiResponse::created(serde_json::json!({
        "id": id,
    }))))
}

/// 更新部门。
pub async fn update_department(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateDepartment>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "department:edit")?;

    let ip = client_ip.0;
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }

    if let Some(ref name) = body.name {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::ValidationError("部门名称不能为空".to_string()));
        }
        if name.chars().count() > 64 {
            return Err(AppError::ValidationError(
                "部门名称不能超过 64 个字符".to_string(),
            ));
        }
    }

    if let Some(ref parent) = body.parent_id {
        if let Some(pid) = parent {
            validate_parent(&state.pool, pid, Some(&id)).await?;
        }
    }
    let leaders = match &body.leader_ids {
        Some(ids) => Some(validate_leaders(&state.pool, ids).await?),
        None => None,
    };

    let mut tx = state.pool.begin().await?;

    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> =
        sqlx::QueryBuilder::new("UPDATE departments SET ");
    let mut has_fields = false;

    if let Some(name) = &body.name {
        qb.push("name = ").push_bind(name.trim());
        has_fields = true;
    }
    if let Some(parent) = &body.parent_id {
        if has_fields {
            qb.push(", ");
        }
        qb.push("parent_id = ").push_bind(parent);
        has_fields = true;
    }
    if let Some(order) = body.sort_order {
        if has_fields {
            qb.push(", ");
        }
        qb.push("sort_order = ").push_bind(order);
        has_fields = true;
    }

    if has_fields {
        qb.push(" WHERE id = ").push_bind(&id);
        qb.build().execute(&mut *tx).await?;
    }

    // 负责人整体替换。
    if let Some(leaders) = leaders {
        sqlx::query("DELETE FROM department_leaders WHERE department_id = ?")
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        for lid in &leaders {
            sqlx::query(
                "INSERT INTO department_leaders (department_id, employee_id) VALUES (?, ?)",
            )
            .bind(&id)
            .bind(lid)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    // 部门结构变化影响 subtree 数据范围，全体员工 perm_version 递增（部门编辑低频，代价可接受）。
    sqlx::query("UPDATE employees SET perm_version = perm_version + 1")
        .execute(&state.pool)
        .await?;

    append_log(
        &state.config.log_file,
        &format!("User {} updated department {}", user_tag(&auth.name, &auth.username), id),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("ok")))
}

/// 删除部门：存在子部门时拒绝（需先删子部门）。
pub async fn delete_department(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "department:delete")?;

    let ip = client_ip.0;

    let child_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE parent_id = ?")
            .bind(&id)
            .fetch_one(&state.pool)
            .await?;
    if child_count > 0 {
        return Err(AppError::BadRequest(
            "该部门存在子部门，请先删除子部门".to_string(),
        ));
    }

    let result = sqlx::query("DELETE FROM departments WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    // 负责人与成员关联由外键级联清理。
    // 部门删除影响 subtree 数据范围，全体员工 perm_version 递增。
    sqlx::query("UPDATE employees SET perm_version = perm_version + 1")
        .execute(&state.pool)
        .await?;

    append_log(
        &state.config.log_file,
        &format!("User {} deleted department {}", user_tag(&auth.name, &auth.username), id),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("ok")))
}

/// 更新员工的归属部门（多对多整体替换）。
pub async fn update_employee_departments(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(employee_id): Path<String>,
    Json(body): Json<UpdateEmployeeDepartmentsRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "employee:edit")?;

    let ip = client_ip.0;

    if employee_id == auth.id {
        return Err(AppError::BadRequest("不能修改自己的归属部门".to_string()));
    }

    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM employees WHERE id = ?")
        .bind(&employee_id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }

    // 校验部门 ID 去重且都存在。
    let mut dept_ids: Vec<String> = body.department_ids;
    dept_ids.sort();
    dept_ids.dedup();
    for did in &dept_ids {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE id = ?")
            .bind(did)
            .fetch_one(&state.pool)
            .await?;
        if n == 0 {
            return Err(AppError::BadRequest("指定的部门不存在".to_string()));
        }
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM employee_departments WHERE employee_id = ?")
        .bind(&employee_id)
        .execute(&mut *tx)
        .await?;
    for did in &dept_ids {
        sqlx::query("INSERT INTO employee_departments (employee_id, department_id) VALUES (?, ?)")
            .bind(&employee_id)
            .bind(did)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    // 部门归属变更影响该员工的部门角色继承与数据范围，递增 perm_version 即时生效。
    sqlx::query("UPDATE employees SET perm_version = perm_version + 1 WHERE id = ?")
        .bind(&employee_id)
        .execute(&state.pool)
        .await?;

    append_log(
        &state.config.log_file,
        &format!(
            "User {} updated department membership of employee {}",
            user_tag(&auth.name, &auth.username),
            employee_id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("ok")))
}
