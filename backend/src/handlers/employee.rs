use axum::extract::{Path, Query, State};
use axum::Json;
use sqlx::QueryBuilder;
use uuid::Uuid;

use crate::error::AppError;
use crate::middleware::auth::{require_permission, AppState, AuthUser};
use crate::models::employee::{
    EmployeeDetail, EmployeeListParams, EmployeeListResponse, EmployeeListRow, NewEmployee,
    ResetPasswordRequest, UpdateEmployee, UpdateEmployeePermissionsRequest,
};
use crate::handlers::auth::{append_log, user_tag};
use crate::services::auth::hash_password;
use crate::utils::response::ApiResponse;

pub async fn list_employees(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<EmployeeListParams>,
) -> Result<Json<ApiResponse<EmployeeListResponse>>, AppError> {
    require_permission(&auth.permissions, "employee:list")?;

    // F-24: page 加上限 clamp（10000*100=1e6 偏移，防 (page-1)*page_size 乘法溢出 → 500/panic）
    let page = params.page.unwrap_or(1).max(1).min(10_000);
    let page_size = params.page_size.unwrap_or(20).max(1).min(100);
    let offset = (page - 1) * page_size;

    let mut count_qb: QueryBuilder<'_, sqlx::MySql> =
        QueryBuilder::new("SELECT COUNT(*) FROM employees e WHERE 1=1");
    let mut query_qb: QueryBuilder<'_, sqlx::MySql> = QueryBuilder::new(
        "SELECT e.id, e.username, e.name, e.title, e.email, e.phone, e.id_number, \
         e.address, e.avatar, e.hire_date, e.status, e.protect_block, e.created_at, e.updated_at \
         FROM employees e \
         WHERE 1=1",
    );

    if let Some(ref keyword) = params.keyword {
        let kw = format!("%{}%", keyword);
        count_qb.push(" AND (e.name LIKE ");
        count_qb.push_bind(kw.clone());
        count_qb.push(" OR e.email LIKE ");
        count_qb.push_bind(kw.clone());
        count_qb.push(" OR e.phone LIKE ");
        count_qb.push_bind(kw.clone());
        count_qb.push(")");

        query_qb.push(" AND (e.name LIKE ");
        query_qb.push_bind(kw.clone());
        query_qb.push(" OR e.email LIKE ");
        query_qb.push_bind(kw.clone());
        query_qb.push(" OR e.phone LIKE ");
        query_qb.push_bind(kw);
        query_qb.push(")");
    }

    let total: i64 = count_qb
        .build_query_scalar()
        .fetch_one(&state.pool)
        .await?;

    query_qb.push(" ORDER BY e.created_at DESC LIMIT ");
    query_qb.push_bind(page_size);
    query_qb.push(" OFFSET ");
    query_qb.push_bind(offset);

    let rows: Vec<EmployeeListRow> = query_qb
        .build_query_as()
        .fetch_all(&state.pool)
        .await?;

    Ok(Json(ApiResponse::ok(EmployeeListResponse {
        items: rows,
        total,
        page,
        page_size,
    })))
}

