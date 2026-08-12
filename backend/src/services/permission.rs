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

//! 权限解析核心（方案 C：RBAC + 数据范围 + 部门角色继承）。
//!
//! 最终权限 = Σ（员工角色 + 部门角色）沿 `parent_id` 继承链展开后的角色权限并集。
//! 每个授权粒度为 `Grant { code, scope_type, scope_department_ids }`，
//! 数据范围仅作用于「数据型权限」（见 `is_data_scoped_code`）。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

/// 单一授权粒度：权限码 + 数据范围。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    pub code: String,
    /// all | subtree | department | self | custom
    pub scope_type: String,
    /// scope_type=custom 时指定的部门集合。
    #[serde(default)]
    pub scope_department_ids: Vec<String>,
}

/// 数据型权限码：受数据范围过滤。
pub fn is_data_scoped_code(code: &str) -> bool {
    matches!(code, "employee:list" | "employee:view" | "employee:view_sensitive")
}

/// 员工是否持有指定权限码（任意来源）。
pub fn has_permission(grants: &[Grant], code: &str) -> bool {
    grants.iter().any(|g| g.code == code)
}

/// 权限码集合（去重），供令牌/前端展示。
pub fn permission_codes(grants: &[Grant]) -> Vec<String> {
    let mut codes: Vec<String> = grants.iter().map(|g| g.code.clone()).collect();
    codes.sort();
    codes.dedup();
    codes
}

/// 解析员工的有效授权：直接角色（employee_roles）∪ 部门角色（department_roles），
/// 再沿 `parent_id` 继承链向上展开；同一 (code, scope) 去重。
pub async fn resolve_effective_grants(
    pool: &MySqlPool,
    employee_id: &str,
) -> Result<Vec<Grant>, sqlx::Error> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "WITH RECURSIVE base AS (
             SELECT role_id AS id FROM employee_roles WHERE employee_id = ?
             UNION
             SELECT dr.role_id FROM employee_departments ed
             JOIN department_roles dr ON dr.department_id = ed.department_id
             WHERE ed.employee_id = ?
         ),
         chain AS (
             SELECT id FROM base
             UNION ALL
             SELECT r.parent_id FROM roles r JOIN chain c ON r.id = c.id
         )
         SELECT DISTINCT c.id AS role_id, r.scope_type, p.code
         FROM chain c
         JOIN roles r ON r.id = c.id
         JOIN role_permissions rp ON rp.role_id = c.id
         JOIN permissions p ON p.id = rp.permission_id",
    )
    .bind(employee_id)
    .bind(employee_id)
    .fetch_all(pool)
    .await?;

    // custom 范围角色的部门集合
    let role_ids: Vec<&str> = {
        let mut ids: Vec<&str> = rows.iter().map(|(rid, _, _)| rid.as_str()).collect();
        ids.sort();
        ids.dedup();
        ids
    };
    let mut dept_map: HashMap<String, Vec<String>> = HashMap::new();
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

    // 聚合去重：code+scope → Grant
    let mut seen: Vec<Grant> = Vec::new();
    for (rid, scope_type, code) in rows {
        let mut depts = match scope_type.as_str() {
            "custom" => dept_map.get(&rid).cloned().unwrap_or_default(),
            _ => Vec::new(),
        };
        depts.sort();
        depts.dedup();
        let grant = Grant {
            code,
            scope_type,
            scope_department_ids: depts,
        };
        if !seen.contains(&grant) {
            seen.push(grant);
        }
    }
    seen.sort_by(|a, b| a.code.cmp(&b.code));
    Ok(seen)
}

/// 员工对某权限码的有效数据范围（各来源取并集；任一来源为 all 则全可见）。
#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// 操作者 id（self 范围判定）。
    pub auth_id: String,
    /// all：全部数据可见。
    pub all: bool,
    /// self：仅本人。
    pub self_only: bool,
    /// department：与本人共享任一部门。
    pub shared_department: bool,
    /// subtree：本人部门 ∪ 全部后代部门。
    pub subtree: bool,
    /// 本人归属部门 id。
    pub own_dept_ids: Vec<String>,
    /// subtree 范围部门集合（本人部门 ∪ 后代）。
    pub subtree_dept_ids: Vec<String>,
    /// custom 范围部门集合。
    pub custom_dept_ids: Vec<String>,
}

/// 构建操作者对指定权限码的有效数据范围。
pub async fn build_scope(
    pool: &MySqlPool,
    grants: &[Grant],
    code: &str,
    auth_id: &str,
) -> Result<Option<Scope>, sqlx::Error> {
    let matched: Vec<&Grant> = grants.iter().filter(|g| g.code == code).collect();
    if matched.is_empty() {
        return Ok(None);
    }

    let own: Vec<String> = sqlx::query_scalar(
        "SELECT department_id FROM employee_departments WHERE employee_id = ?",
    )
    .bind(auth_id)
    .fetch_all(pool)
    .await?;

    let mut scope = Scope {
        auth_id: auth_id.to_string(),
        own_dept_ids: own.clone(),
        ..Default::default()
    };
    let mut custom: Vec<String> = Vec::new();
    for g in matched {
        match g.scope_type.as_str() {
            "all" => scope.all = true,
            "subtree" => scope.subtree = true,
            "department" => scope.shared_department = true,
            "self" => scope.self_only = true,
            "custom" => custom.extend(g.scope_department_ids.iter().cloned()),
            _ => {}
        }
    }
    custom.sort();
    custom.dedup();
    scope.custom_dept_ids = custom;

    if scope.subtree && !scope.own_dept_ids.is_empty() {
        let placeholders = vec!["?"; scope.own_dept_ids.len()].join(",");
        let sql = format!(
            "WITH RECURSIVE d AS (
                 SELECT id FROM departments WHERE id IN ({})
                 UNION ALL
                 SELECT c.id FROM departments c JOIN d ON c.parent_id = d.id
             )
             SELECT DISTINCT id FROM d",
            placeholders
        );
        let mut q = sqlx::query_scalar::<_, String>(&sql);
        for did in &scope.own_dept_ids {
            q = q.bind(did);
        }
        let ids: Vec<String> = q.fetch_all(pool).await?;
        scope.subtree_dept_ids = ids;
    }

    Ok(Some(scope))
}

