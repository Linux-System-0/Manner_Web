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
    /// 有效权限码集合（由角色授权派生）。
    pub permissions: Vec<String>,
    /// 有效授权（码 + 数据范围），供前端范围感知展示。
    pub grants: Vec<crate::services::permission::Grant>,
    /// 归属部门 id 列表（多对多）。
    pub department_ids: Vec<String>,
    /// 分配的角色 id 列表。
    pub role_ids: Vec<String>,
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

// ---- 权限字典（角色授权用）----

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
