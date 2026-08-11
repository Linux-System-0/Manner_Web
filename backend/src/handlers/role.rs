//! 角色管理（方案 C：RBAC + 数据范围 + 部门角色继承）。
//!
//! 权限校验：
//! - 角色 CRUD / 部门角色绑定 / 员工角色分配均要求 `role:manage`；
//! - 防提权：操作者只能管理「权限集 ⊆ 自己有效权限、且数据范围 ≤ 自己范围」的角色；
//! - 父子角色：子角色范围不得大于父角色范围；沿 parent_id 防环；
//! - `super_admin`（is_system=1）：不可删除/改名/改权限/改范围，不可经部门角色绑定，
//!   且至少保留一名持有者；
//! - 权限/角色相关变更后批量递增受影响员工的 `perm_version`，实现即时生效。

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::error::AppError;
use crate::handlers::auth::{append_log, user_tag, ClientIp};
use crate::middleware::auth::{require_permission, AppState, AuthUser};
use crate::models::role::{
    CreateRoleRequest, Role, RoleListItem, UpdateDepartmentRolesRequest, UpdateEmployeeRolesRequest,
    UpdateRoleRequest,
};
use crate::services::permission::{self, Grant};
use crate::utils::response::ApiResponse;

const SUPER_ADMIN_ROLE_ID: &str = "00000000-0000-0000-0000-000000000001";

fn valid_scope_type(s: &str) -> bool {
    matches!(s, "all" | "subtree" | "department" | "self" | "custom")
}