pub async fn get_employee(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<EmployeeDetail>>, AppError> {
    require_permission(&auth.permissions, "employee:view")?;

    let row = sqlx::query_as::<_, EmployeeDetailRow>(
        "SELECT e.id, e.username, e.name, e.title, e.email, e.phone, e.id_number, \
         e.address, e.avatar, e.hire_date, e.status, e.protect_block, e.created_at, e.updated_at \
         FROM employees e \
         WHERE e.id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let permissions: Vec<String> = sqlx::query_scalar(
        "SELECT p.code FROM permissions p
         INNER JOIN employee_permissions ep ON p.id = ep.permission_id
         WHERE ep.employee_id = ?",
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(ApiResponse::ok(EmployeeDetail {
        id: row.id,
        username: row.username,
        name: row.name,
        title: row.title,
        email: row.email,
        phone: row.phone,
        id_number: row.id_number,
        address: row.address,
        avatar: row.avatar,
        hire_date: row.hire_date,
        status: row.status,
        protect_block: row.protect_block,
        permissions,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })))
}

#[derive(sqlx::FromRow)]
struct EmployeeDetailRow {
    pub id: String,
    pub username: String,
    pub name: String,
    pub title: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub id_number: Option<String>,
    pub address: Option<String>,
    pub avatar: Option<String>,
    pub hire_date: Option<chrono::NaiveDate>,
    pub status: i8,
    pub protect_block: i8,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

pub async fn create_employee(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<NewEmployee>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "employee:create")?;

    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM employees WHERE username = ?",
    )
    .bind(&body.username)
    .fetch_one(&state.pool)
    .await?;

    if exists > 0 {
        return Err(AppError::Conflict);
    }

    let id = Uuid::new_v4().to_string();
    // F-02: 生成随机初始密码（不再硬编码 "default"），并标记首次登录强制改密。
    let initial_password = crate::services::auth::generate_random_password();
    let default_password = hash_password(&initial_password, state.config.bcrypt_cost)?;

    sqlx::query(
        "INSERT INTO employees (id, username, password, name, title, email, phone, \
         id_number, address, hire_date, must_change_password) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1)",
    )
    .bind(&id)
    .bind(&body.username)
    .bind(&default_password)
    .bind(&body.name)
    .bind(&body.title)
    .bind(&body.email)
    .bind(&body.phone)
    .bind(&body.id_number)
    .bind(&body.address)
    .bind(&body.hire_date)
    .execute(&state.pool)
    .await?;

    let conv_id = Uuid::new_v4().to_string();
    let _ = sqlx::query(
        "INSERT IGNORE INTO conversations (id, type, name, created_by) VALUES (?, 'single', NULL, ?)",
    )
    .bind(&conv_id)
    .bind(&auth.id)
    .execute(&state.pool)
    .await;

    let _ = sqlx::query(
        "INSERT IGNORE INTO conversation_participants (conversation_id, employee_id, role) VALUES (?, ?, 'member'), (?, ?, 'member')",
    )
    .bind(&conv_id)
    .bind(&auth.id)
    .bind(&conv_id)
    .bind(&id)
    .execute(&state.pool)
    .await?;

    append_log(&state.config.log_file, &format!("用户 {} 创建员工 {} ({})", user_tag(&auth.name, &auth.username), body.name, body.username));

    Ok(Json(ApiResponse::created(serde_json::json!({
        "id": id,
        "username": body.username,
        "name": body.name,
        // F-02: 一次性初始密码，仅在此响应中返回一次，请通过安全渠道转交员工，员工首次登录后必须修改。
        "initial_password": initial_password,
        "must_change_password": true
    }))))
}

pub async fn update_employee(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateEmployee>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "employee:edit")?;

    if id == auth.id {
        let only_avatar = body.name.is_none()
            && body.title.is_none()
            && body.email.is_none()
            && body.phone.is_none()
            && body.id_number.is_none()
            && body.address.is_none()
            && body.hire_date.is_none()
            && body.status.is_none();
        if !only_avatar {
            return Err(AppError::BadRequest("员工管理不能更改自己的资料".to_string()));
        }
    }

    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM employees WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }

    // F-24: 受保护账号（protect_block=1）禁止管理操作，防越权删除/禁用/改密/改权超管。
    let protected: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM employees WHERE id = ? AND protect_block = 1",
    )
    .bind(&id)
    .fetch_one(&state.pool)
    .await?;
    if protected > 0 {
        return Err(AppError::BadRequest("该账号受保护，禁止该操作".to_string()));
    }

    let mut qb: QueryBuilder<'_, sqlx::MySql> =
        QueryBuilder::new("UPDATE employees SET ");

    let mut has_fields = false;

    if let Some(name) = body.name {
        qb.push("name = ");
        qb.push_bind(name);
        has_fields = true;
    }

    if let Some(title) = body.title {
        if has_fields {
            qb.push(", ");
        }
        qb.push("title = ");
        qb.push_bind(title);
        has_fields = true;
    }

    if let Some(email) = body.email {
        if has_fields {
            qb.push(", ");
        }
        qb.push("email = ");
        qb.push_bind(email);
        has_fields = true;
    }

    if let Some(phone) = body.phone {
        if has_fields {
            qb.push(", ");
        }
        qb.push("phone = ");
        qb.push_bind(phone);
        has_fields = true;
    }

    if let Some(id_number) = body.id_number {
        if has_fields {
            qb.push(", ");
        }
        qb.push("id_number = ");
        qb.push_bind(id_number);
        has_fields = true;
    }

    if let Some(address) = body.address {
        if has_fields {
            qb.push(", ");
        }
        qb.push("address = ");
        qb.push_bind(address);
        has_fields = true;
    }

    if let Some(avatar) = body.avatar {
        if has_fields {
            qb.push(", ");
        }
        qb.push("avatar = ");
        qb.push_bind(avatar);
        has_fields = true;
    }

    if let Some(hire_date) = body.hire_date {
        if has_fields {
            qb.push(", ");
        }
        qb.push("hire_date = ");
        qb.push_bind(hire_date);
        has_fields = true;
    }

    if let Some(status) = body.status {
        if has_fields {
            qb.push(", ");
        }
        qb.push("status = ");
        qb.push_bind(status);
        has_fields = true;
    }

    if !has_fields {
        return Ok(Json(ApiResponse::ok_msg("ok")));
    }

    qb.push(" WHERE id = ");
    qb.push_bind(&id);

    qb.build().execute(&state.pool).await?;

    Ok(Json(ApiResponse::ok_msg("ok")))
}