/// 把数据范围过滤条件追加到员工查询（目标表别名 alias，默认 `e`）。
/// 各范围条件按 OR 并集：满足任意一条即可见；all 直接不过滤。
pub fn apply_scope_filter(
    qb: &mut sqlx::QueryBuilder<'_, sqlx::MySql>,
    scope: &Scope,
    alias: &str,
) {
    if scope.all {
        return;
    }
    qb.push(" AND (");
    let mut first = true;
    if scope.self_only {
        qb.push(alias)
            .push(".id = ")
            .push_bind(scope.auth_id.clone());
        first = false;
    }
    if scope.shared_department && !scope.own_dept_ids.is_empty() {
        if !first {
            qb.push(" OR ");
        }
        qb.push("EXISTS (SELECT 1 FROM employee_departments _ed WHERE _ed.employee_id = ")
            .push(alias)
            .push(".id AND _ed.department_id IN (");
        push_in(qb, &scope.own_dept_ids);
        qb.push("))");
        first = false;
    }
    if scope.subtree && !scope.subtree_dept_ids.is_empty() {
        if !first {
            qb.push(" OR ");
        }
        qb.push("EXISTS (SELECT 1 FROM employee_departments _ed WHERE _ed.employee_id = ")
            .push(alias)
            .push(".id AND _ed.department_id IN (");
        push_in(qb, &scope.subtree_dept_ids);
        qb.push("))");
        first = false;
    }
    if !scope.custom_dept_ids.is_empty() {
        if !first {
            qb.push(" OR ");
        }
        qb.push("EXISTS (SELECT 1 FROM employee_departments _ed WHERE _ed.employee_id = ")
            .push(alias)
            .push(".id AND _ed.department_id IN (");
        push_in(qb, &scope.custom_dept_ids);
        qb.push("))");
        first = false;
    }
    if first {
        qb.push("1 = 0");
    }
    qb.push(")");
}

fn push_in(qb: &mut sqlx::QueryBuilder<'_, sqlx::MySql>, ids: &[String]) {
    for (i, id) in ids.iter().enumerate() {
        if i > 0 {
            qb.push(", ");
        }
        qb.push_bind(id.clone());
    }
}

/// 目标员工是否在当前范围内可见（单资源接口用，如详情/敏感信息）。
pub async fn is_visible(
    pool: &MySqlPool,
    grants: &[Grant],
    code: &str,
    auth_id: &str,
    target_id: &str,
) -> Result<bool, sqlx::Error> {
    let Some(scope) = build_scope(pool, grants, code, auth_id).await? else {
        return Ok(false);
    };
    scope_contains(pool, &scope, target_id).await
}

/// 目标员工是否在指定范围内可见。
pub async fn scope_contains(pool: &MySqlPool, scope: &Scope, target_id: &str) -> Result<bool, sqlx::Error> {
    if scope.all {
        return Ok(true);
    }
    if scope.self_only && target_id == scope.auth_id {
        return Ok(true);
    }
    let target_depts: Vec<String> = sqlx::query_scalar(
        "SELECT department_id FROM employee_departments WHERE employee_id = ?",
    )
    .bind(target_id)
    .fetch_all(pool)
    .await?;
    if target_depts.is_empty() {
        return Ok(false);
    }
    if scope.shared_department && target_depts.iter().any(|d| scope.own_dept_ids.contains(d)) {
        return Ok(true);
    }
    if scope.subtree && target_depts.iter().any(|d| scope.subtree_dept_ids.contains(d)) {
        return Ok(true);
    }
    if !scope.custom_dept_ids.is_empty() && target_depts.iter().any(|d| scope.custom_dept_ids.contains(d)) {
        return Ok(true);
    }
    Ok(false)
}

/// 防提权：操作者范围能否覆盖目标角色声明的范围。
/// 偏序：all ⊇ subtree ⊇ department ⊇ self；custom 须被操作者 custom 集合覆盖。
pub fn scope_covers(operator: &Scope, role_scope_type: &str, role_custom_depts: &[String]) -> bool {
    if operator.all {
        return true;
    }
    match role_scope_type {
        "all" => false,
        "subtree" => operator.subtree,
        "department" => operator.subtree || operator.shared_department,
        "self" => operator.subtree || operator.shared_department || operator.self_only,
        "custom" => {
            if operator.custom_dept_ids.is_empty() {
                return false;
            }
            role_custom_depts
                .iter()
                .all(|d| operator.custom_dept_ids.contains(d))
        }
        _ => false,
    }
}