async fn get_role_opt(pool: &sqlx::MySqlPool, id: &str) -> Result<Option<Role>, sqlx::Error> {
    sqlx::query_as::<_, Role>("SELECT * FROM roles WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

async fn roles_exist(pool: &sqlx::MySqlPool, ids: &[String]) -> Result<bool, sqlx::Error> {
    if ids.is_empty() {
        return Ok(true);
    }
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!("SELECT COUNT(*) FROM roles WHERE id IN ({})", placeholders);
    let mut q = sqlx::query_scalar::<_, i64>(&sql);
    for id in ids {
        q = q.bind(id);
    }
    let n: i64 = q.fetch_one(pool).await?;
    Ok(n as usize == ids.len())
}

async fn permissions_exist(pool: &sqlx::MySqlPool, codes: &[String]) -> Result<bool, sqlx::Error> {
    if codes.is_empty() {
        return Ok(true);
    }
    let placeholders = vec!["?"; codes.len()].join(",");
    let sql = format!("SELECT COUNT(*) FROM permissions WHERE code IN ({})", placeholders);
    let mut q = sqlx::query_scalar::<_, i64>(&sql);
    for c in codes {
        q = q.bind(c);
    }
    let n: i64 = q.fetch_one(pool).await?;
    Ok(n as usize == codes.len())
}

async fn departments_exist(pool: &sqlx::MySqlPool, ids: &[String]) -> Result<bool, sqlx::Error> {
    if ids.is_empty() {
        return Ok(true);
    }
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!("SELECT COUNT(*) FROM departments WHERE id IN ({})", placeholders);
    let mut q = sqlx::query_scalar::<_, i64>(&sql);
    for id in ids {
        q = q.bind(id);
    }
    let n: i64 = q.fetch_one(pool).await?;
    Ok(n as usize == ids.len())
}

/// 沿 parent_id 从 parent 向上走，若遇到 self_id 则成环。
async fn would_create_cycle(
    pool: &sqlx::MySqlPool,
    parent_id: &str,
    self_id: &str,
) -> Result<bool, sqlx::Error> {
    let mut cur = Some(parent_id.to_string());
    while let Some(c) = cur {
        if c == self_id {
            return Ok(true);
        }
        // parent_id 为 NULL（根角色）时以 Option<Option<String>> 解码，避免 UnexpectedNull。
        let parent: Option<Option<String>> =
            sqlx::query_scalar::<_, Option<String>>("SELECT parent_id FROM roles WHERE id = ?")
                .bind(&c)
                .fetch_optional(pool)
                .await?;
        cur = parent.flatten();
    }
    Ok(false)
}

async fn role_custom_depts(pool: &sqlx::MySqlPool, role_id: &str) -> Result<Vec<String>, sqlx::Error> {
    let mut depts: Vec<String> =
        sqlx::query_scalar("SELECT department_id FROM role_department_scopes WHERE role_id = ?")
            .bind(role_id)
            .fetch_all(pool)
            .await?;
    depts.sort();
    depts.dedup();
    Ok(depts)
}

/// 角色有效授权 = 自身 + 沿 parent_id 祖先的全部授权（每个节点权限携带节点自身范围）。
async fn role_effective_grants(pool: &sqlx::MySqlPool, role_id: &str) -> Result<Vec<Grant>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "WITH RECURSIVE chain AS (
             SELECT id FROM roles WHERE id = ?
             UNION ALL
             SELECT r.parent_id FROM roles r JOIN chain c ON r.id = c.id
         )
         SELECT DISTINCT c.id AS role_id, p.code
         FROM chain c
         JOIN roles r ON r.id = c.id
         JOIN role_permissions rp ON rp.role_id = c.id
         JOIN permissions p ON p.id = rp.permission_id",
    )
    .bind(role_id)
    .fetch_all(pool)
    .await?;

    let role_ids: Vec<&str> = {
        let mut ids: Vec<&str> = rows.iter().map(|(rid, _)| rid.as_str()).collect();
        ids.sort();
        ids.dedup();
        ids
    };
    let mut dept_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    if !role_ids.is_empty() {
        let placeholders = vec!["?"; role_ids.len()].join(",");
        let sql = format!(
            "SELECT role_id, department_id FROM role_department_scopes WHERE role_id IN ({})",
            placeholders
        );
        let mut q = sqlx::query_as::<_, (String, String)>(&sql);
        for rid in &role_ids {
            q = q.bind(rid);
        }
        let dept_rows: Vec<(String, String)> = q.fetch_all(pool).await?;
        for (rid, did) in dept_rows {
            dept_map.entry(rid).or_default().push(did);
        }
    }

    let mut grants: Vec<Grant> = Vec::new();
    for (rid, code) in rows {
        let scope_type: String = sqlx::query_scalar("SELECT scope_type FROM roles WHERE id = ?")
            .bind(&rid)
            .fetch_one(pool)
            .await?;
        let mut depts = match scope_type.as_str() {
            "custom" => dept_map.get(&rid).cloned().unwrap_or_default(),
            _ => Vec::new(),
        };
        depts.sort();
        depts.dedup();
        grants.push(Grant {
            code,
            scope_type,
            scope_department_ids: depts,
        });
    }
    grants.sort_by(|a, b| a.code.cmp(&b.code));
    Ok(grants)
}

