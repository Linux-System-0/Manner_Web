use axum::extract::{Path, Query, State};
use axum::Json;
use sqlx::QueryBuilder;
use uuid::Uuid;

use crate::error::AppError;
use crate::handlers::auth::{append_log, user_tag, ClientIp};
use crate::middleware::auth::{require_permission, AppState, AuthUser};
use crate::models::employee::{
    EmployeeDetail, EmployeeListParams, EmployeeListResponse, EmployeeListRow, NewEmployee,
    ResetPasswordRequest, SensitiveEmployeeInfo, UpdateEmployee,
    UpdateEmployeePermissionsRequest,
};
use crate::services::auth::hash_password;
use crate::utils::crypto;
use crate::utils::response::ApiResponse;

/// 解密单个敏感字段：无前缀视为存量明文（迁移兜底），带前缀则严格解密。
fn decrypt_field(v: Option<String>, key: &[u8; 32]) -> Result<Option<String>, AppError> {
    match v {
        None => Ok(None),
        Some(s) => {
            if s.starts_with(crypto::ENC_PREFIX) {
                crypto::try_decrypt_field(&s, key)
            } else {
                Ok(Some(s))
            }
        }
    }
}

/// 加密单个敏感字段：空串/空白转为 NULL，非空加密为密文。
fn encrypt_field_opt(v: Option<String>, key: &[u8; 32]) -> Result<Option<String>, AppError> {
    match v {
        None => Ok(None),
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => crypto::encrypt_field(s.trim(), key).map(Some),
    }
}

/// 头像 URL 白名单校验：仅接受本站上传接口产物 `/uploads/<uuid>.<图片扩展名>`。
/// 防止任意字符串（javascript: 等伪协议、外部 URL、含路径分隔符的穿越串）写入 avatar。
fn is_valid_avatar_url(url: &str) -> bool {
    let Some(rel) = url.strip_prefix("/uploads/") else {
        return false;
    };
    if rel.is_empty() || rel.contains('/') || rel.contains('\\') || rel.contains("..") {
        return false;
    }
    let Some(ext) = std::path::Path::new(rel)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
    else {
        return false;
    };
    if !crate::handlers::system::is_allowed_extension(&ext) {
        return false;
    }
    // 文件名须为服务端生成的 UUID（36 字符，字母数字 + 连字符）
    let stem = rel.strip_suffix(&format!(".{ext}")).unwrap_or(rel);
    stem.len() == 36 && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

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
         e.address, e.avatar, e.hire_date, e.status, e.protect_block, e.created_at, \
         (SELECT GROUP_CONCAT(d.name ORDER BY d.sort_order, d.created_at SEPARATOR '、') \
          FROM employee_departments ed JOIN departments d ON d.id = ed.department_id \
          WHERE ed.employee_id = e.id) AS departments \
         FROM employees e \
         WHERE 1=1",
    );

    if let Some(ref keyword) = params.keyword {
        let kw = format!("%{}%", keyword);
        // 敏感字段已静态加密（不可模糊查询），搜索仅支持 姓名/用户名。
        count_qb.push(" AND (e.username LIKE ");
        count_qb.push_bind(kw.clone());
        count_qb.push(" OR e.name LIKE ");
        count_qb.push_bind(kw.clone());
        count_qb.push(")");

        query_qb.push(" AND (e.username LIKE ");
        query_qb.push_bind(kw.clone());
        query_qb.push(" OR e.name LIKE ");
        query_qb.push_bind(kw);
        query_qb.push(")");
    }

    // 按部门过滤：员工须属于该部门（多对多任意一个匹配即可）。
    if let Some(ref dept_id) = params.department_id {
        let dq = " AND EXISTS (SELECT 1 FROM employee_departments ed \
                   WHERE ed.employee_id = e.id AND ed.department_id = ";
        count_qb.push(dq);
        count_qb.push_bind(dept_id);
        count_qb.push(")");

        query_qb.push(dq);
        query_qb.push_bind(dept_id);
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

    // 敏感字段全掩脱敏：密文与明文一律不返回，前端仅见 "***"。
    let rows = rows
        .into_iter()
        .map(|mut r| {
            r.email = crypto::mask_field(r.email);
            r.phone = crypto::mask_field(r.phone);
            r.id_number = crypto::mask_field(r.id_number);
            r.address = crypto::mask_field(r.address);
            r
        })
        .collect();

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

    let department_ids: Vec<String> = sqlx::query_scalar(
        "SELECT department_id FROM employee_departments WHERE employee_id = ?",
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(ApiResponse::ok(EmployeeDetail {
        id: row.id,
        username: row.username,
        name: row.name,
        title: row.title,
        email: crypto::mask_field(row.email),
        phone: crypto::mask_field(row.phone),
        id_number: crypto::mask_field(row.id_number),
        address: crypto::mask_field(row.address),
        avatar: row.avatar,
        hire_date: row.hire_date,
        status: row.status,
        protect_block: row.protect_block,
        permissions,
        department_ids,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })))
}