pub async fn delete_employee(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "employee:delete")?;

    if id == auth.id {
        return Err(AppError::BadRequest("不能删除自己".to_string()));
    }

    // F-24: 受保护账号（protect_block=1）禁止管理操作，防越权删除/禁用/改密/改权超管。
    let protected: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM employees WHERE id = ? AND protect_block = 1",
    )
    .bind(&id)
    .fetch_one(&state.pool)
    .await?;
    if protected > 0 {
        return Err(AppError::BadRequest("该账号受保护，禁止该操作".to_string()));
    }

    let result = sqlx::query("DELETE FROM employees WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    append_log(&state.config.log_file, &format!("用户 {} 删除了员工 {}", user_tag(&auth.name, &auth.username), id));
    Ok(Json(ApiResponse::ok_msg("ok")))
}

pub async fn reset_password(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<ResetPasswordRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "employee:password")?;

    if id == auth.id {
        return Err(AppError::BadRequest("不能重置自己的密码，请在个人资料中修改".to_string()));
    }

    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM employees WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }

    // F-24: 受保护账号（protect_block=1）禁止管理操作，防越权删除/禁用/改密/改权超管。
    let protected: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM employees WHERE id = ? AND protect_block = 1",
    )
    .bind(&id)
    .fetch_one(&state.pool)
    .await?;
    if protected > 0 {
        return Err(AppError::BadRequest("该账号受保护，禁止该操作".to_string()));
    }

    let password = hash_password(&body.new_password, state.config.bcrypt_cost)?;

    // F-08: 重置密码后 pwd_version 递增，踢掉该员工所有已登录会话；F-02: 强制其下次登录修改密码。
    sqlx::query(
        "UPDATE employees SET password = ?, pwd_version = pwd_version + 1, must_change_password = 1 WHERE id = ?",
    )
    .bind(&password)
    .bind(&id)
    .execute(&state.pool)
    .await?;

    Ok(Json(ApiResponse::ok_msg("密码已重置")))
}

pub async fn update_employee_permissions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateEmployeePermissionsRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "employee:edit")?;

    if id == auth.id {
        return Err(AppError::BadRequest("不能修改自己的权限".to_string()));
    }

    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM employees WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }

    // F-24: 受保护账号（protect_block=1）禁止管理操作，防越权删除/禁用/改密/改权超管。
    let protected: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM employees WHERE id = ? AND protect_block = 1",
    )
    .bind(&id)
    .fetch_one(&state.pool)
    .await?;
    if protected > 0 {
        return Err(AppError::BadRequest("该账号受保护，禁止该操作".to_string()));
    }

    // F-01: 新授权权限必须是操作者自身权限的子集，防止受限管理员授予自己没有的权限（如 system:config）。
    for code in &body.permission_codes {
        if !auth.permissions.iter().any(|p| p == code) {
            return Err(AppError::Forbidden);
        }
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM employee_permissions WHERE employee_id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await?;

    for code in &body.permission_codes {
        let perm_id: Option<i32> = sqlx::query_scalar("SELECT id FROM permissions WHERE code = ?")
            .bind(code)
            .fetch_optional(&mut *tx)
            .await?;
        if let Some(pid) = perm_id {
            sqlx::query("INSERT INTO employee_permissions (employee_id, permission_id) VALUES (?, ?)")
                .bind(&id)
                .bind(pid)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    Ok(Json(ApiResponse::ok_msg("权限已更新")))
}