/// 防提权（码级 + 范围）：操作者能授予/维护这些码、且范围能覆盖指定范围。
async fn check_codes_within_operator(
    pool: &sqlx::MySqlPool,
    operator: &AuthUser,
    codes: &[String],
    scope_type: &str,
    custom_depts: &[String],
) -> Result<bool, sqlx::Error> {
    for code in codes {
        if !permission::has_permission(&operator.grants, code) {
            return Ok(false);
        }
        if permission::is_data_scoped_code(code) {
            let Some(op_scope) =
                permission::build_scope(pool, &operator.grants, code, &operator.id).await?
            else {
                return Ok(false);
            };
            if !permission::scope_covers(&op_scope, scope_type, custom_depts) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// 防提权：操作者能否管理该角色（角色的有效授权 ⊆ 操作者授权）。
async fn check_role_within_operator(
    pool: &sqlx::MySqlPool,
    operator: &AuthUser,
    role_id: &str,
) -> Result<bool, sqlx::Error> {
    let grants = role_effective_grants(pool, role_id).await?;
    if grants.is_empty() {
        return Ok(true);
    }
    for g in &grants {
        if !permission::has_permission(&operator.grants, &g.code) {
            return Ok(false);
        }
        if permission::is_data_scoped_code(&g.code) {
            let Some(op_scope) =
                permission::build_scope(pool, &operator.grants, &g.code, &operator.id).await?
            else {
                return Ok(false);
            };
            if !permission::scope_covers(&op_scope, &g.scope_type, &g.scope_department_ids) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// 父子角色范围约束：子角色范围不得大于父角色。
fn role_scope_within(
    parent_scope_type: &str,
    parent_custom_depts: &[String],
    child_scope_type: &str,
    child_custom_depts: &[String],
) -> bool {
    let level_ok = match parent_scope_type {
        "all" => true,
        "subtree" => matches!(child_scope_type, "subtree" | "department" | "self" | "custom"),
        "department" => matches!(child_scope_type, "department" | "self" | "custom"),
        "self" => matches!(child_scope_type, "self"),
        "custom" => matches!(child_scope_type, "self" | "custom"),
        _ => false,
    };
    if !level_ok {
        return false;
    }
    if child_scope_type == "custom" && parent_scope_type == "custom" {
        return child_custom_depts
            .iter()
            .all(|d| parent_custom_depts.contains(d));
    }
    true
}

/// 受影响员工：持有该角色（直接/经部门）或持有其子孙角色（经 parent 继承）的全部员工。
async fn affected_employee_ids(
    pool: &sqlx::MySqlPool,
    role_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "WITH RECURSIVE desc_roles AS (
             SELECT id FROM roles WHERE parent_id = ?
             UNION ALL
             SELECT r.id FROM roles r JOIN desc_roles d ON r.parent_id = d.id
         )
         SELECT employee_id FROM employee_roles WHERE role_id = ?
         UNION
         SELECT ed.employee_id FROM department_roles dr
         JOIN employee_departments ed ON ed.department_id = dr.department_id
         WHERE dr.role_id = ?
         UNION
         SELECT er.employee_id FROM desc_roles d JOIN employee_roles er ON er.role_id = d.id
         UNION
         SELECT ed.employee_id FROM desc_roles d
         JOIN department_roles dr ON dr.role_id = d.id
         JOIN employee_departments ed ON ed.department_id = dr.department_id",
    )
    .bind(role_id)
    .bind(role_id)
    .bind(role_id)
    .fetch_all(pool)
    .await
}

async fn bump_perm_version(pool: &sqlx::MySqlPool, ids: &[String]) -> Result<(), sqlx::Error> {
    if ids.is_empty() {
        return Ok(());
    }
    let placeholders = vec!["?"; ids.len()].join(",");
    let sql = format!(
        "UPDATE employees SET perm_version = perm_version + 1 WHERE id IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&sql);
    for id in ids {
        q = q.bind(id);
    }
    q.execute(pool).await?;
    Ok(())
}

async fn bump_department_employees(
    pool: &sqlx::MySqlPool,
    dept_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE employees SET perm_version = perm_version + 1 WHERE id IN \
         (SELECT employee_id FROM employee_departments WHERE department_id = ?)",
    )
    .bind(dept_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn count_super_admin_holders(pool: &sqlx::MySqlPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM employee_roles WHERE role_id = ?")
        .bind(SUPER_ADMIN_ROLE_ID)
        .fetch_one(pool)
        .await
}

async fn replace_role_permissions<'e>(
    tx: &mut sqlx::Transaction<'e, sqlx::MySql>,
    role_id: &str,
    codes: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM role_permissions WHERE role_id = ?")
        .bind(role_id)
        .execute(&mut **tx)
        .await?;
    for code in codes {
        let pid: Option<i32> = sqlx::query_scalar("SELECT id FROM permissions WHERE code = ?")
            .bind(code)
            .fetch_optional(&mut **tx)
            .await?;
        if let Some(pid) = pid {
            sqlx::query("INSERT INTO role_permissions (role_id, permission_id) VALUES (?, ?)")
                .bind(role_id)
                .bind(pid)
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}

async fn replace_role_scopes<'e>(
    tx: &mut sqlx::Transaction<'e, sqlx::MySql>,
    role_id: &str,
    dept_ids: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM role_department_scopes WHERE role_id = ?")
        .bind(role_id)
        .execute(&mut **tx)
        .await?;
    for did in dept_ids {
        sqlx::query(
            "INSERT IGNORE INTO role_department_scopes (role_id, department_id) VALUES (?, ?)",
        )
        .bind(role_id)
        .bind(did)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/roles —— 角色列表（含权限码、范围、成员数）。
pub async fn list_roles(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "role:manage")?;

    let roles: Vec<Role> = sqlx::query_as("SELECT * FROM roles ORDER BY is_system DESC, created_at")
        .fetch_all(&state.pool)
        .await?;

    if roles.is_empty() {
        return Ok(Json(ApiResponse::ok(serde_json::json!({
            "items": [],
            "total": 0,
        }))));
    }

    let role_ids: Vec<String> = roles.iter().map(|r| r.id.clone()).collect();
    let placeholders = vec!["?"; role_ids.len()].join(",");

    // 批量权限码
    let mut perm_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    {
        let sql = format!(
            "SELECT rp.role_id, p.code FROM role_permissions rp \
             JOIN permissions p ON p.id = rp.permission_id WHERE rp.role_id IN ({})",
            placeholders
        );
        let mut q = sqlx::query_as::<_, (String, String)>(&sql);
        for id in &role_ids {
            q = q.bind(id);
        }
        let rows: Vec<(String, String)> = q.fetch_all(&state.pool).await?;
        for (rid, code) in rows {
            perm_map.entry(rid).or_default().push(code);
        }
    }
    // 批量 custom 范围部门
    let mut scope_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    {
        let sql = format!(
            "SELECT role_id, department_id FROM role_department_scopes WHERE role_id IN ({})",
            placeholders
        );
        let mut q = sqlx::query_as::<_, (String, String)>(&sql);
        for id in &role_ids {
            q = q.bind(id);
        }
        let rows: Vec<(String, String)> = q.fetch_all(&state.pool).await?;
        for (rid, did) in rows {
            scope_map.entry(rid).or_default().push(did);
        }
    }
    // 批量成员数（直接分配 + 经部门）
    let mut count_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    {
        let sql = format!(
            "SELECT t.role_id, COUNT(*) FROM ( \
             SELECT er.role_id FROM employee_roles er \
             UNION ALL \
             SELECT dr.role_id FROM department_roles dr JOIN employee_departments ed ON ed.department_id = dr.department_id \
             ) t WHERE t.role_id IN ({}) GROUP BY t.role_id",
            placeholders
        );
        let mut q = sqlx::query_as::<_, (String, i64)>(&sql);
        for id in &role_ids {
            q = q.bind(id);
        }
        let rows: Vec<(String, i64)> = q.fetch_all(&state.pool).await?;
        for (rid, n) in rows {
            count_map.insert(rid, n);
        }
    }

    let name_map: std::collections::HashMap<String, String> = roles
        .iter()
        .map(|r| (r.id.clone(), r.name.clone()))
        .collect();

    let items: Vec<RoleListItem> = roles
        .into_iter()
        .map(|r| {
            let mut codes = perm_map.get(&r.id).cloned().unwrap_or_default();
            codes.sort();
            codes.dedup();
            let mut depts = scope_map.get(&r.id).cloned().unwrap_or_default();
            depts.sort();
            depts.dedup();
            RoleListItem {
                parent_name: r.parent_id.as_ref().and_then(|pid| name_map.get(pid).cloned()),
                permission_codes: codes,
                scope_department_ids: depts,
                member_count: count_map.get(&r.id).copied().unwrap_or(0),
                id: r.id,
                name: r.name,
                parent_id: r.parent_id,
                is_system: r.is_system,
                scope_type: r.scope_type,
                description: r.description,
                created_at: r.created_at,
            }
        })
        .collect();

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "items": items,
        "total": items.len(),
    }))))
}

/// POST /api/roles —— 创建角色。
pub async fn create_role(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Json(body): Json<CreateRoleRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "role:manage")?;
    let ip = client_ip.0;

    let name = body.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(AppError::ValidationError(
            "角色名称不能为空且不超过 64 个字符".to_string(),
        ));
    }
    if !valid_scope_type(&body.scope_type) {
        return Err(AppError::ValidationError("非法数据范围类型".to_string()));
    }
    if body.scope_type == "custom" && body.scope_department_ids.is_empty() {
        return Err(AppError::ValidationError(
            "custom 范围必须指定至少一个部门".to_string(),
        ));
    }

    let name_exist: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles WHERE name = ?")
        .bind(&name)
        .fetch_one(&state.pool)
        .await?;
    if name_exist > 0 {
        return Err(AppError::Conflict);
    }

    let parent_id = body.parent_id.filter(|s| !s.trim().is_empty());
    if let Some(ref pid) = parent_id {
        if get_role_opt(&state.pool, pid).await?.is_none() {
            return Err(AppError::BadRequest("指定的父角色不存在".to_string()));
        }
    }

    if !permissions_exist(&state.pool, &body.permission_codes).await? {
        return Err(AppError::BadRequest("包含无效的权限码".to_string()));
    }
    if !departments_exist(&state.pool, &body.scope_department_ids).await? {
        return Err(AppError::BadRequest("包含无效的部门".to_string()));
    }

    // 父角色范围约束 + 汇总有效权限码（自身 ∪ 父角色）
    let mut effective_codes: Vec<String> = body.permission_codes.clone();
    if let Some(ref pid) = parent_id {
        let parent = get_role_opt(&state.pool, pid).await?.unwrap();
        let parent_custom = role_custom_depts(&state.pool, pid).await?;
        if !role_scope_within(
            &parent.scope_type,
            &parent_custom,
            &body.scope_type,
            &body.scope_department_ids,
        ) {
            return Err(AppError::BadRequest(
                "子角色数据范围不能大于父角色数据范围".to_string(),
            ));
        }
        let parent_grants = role_effective_grants(&state.pool, pid).await?;
        effective_codes.extend(parent_grants.iter().map(|g| g.code.clone()));
        effective_codes.sort();
        effective_codes.dedup();
    }

    // 防提权：操作者必须能覆盖该角色全部有效授权
    if !check_codes_within_operator(
        &state.pool,
        &auth,
        &effective_codes,
        &body.scope_type,
        &body.scope_department_ids,
    )
    .await?
    {
        return Err(AppError::Forbidden);
    }

    let id = Uuid::new_v4().to_string();
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO roles (id, name, parent_id, is_system, scope_type, description) \
         VALUES (?, ?, ?, 0, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&parent_id)
    .bind(&body.scope_type)
    .bind(body.description.as_deref())
    .execute(&mut *tx)
    .await?;

    replace_role_permissions(&mut tx, &id, &body.permission_codes).await?;
    if body.scope_type == "custom" {
        replace_role_scopes(&mut tx, &id, &body.scope_department_ids).await?;
    }
    tx.commit().await?;

    append_log(
        &state.config.log_file,
        &format!(
            "用户 {} 创建了角色 {}",
            user_tag(&auth.name, &auth.username),
            name
        ),
        &ip,
    );

    Ok(Json(ApiResponse::created(serde_json::json!({ "id": id }))))
}