/// 查看员工敏感信息（解密明文）。
///
/// 安全约束：
/// - 必须持有 `employee:view_sensitive` 权限（身份由鉴权中间件校验）；
/// - 每次调用强制写入业务日志（manner.log，经 /api/system/logs 可见）：
///   记录操作者身份、被查看员工与访问 IP，实现可审计；
/// - 前端须先经两次确认（操作入口弹窗 + 查看按钮弹窗）才允许调用本接口。
pub async fn view_sensitive_info(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<SensitiveEmployeeInfo>>, AppError> {
    require_permission(&auth.permissions, "employee:view_sensitive")?;

    let ip = client_ip.0;

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

    let key = &state.config.field_enc_key;
    let email = decrypt_field(row.email, key)?;
    let phone = decrypt_field(row.phone, key)?;
    let id_number = decrypt_field(row.id_number, key)?;
    let address = decrypt_field(row.address, key)?;

    // 严格审计：记录操作者、被查看员工与访问 IP，杜绝无痕访问。
    append_log(
        &state.config.log_file,
        &format!(
            "【敏感信息】用户 {} 查看了员工 {} (id={}) 的完整信息",
            user_tag(&auth.name, &auth.username),
            user_tag(&row.name, &row.username),
            row.id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok(SensitiveEmployeeInfo {
        id: row.id,
        username: row.username,
        name: row.name,
        email,
        phone,
        id_number,
        address,
    })))
}

/// 查看员工敏感信息中的单个字段（解密明文），供前端逐字段「显示」按钮调用。
///
/// 安全约束与 `view_sensitive_info` 一致，但日志更细粒度：记录具体查看的是哪个字段
/// （邮箱/手机号/身份证号/地址），并追加访问 IP。
pub async fn view_sensitive_field(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path((id, field)): Path<(String, String)>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "employee:view_sensitive")?;

    let ip = client_ip.0;

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

    let key = &state.config.field_enc_key;
    let (value, label) = match field.as_str() {
        "email" => (decrypt_field(row.email, key)?, "邮箱"),
        "phone" => (decrypt_field(row.phone, key)?, "手机号"),
        "id_number" => (decrypt_field(row.id_number, key)?, "身份证号"),
        "address" => (decrypt_field(row.address, key)?, "地址"),
        _ => return Err(AppError::BadRequest("不支持的敏感字段".to_string())),
    };

    // 细粒度审计：记录具体查看的字段与访问 IP。
    append_log(
        &state.config.log_file,
        &format!(
            "【敏感信息】用户 {} 查看了员工 {} (id={}) 的{}",
            user_tag(&auth.name, &auth.username),
            user_tag(&row.name, &row.username),
            row.id,
            label
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "field": field,
        "value": value
    }))))
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
    client_ip: ClientIp,
    auth: AuthUser,
    Json(body): Json<NewEmployee>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "employee:create")?;

    let ip = client_ip.0;

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

    // 敏感字段静态加密后落库（AES-256-GCM，密钥 FIELD_ENC_KEY）。
    let key = &state.config.field_enc_key;
    let email = encrypt_field_opt(body.email, key)?;
    let phone = encrypt_field_opt(body.phone, key)?;
    let id_number = encrypt_field_opt(body.id_number, key)?;
    let address = encrypt_field_opt(body.address, key)?;

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
    .bind(&email)
    .bind(&phone)
    .bind(&id_number)
    .bind(&address)
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

    append_log(
        &state.config.log_file,
        &format!(
            "用户 {} 创建员工 {} ({})",
            user_tag(&auth.name, &auth.username),
            body.name,
            body.username
        ),
        &ip,
    );

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
        let enc = encrypt_field_opt(email, &state.config.field_enc_key)?;
        if has_fields {
            qb.push(", ");
        }
        qb.push("email = ");
        qb.push_bind(enc);
        has_fields = true;
    }

    if let Some(phone) = body.phone {
        let enc = encrypt_field_opt(phone, &state.config.field_enc_key)?;
        if has_fields {
            qb.push(", ");
        }
        qb.push("phone = ");
        qb.push_bind(enc);
        has_fields = true;
    }

    if let Some(id_number) = body.id_number {
        let enc = encrypt_field_opt(id_number, &state.config.field_enc_key)?;
        if has_fields {
            qb.push(", ");
        }
        qb.push("id_number = ");
        qb.push_bind(enc);
        has_fields = true;
    }

    if let Some(address) = body.address {
        let enc = encrypt_field_opt(address, &state.config.field_enc_key)?;
        if has_fields {
            qb.push(", ");
        }
        qb.push("address = ");
        qb.push_bind(enc);
        has_fields = true;
    }

    if let Some(avatar) = body.avatar {
        // F-6: 头像只接受本站上传图片（/uploads/<uuid>.<img-ext>），拒绝任意字符串。
        if let Some(ref a) = avatar {
            if !is_valid_avatar_url(a) {
                return Err(AppError::BadRequest(
                    "头像必须是本站上传的图片".to_string(),
                ));
            }
        }
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
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "employee:delete")?;

    let ip = client_ip.0;

    if id == auth.id {
        return Err(AppError::BadRequest("不能删除自己".to_string()));
    }

    let result = sqlx::query("DELETE FROM employees WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    append_log(
        &state.config.log_file,
        &format!(
            "用户 {} 删除了员工 {}",
            user_tag(&auth.name, &auth.username),
            id
        ),
        &ip,
    );
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

    crate::services::auth::validate_password_strength(&body.new_password)?;
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

    // F-01: 操作者只能增删「自己拥有的权限」——目标现有但操作者没有的权限冻结保持：
    //   - 新增（目标当前没有、新集合里有）→ 必须是操作者已有权限，否则 403（防提权）；
    //   - 移除（目标当前有、新集合里没有）→ 必须是操作者已有权限，否则 403（防降权他人能力）。
    //   两者都满足时，目标已有但操作者没有的权限被自动保留。
    let current: Vec<String> = sqlx::query_scalar(
        "SELECT p.code FROM employee_permissions ep JOIN permissions p ON p.id = ep.permission_id WHERE ep.employee_id = ?",
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;

    let owned: std::collections::HashSet<&str> =
        auth.permissions.iter().map(|s| s.as_str()).collect();
    let new_set: std::collections::HashSet<&str> =
        body.permission_codes.iter().map(|s| s.as_str()).collect();
    let cur_set: std::collections::HashSet<&str> = current.iter().map(|s| s.as_str()).collect();

    for code in &new_set {
        if !cur_set.contains(code) && !owned.contains(code) {
            return Err(AppError::Forbidden);
        }
    }
    for code in &cur_set {
        if !new_set.contains(code) && !owned.contains(code) {
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

    // 防拉黑保护与权限联动：勾选 chat:protect_block 权限 ⇔ 该员工受保护（protect_block=1）。
    // 该标记只影响聊天拉黑拦截（见 chat.rs block_user），不影响改密/删除/改权等管理操作。
    let protect: i8 = if body.permission_codes.iter().any(|c| c == "chat:protect_block") {
        1
    } else {
        0
    };
    sqlx::query("UPDATE employees SET protect_block = ? WHERE id = ?")
        .bind(protect)
        .bind(&id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Json(ApiResponse::ok_msg("权限已更新")))
}
