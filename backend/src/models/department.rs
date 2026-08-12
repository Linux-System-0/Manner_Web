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

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 部门列表行：附带负责人（多选）与成员数。
#[derive(Debug, Serialize, FromRow)]
pub struct DepartmentListRow {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    /// 负责人姓名列表（顿号分隔，GROUP_CONCAT 聚合）。
    pub leader_names: Option<String>,
    /// 绑定角色名称列表（顿号分隔，GROUP_CONCAT 聚合）。
    pub role_names: Option<String>,
    pub member_count: i64,
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub struct NewDepartment {
    pub name: String,
    pub parent_id: Option<String>,
    /// 负责人 id 列表（可多选）。
    pub leader_ids: Vec<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateDepartment {
    pub name: Option<String>,
    pub parent_id: Option<Option<String>>,
    /// 负责人 id 列表整体替换（空数组表示清空）。
    pub leader_ids: Option<Vec<String>>,
    pub sort_order: Option<i32>,
}

/// 部门成员行：附带是否为本部门负责人标记。
#[derive(Debug, Serialize, FromRow)]
pub struct DepartmentMemberRow {
    pub id: String,
    pub username: String,
    pub name: String,
    pub title: Option<String>,
    pub avatar: Option<String>,
    pub status: i8,
    /// 1 = 本部门负责人，0 = 普通成员。
    pub is_leader: i8,
}

/// 更新员工归属部门的请求（多对多，整体替换）。
#[derive(Debug, Deserialize)]
pub struct UpdateEmployeeDepartmentsRequest {
    pub department_ids: Vec<String>,
}