/// PUT /api/roles/:id —— 更新角色（名称/描述/父角色/范围/权限码）。
pub async fn update_role(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateRoleRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "role:manage")?;
    let ip = client_ip.0;

    let role = get_role_opt(&state.pool, &id).await?.ok_or(AppError::NotFound)?;

    // is_system 角色（super_admin）仅允许改描述。
    if role.is_system == 1 {
        let only_desc = body.name.is_none()
            && body.parent_id.is_none()
            && body.scope_type.is_none()
            && body.permission_codes.is_none()
            && body.scope_department_ids.is_none();
        if !only_desc {
            return Err(AppError::BadRequest(
                "系统内置角色不可修改名称/继承/范围/权限".to_string(),
            ));
        }
        if let Some(Some(desc)) = &body.description {
            if desc.len() > 255 {
                return Err(AppError::ValidationError(
                    "描述不能超过 255 个字符".to_string(),
                ));
            }
        }
        sqlx::query("UPDATE roles SET description = ? WHERE id = ?")
            .bind(body.description.and_then(|d| d))
            .bind(&id)
            .execute(&state.pool)
            .await?;
        return Ok(Json(ApiResponse::ok_msg("ok")));
    }

    let name = match &body.name {
        Some(n) => {
            let n = n.trim().to_string();
            if n.is_empty() || n.chars().count() > 64 {
                return Err(AppError::ValidationError(
                    "角色名称不能为空且不超过 64 个字符".to_string(),
                ));
            }
            let dup: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM roles WHERE name = ? AND id != ?")
                    .bind(&n)
                    .bind(&id)
                    .fetch_one(&state.pool)
                    .await?;
            if dup > 0 {
                return Err(AppError::Conflict);
            }
            Some(n)
        }
        None => None,
    };

    let scope_type = match &body.scope_type {
        Some(s) => {
            if !valid_scope_type(s) {
                return Err(AppError::ValidationError("非法数据范围类型".to_string()));
            }
            if s == "custom" {
                let depts = body.scope_department_ids.as_deref().unwrap_or_default();
                if depts.is_empty() {
                    return Err(AppError::ValidationError(
                        "custom 范围必须指定至少一个部门".to_string(),
                    ));
                }
            }
            Some(s.clone())
        }
        None => None,
    };

    // 父角色解析：body 指定则用指定值（None 清空），否则沿用现有。
    let parent_id = match &body.parent_id {
        Some(Some(pid)) if !pid.trim().is_empty() => {
            if pid == &id {
                return Err(AppError::BadRequest("父角色不能是自身".to_string()));
            }
            if get_role_opt(&state.pool, pid).await?.is_none() {
                return Err(AppError::BadRequest("指定的父角色不存在".to_string()));
            }
            if would_create_cycle(&state.pool, pid, &id).await? {
                return Err(AppError::BadRequest("父角色形成循环继承".to_string()));
            }
            Some(pid.clone())
        }
        Some(Some(_)) => None,
        Some(None) => None,
        None => role.parent_id.clone(),
    };

    // 现有效率授权（用于结果集防提权校验）。
    let existing_custom = role_custom_depts(&state.pool, &id).await?;
    let existing_codes: Vec<String> = sqlx::query_scalar(
        "SELECT p.code FROM role_permissions rp JOIN permissions p ON p.id = rp.permission_id \
         WHERE rp.role_id = ?",
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;

    if let Some(ref pid) = parent_id {
        if pid == &id {
            return Err(AppError::BadRequest("父角色不能是自身".to_string()));
        }
        if would_create_cycle(&state.pool, pid, &id).await? {
            return Err(AppError::BadRequest("父角色形成循环继承".to_string()));
        }
        let parent = get_role_opt(&state.pool, pid).await?.unwrap();
        let parent_custom = role_custom_depts(&state.pool, pid).await?;
        let child_scope = scope_type.clone().unwrap_or(role.scope_type.clone());
        let child_custom = body
            .scope_department_ids
            .clone()
            .unwrap_or(existing_custom.clone());
        if !role_scope_within(&parent.scope_type, &parent_custom, &child_scope, &child_custom) {
            return Err(AppError::BadRequest(
                "子角色数据范围不能大于父角色数据范围".to_string(),
            ));
        }
    }

    let new_codes = match &body.permission_codes {
        Some(codes) => {
            if !permissions_exist(&state.pool, codes).await? {
                return Err(AppError::BadRequest("包含无效的权限码".to_string()));
            }
            Some(codes.clone())
        }
        None => None,
    };
    let new_depts = match &body.scope_department_ids {
        Some(depts) => {
            if !departments_exist(&state.pool, depts).await? {
                return Err(AppError::BadRequest("包含无效的部门".to_string()));
            }
            Some(depts.clone())
        }
        None => None,
    };

    // 防提权：结果集（自身权限 ∪ 父角色权限）须在操作者授权内。
    let result_scope = scope_type.clone().unwrap_or(role.scope_type.clone());
    let result_custom = new_depts.clone().unwrap_or(existing_custom.clone());
    let mut result_codes = new_codes.clone().unwrap_or(existing_codes.clone());
    if let Some(ref pid) = parent_id {
        let parent_grants = role_effective_grants(&state.pool, pid).await?;
        result_codes.extend(parent_grants.iter().map(|g| g.code.clone()));
        result_codes.sort();
        result_codes.dedup();
    }
    if !check_codes_within_operator(&state.pool, &auth, &result_codes, &result_scope, &result_custom)
        .await?
    {
        return Err(AppError::Forbidden);
    }

    let affected = affected_employee_ids(&state.pool, &id).await?;

    let final_scope = scope_type.clone().unwrap_or(role.scope_type.clone());
    let mut tx = state.pool.begin().await?;
    if let Some(name) = name {
        sqlx::query("UPDATE roles SET name = ? WHERE id = ?")
            .bind(name)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(desc_opt) = body.description {
        let desc = desc_opt;
        if desc.as_ref().map_or(false, |d| d.len() > 255) {
            return Err(AppError::ValidationError(
                "描述不能超过 255 个字符".to_string(),
            ));
        }
        sqlx::query("UPDATE roles SET description = ? WHERE id = ?")
            .bind(desc)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE roles SET parent_id = ?, scope_type = ? WHERE id = ?")
        .bind(parent_id)
        .bind(final_scope.clone())
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    if let Some(codes) = new_codes {
        replace_role_permissions(&mut tx, &id, &codes).await?;
    }
    if final_scope == "custom" {
        replace_role_scopes(&mut tx, &id, &new_depts.unwrap_or(existing_custom)).await?;
    } else {
        sqlx::query("DELETE FROM role_department_scopes WHERE role_id = ?")
            .bind(&id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    bump_perm_version(&state.pool, &affected).await?;

    append_log(
        &state.config.log_file,
        &format!(
            "用户 {} 更新了角色 {}",
            user_tag(&auth.name, &auth.username),
            id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("ok")))
}

/// DELETE /api/roles/:id —— 删除角色。
pub async fn delete_role(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "role:manage")?;
    let ip = client_ip.0;

    let role = get_role_opt(&state.pool, &id).await?.ok_or(AppError::NotFound)?;
    if role.is_system == 1 {
        return Err(AppError::BadRequest("系统内置角色不可删除".to_string()));
    }
    if !check_role_within_operator(&state.pool, &auth, &id).await? {
        return Err(AppError::Forbidden);
    }

    let child_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM roles WHERE parent_id = ?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;
    if child_count > 0 {
        return Err(AppError::BadRequest(
            "该角色存在子角色，请先处理子角色继承".to_string(),
        ));
    }

    let affected = affected_employee_ids(&state.pool, &id).await?;

    let result = sqlx::query("DELETE FROM roles WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    bump_perm_version(&state.pool, &affected).await?;

    append_log(
        &state.config.log_file,
        &format!("用户 {} 删除了角色 {}", user_tag(&auth.name, &auth.username), id),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("ok")))
}

/// PUT /api/employees/:id/roles —— 整体替换员工分配的角色。
pub async fn update_employee_roles(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(employee_id): Path<String>,
    Json(body): Json<UpdateEmployeeRolesRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "role:manage")?;
    let ip = client_ip.0;

    if employee_id == auth.id {
        return Err(AppError::BadRequest("不能修改自己的角色".to_string()));
    }

    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM employees WHERE id = ?")
        .bind(&employee_id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }

    let mut role_ids: Vec<String> = body.role_ids.clone();
    role_ids.sort();
    role_ids.dedup();
    if !roles_exist(&state.pool, &role_ids).await? {
        return Err(AppError::BadRequest("包含无效的角色".to_string()));
    }

    for rid in &role_ids {
        if !check_role_within_operator(&state.pool, &auth, rid).await? {
            return Err(AppError::Forbidden);
        }
    }

    // 超级管理员最后一名保护。
    let old_ids: Vec<String> = sqlx::query_scalar(
        "SELECT role_id FROM employee_roles WHERE employee_id = ?",
    )
    .bind(&employee_id)
    .fetch_all(&state.pool)
    .await?;
    if old_ids.contains(&SUPER_ADMIN_ROLE_ID.to_string())
        && !role_ids.contains(&SUPER_ADMIN_ROLE_ID.to_string())
    {
        let total: i64 = count_super_admin_holders(&state.pool).await?;
        if total <= 1 {
            return Err(AppError::BadRequest(
                "系统至少需要一名超级管理员，无法移除".to_string(),
            ));
        }
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM employee_roles WHERE employee_id = ?")
        .bind(&employee_id)
        .execute(&mut *tx)
        .await?;
    for rid in &role_ids {
        sqlx::query("INSERT IGNORE INTO employee_roles (employee_id, role_id) VALUES (?, ?)")
            .bind(&employee_id)
            .bind(rid)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    bump_perm_version(&state.pool, &[employee_id.clone()]).await?;

    append_log(
        &state.config.log_file,
        &format!(
            "用户 {} 更新了员工 {} 的角色",
            user_tag(&auth.name, &auth.username),
            employee_id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("ok")))
}

/// PUT /api/departments/:id/roles —— 整体替换部门绑定的角色。
pub async fn update_department_roles(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(department_id): Path<String>,
    Json(body): Json<UpdateDepartmentRolesRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "role:manage")?;
    let ip = client_ip.0;

    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE id = ?")
        .bind(&department_id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }

    let mut role_ids: Vec<String> = body.role_ids.clone();
    role_ids.sort();
    role_ids.dedup();
    if role_ids.iter().any(|r| r == SUPER_ADMIN_ROLE_ID) {
        return Err(AppError::BadRequest(
            "超级管理员角色不允许通过部门绑定".to_string(),
        ));
    }
    if !roles_exist(&state.pool, &role_ids).await? {
        return Err(AppError::BadRequest("包含无效的角色".to_string()));
    }
    for rid in &role_ids {
        if !check_role_within_operator(&state.pool, &auth, rid).await? {
            return Err(AppError::Forbidden);
        }
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM department_roles WHERE department_id = ?")
        .bind(&department_id)
        .execute(&mut *tx)
        .await?;
    for rid in &role_ids {
        sqlx::query("INSERT IGNORE INTO department_roles (department_id, role_id) VALUES (?, ?)")
            .bind(&department_id)
            .bind(rid)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    bump_department_employees(&state.pool, &department_id).await?;

    append_log(
        &state.config.log_file,
        &format!(
            "用户 {} 更新了部门 {} 的角色绑定",
            user_tag(&auth.name, &auth.username),
            department_id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("ok")))
}

/// GET /api/departments/:id/roles —— 部门绑定的角色列表。
pub async fn list_department_roles(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(department_id): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "role:manage")?;

    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE id = ?")
        .bind(&department_id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT r.id, r.name FROM department_roles dr JOIN roles r ON r.id = dr.role_id \
         WHERE dr.department_id = ? ORDER BY r.created_at",
    )
    .bind(&department_id)
    .fetch_all(&state.pool)
    .await?;

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(id, name)| serde_json::json!({ "id": id, "name": name }))
        .collect();

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "items": items,
        "total": items.len(),
    }))))
}
