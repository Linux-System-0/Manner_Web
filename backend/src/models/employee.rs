use chrono::NaiveDate;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Employee {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub name: String,
    pub title: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub id_number: Option<String>,
    pub address: Option<String>,
    pub avatar: Option<String>,
    pub hire_date: Option<NaiveDate>,
    #[allow(dead_code)]
    pub status: i8,
    pub protect_block: i8,
    /// F-08: 密码版本号（改密时递增，旧 token 失效）。
    #[allow(dead_code)]
    pub pwd_version: i64,
    /// F-02: 首次登录强制改密标记（随机初始密码创建的用户为 1）。
    #[allow(dead_code)]
    pub must_change_password: i8,
    pub preferences: Option<String>,
    /// 当前有效会话 id（单设备登录：新登录会覆盖此值，旧会话令牌立即失效）。
    #[allow(dead_code)]
    pub active_session: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub name: String,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct NewEmployee {
    pub username: String,
    pub name: String,
    pub title: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub id_number: Option<String>,
    pub address: Option<String>,
    pub hire_date: Option<NaiveDate>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateEmployee {
    pub name: Option<String>,
    pub title: Option<Option<String>>,
    pub email: Option<Option<String>>,
    pub phone: Option<Option<String>>,
    pub id_number: Option<Option<String>>,
    pub address: Option<Option<String>>,
    pub avatar: Option<Option<String>>,
    pub hire_date: Option<Option<NaiveDate>>,
    pub status: Option<i8>,
}

#[derive(Debug, Deserialize)]
pub struct EmployeeListParams {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub keyword: Option<String>,
    /// 按部门 id 过滤（部门成员筛选）。
    pub department_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EmployeeListResponse {
    pub items: Vec<EmployeeListRow>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EmployeeListRow {
    pub id: String,
    pub username: String,
    pub name: String,
    pub title: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub id_number: Option<String>,
    pub address: Option<String>,
    pub avatar: Option<String>,
    pub hire_date: Option<NaiveDate>,
    pub status: i8,
    pub protect_block: i8,
    pub created_at: NaiveDateTime,
    /// 归属部门名称（逗号分隔，无部门为 NULL）。
    pub departments: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EmployeeDetail {
    pub id: String,
    pub username: String,
    pub name: String,
    pub title: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub id_number: Option<String>,
    pub address: Option<String>,
    pub avatar: Option<String>,
    pub hire_date: Option<NaiveDate>,
    pub status: i8,
    pub protect_block: i8,
    pub permissions: Vec<String>,
    /// 归属部门 id 列表（多对多）。
    pub department_ids: Vec<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// 查看敏感信息解密结果（仅 employee:view_sensitive 权限可调用，且强制写日志）。
#[derive(Debug, Serialize)]
pub struct SensitiveEmployeeInfo {
    pub id: String,
    pub username: String,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub id_number: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: i64,
    pub user: LoginUserInfo,
}

#[derive(Debug, Serialize)]
pub struct LoginUserInfo {
    pub id: String,
    pub username: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub avatar: Option<String>,
    /// F-02: 首次登录强制改密标记（随机初始密码创建的用户为 true，前端应引导修改密码）。
    pub must_change_password: bool,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEmployeePermissionsRequest {
    pub permission_codes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PrecheckRequest {
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct PrecheckResponse {
    /// true 表示该用户名处于「首次登录待设置密码」状态（must_change_password=1）。
    /// 用户名不存在或无需改密时均返回 false，避免泄露账号枚举信号。
    pub must_change: bool,
}

#[derive(Debug, Deserialize)]
pub struct FirstLoginRequest {
    pub username: String,
    /// 当前生效的初始密码（创建员工/重置密码时下发的一次性密码）。
    /// F-20: 改密前必须校验，否则任意人可接管待激活账号。
    pub initial_password: String,
    /// 用户设置的新密码。
    pub new_password: String,
}

// ---- 权限字典（员工直接授权用，原角色模块迁移至此）----

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Permission {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub module: String,
}

#[derive(Debug, Serialize)]
pub struct PermissionModule {
    pub module: String,
    pub module_name: String,
    pub permissions: Vec<PermissionInfo>,
}

#[derive(Debug, Serialize)]
pub struct PermissionInfo {
    pub code: String,
    pub name: String,
}
