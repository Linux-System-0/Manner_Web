use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub is_system: i8,
    pub scope_type: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// 角色列表/详情项：附带权限码、custom 范围部门、成员数。
#[derive(Debug, Serialize)]
pub struct RoleListItem {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub parent_name: Option<String>,
    pub is_system: i8,
    pub scope_type: String,
    pub description: Option<String>,
    pub permission_codes: Vec<String>,
    pub scope_department_ids: Vec<String>,
    pub member_count: i64,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub parent_id: Option<String>,
    pub scope_type: String,
    pub description: Option<String>,
    #[serde(default)]
    pub permission_codes: Vec<String>,
    #[serde(default)]
    pub scope_department_ids: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub parent_id: Option<Option<String>>,
    pub scope_type: Option<String>,
    pub description: Option<Option<String>>,
    pub permission_codes: Option<Vec<String>>,
    pub scope_department_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEmployeeRolesRequest {
    pub role_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDepartmentRolesRequest {
    pub role_ids: Vec<String>,
}
