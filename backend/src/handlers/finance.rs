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

//! 财务模块：报销管理（提交/审批/复核/付款/撤回，全程留痕）、发票管理（查重）、
//! 收付款记录、预算管理（超支预警）、财务报表（汇总/排行/趋势/CSV 导出）。
//!
//! 权限码（module = finance）：
//! - finance:reimburse_view    查看报销单（数据范围过滤，范围作用于提交时部门快照）
//! - finance:reimburse_create  提交报销
//! - finance:reimburse_approve 部门负责人审批（数据范围过滤）
//! - finance:reimburse_manage  财务复核/付款/删除（全量）
//! - finance:invoice_manage    发票管理
//! - finance:payment_manage    收付款记录管理
//! - finance:budget_manage     预算管理
//! - finance:report_view       财务报表

use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::Response;
use axum::Json;
use chrono::{Local, NaiveDate};
use uuid::Uuid;

use crate::error::AppError;
use crate::handlers::auth::{append_log, user_tag, ClientIp};
use crate::middleware::auth::{require_permission, AppState, AuthUser};
use crate::models::finance::{
    BudgetQuery, BudgetRawRow, DepartmentReportRow, InvoiceQuery, InvoiceRow, NewBudget, NewInvoice,
    NewPayment, NewReimbursement, PaymentQuery, PaymentRow, ReimbursementDetail,
    ReimbursementDetailRow, ReimbursementListRow, ReimbursementLogRow, ReimbursementQuery,
    ReportQuery, ReportSummary, ReviewAction, TrendRow, UpdateBudget, UpdateInvoice,
    UpdatePayment, UpdateReimbursement,
};
use crate::services::permission::{build_scope, has_permission, Scope};
use crate::utils::response::ApiResponse;

/// 报销单可编辑状态（提交人视角）：待审批 / 已驳回 / 已撤回。
fn is_owner_editable(status: &str) -> bool {
    matches!(status, "pending_leader" | "rejected" | "withdrawn")
}

/// 金额校验：正数、≤ 99999999.99、最多两位小数（与 DECIMAL(12,2) 对齐）。
fn validate_amount(amount: f64) -> Result<(), AppError> {
    if !amount.is_finite() || amount <= 0.0 || amount > 99_999_999.99 {
        return Err(AppError::ValidationError(
            "金额必须是大于 0 且不超过 99999999.99 的数字".to_string(),
        ));
    }
    let cents = (amount * 100.0).round();
    if (cents / 100.0 - amount).abs() > 1e-9 {
        return Err(AppError::ValidationError(
            "金额最多保留两位小数".to_string(),
        ));
    }
    Ok(())
}

/// 追加「报销单可见范围」条件（作用于别名 r）：
/// 本人提交的 ∪ 范围内部门（department_id 快照）的报销单。
fn push_reimburse_scope(qb: &mut sqlx::QueryBuilder<'_, sqlx::MySql>, scope: &Scope) {
    if scope.all {
        return;
    }
    qb.push(" AND (");
    let mut first = true;
    if scope.self_only {
        qb.push("r.employee_id = ").push_bind(scope.auth_id.clone());
        first = false;
    }
    let mut dept_ids: Vec<String> = Vec::new();
    if scope.shared_department {
        dept_ids.extend(scope.own_dept_ids.iter().cloned());
    }
    if scope.subtree {
        dept_ids.extend(scope.subtree_dept_ids.iter().cloned());
    }
    dept_ids.extend(scope.custom_dept_ids.iter().cloned());
    dept_ids.sort();
    dept_ids.dedup();
    if !dept_ids.is_empty() {
        if !first {
            qb.push(" OR ");
        }
        qb.push("r.department_id IN (");
        for (i, id) in dept_ids.iter().enumerate() {
            if i > 0 {
                qb.push(", ");
            }
            qb.push_bind(id.clone());
        }
        qb.push(")");
        first = false;
    }
    if first {
        qb.push("1 = 0");
    }
    qb.push(")");
}

/// 目标部门是否在操作者的报销审批范围内。
fn dept_in_scope(scope: &Scope, dept_id: &str) -> bool {
    if scope.all {
        return true;
    }
    if scope.shared_department && scope.own_dept_ids.iter().any(|d| d == dept_id) {
        return true;
    }
    if scope.subtree && scope.subtree_dept_ids.iter().any(|d| d == dept_id) {
        return true;
    }
    scope.custom_dept_ids.iter().any(|d| d == dept_id)
}

/// 报销单是否对当前操作者可见（详情/操作前置校验）。
async fn can_view_reimbursement(
    state: &AppState,
    auth: &AuthUser,
    employee_id: &str,
    department_id: &str,
) -> Result<bool, AppError> {
    // 财务复核/付款权限 = 全量可见。
    if has_permission(&auth.grants, "finance:reimburse_manage") {
        return Ok(true);
    }
    if auth.id == employee_id {
        return Ok(true);
    }
    let code = if has_permission(&auth.grants, "finance:reimburse_approve") {
        "finance:reimburse_approve"
    } else {
        "finance:reimburse_view"
    };
    if !has_permission(&auth.grants, code) {
        return Ok(false);
    }
    let Some(scope) = build_scope(&state.pool, &auth.grants, code, &auth.id).await? else {
        return Ok(false);
    };
    Ok(dept_in_scope(&scope, department_id))
}

/// 插入审批/状态流水（全程留痕）。executor 须与业务写入处于同一事务，
/// 否则 FK 校验会等待未提交的行锁（lock wait timeout）。
async fn insert_log<'e, E>(
    executor: E,
    reimbursement_id: &str,
    action: &str,
    actor_id: &str,
    comment: Option<&str>,
) -> Result<(), AppError>
where
    E: sqlx::Executor<'e, Database = sqlx::MySql>,
{
    sqlx::query(
        "INSERT INTO reimbursement_logs (id, reimbursement_id, action, actor_id, comment) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(reimbursement_id)
    .bind(action)
    .bind(actor_id)
    .bind(comment)
    .execute(executor)
    .await?;
    Ok(())
}

/// 按发票 id 列表同步发票关联状态（claimed 已关联 / unused 未关联）。
async fn sync_invoice_status<'e, E>(
    executor: E,
    invoice_ids: &[String],
) -> Result<(), AppError>
where
    E: sqlx::Executor<'e, Database = sqlx::MySql>,
{
    if invoice_ids.is_empty() {
        return Ok(());
    }
    let placeholders = vec!["?"; invoice_ids.len()].join(",");
    let sql = format!(
        "UPDATE invoices i \
         SET i.status = CASE WHEN EXISTS (SELECT 1 FROM reimbursement_invoices ri \
                          WHERE ri.invoice_id = i.id) THEN 'claimed' ELSE 'unused' END \
         WHERE i.id IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&sql);
    for id in invoice_ids {
        q = q.bind(id);
    }
    q.execute(executor).await?;
    Ok(())
}

/// 校验发票 id 列表：去重、存在、未被其他报销单占用；返回去重后的列表。
async fn validate_invoice_ids(
    pool: &sqlx::MySqlPool,
    invoice_ids: &[String],
    exclude_reimbursement: Option<&str>,
) -> Result<Vec<String>, AppError> {
    let mut unique: Vec<String> = invoice_ids.to_vec();
    unique.sort();
    unique.dedup();
    if unique.is_empty() {
        return Ok(unique);
    }
    let placeholders = vec!["?"; unique.len()].join(",");
    let sql = format!(
        "SELECT id FROM invoices WHERE id IN ({}) AND NOT EXISTS (\
         SELECT 1 FROM reimbursement_invoices ri \
         WHERE ri.invoice_id = invoices.id{} \
        )",
        placeholders,
        if exclude_reimbursement.is_some() {
            " AND ri.reimbursement_id != ?"
        } else {
            ""
        }
    );
    let mut q = sqlx::query_scalar::<_, String>(&sql);
    for id in &unique {
        q = q.bind(id);
    }
    if let Some(rid) = exclude_reimbursement {
        q = q.bind(rid);
    }
    let found: Vec<String> = q.fetch_all(pool).await?;
    let missing: Vec<String> = unique
        .iter()
        .filter(|id| !found.contains(id))
        .cloned()
        .collect();
    if !missing.is_empty() {
        return Err(AppError::BadRequest(
            "部分发票不存在或已被其他报销单关联".to_string(),
        ));
    }
    Ok(unique)
}

/// 员工首个归属部门（报销提交时部门快照）。
async fn employee_primary_department(
    pool: &sqlx::MySqlPool,
    employee_id: &str,
) -> Result<String, AppError> {
    let dept: Option<String> = sqlx::query_scalar(
        "SELECT department_id FROM employee_departments WHERE employee_id = ? \
         ORDER BY created_at LIMIT 1",
    )
    .bind(employee_id)
    .fetch_optional(pool)
    .await?;
    dept.ok_or_else(|| {
        AppError::BadRequest("请先加入部门后再提交报销申请".to_string())
    })
}

// ============================================================================
// 报销单
// ============================================================================

/// 报销单列表（数据范围过滤 + 状态/关键词/部门筛选）。
pub async fn list_reimbursements(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ReimbursementQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let manage = has_permission(&auth.grants, "finance:reimburse_manage");
    if !manage
        && !has_permission(&auth.grants, "finance:reimburse_view")
        && !has_permission(&auth.grants, "finance:reimburse_approve")
    {
        return Err(AppError::Forbidden);
    }

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT r.id, r.employee_id, e.name AS employee_name, r.department_id, \
         d.name AS department_name, r.title, r.category, CAST(r.amount AS DOUBLE) AS amount, \
         r.currency, r.status, r.approver_id, ae.name AS approver_name, r.approve_comment, \
         r.finance_reviewer_id, af.name AS finance_reviewer_name, r.finance_comment, \
         r.paid_at, r.created_at \
         FROM reimbursements r \
         JOIN employees e ON e.id = r.employee_id \
         JOIN departments d ON d.id = r.department_id \
         LEFT JOIN employees ae ON ae.id = r.approver_id \
         LEFT JOIN employees af ON af.id = r.finance_reviewer_id \
         WHERE 1 = 1",
    );
    if !manage {
        let code = if has_permission(&auth.grants, "finance:reimburse_approve") {
            "finance:reimburse_approve"
        } else {
            "finance:reimburse_view"
        };
        let scope = build_scope(&state.pool, &auth.grants, code, &auth.id).await?;
        if let Some(scope) = scope {
            push_reimburse_scope(&mut qb, &scope);
        }
    }
    if let Some(status) = &query.status {
        if !status.is_empty() {
            qb.push(" AND r.status = ").push_bind(status);
        }
    }
    if let Some(dept) = &query.department_id {
        if !dept.is_empty() {
            qb.push(" AND r.department_id = ").push_bind(dept);
        }
    }
    if let Some(keyword) = &query.keyword {
        let kw = keyword.trim();
        if !kw.is_empty() {
            qb.push(" AND (r.title LIKE ")
                .push_bind(format!("%{}%", kw))
                .push(" OR e.name LIKE ")
                .push_bind(format!("%{}%", kw))
                .push(")");
        }
    }
    qb.push(" ORDER BY r.created_at DESC LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind((page - 1) * page_size);

    let items: Vec<ReimbursementListRow> = qb.build_query_as().fetch_all(&state.pool).await?;

    // 总条数（同条件计数）。
    let mut cqb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT COUNT(*) FROM reimbursements r \
         JOIN employees e ON e.id = r.employee_id \
         WHERE 1 = 1",
    );
    if !manage {
        let code = if has_permission(&auth.grants, "finance:reimburse_approve") {
            "finance:reimburse_approve"
        } else {
            "finance:reimburse_view"
        };
        let scope = build_scope(&state.pool, &auth.grants, code, &auth.id).await?;
        if let Some(scope) = scope {
            push_reimburse_scope(&mut cqb, &scope);
        }
    }
    if let Some(status) = &query.status {
        if !status.is_empty() {
            cqb.push(" AND r.status = ").push_bind(status);
        }
    }
    if let Some(dept) = &query.department_id {
        if !dept.is_empty() {
            cqb.push(" AND r.department_id = ").push_bind(dept);
        }
    }
    if let Some(keyword) = &query.keyword {
        let kw = keyword.trim();
        if !kw.is_empty() {
            cqb.push(" AND (r.title LIKE ")
                .push_bind(format!("%{}%", kw))
                .push(" OR e.name LIKE ")
                .push_bind(format!("%{}%", kw))
                .push(")");
        }
    }
    let total: i64 = cqb.build_query_scalar().fetch_one(&state.pool).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// 报销单详情（含发票与审批流水）。
pub async fn get_reimbursement(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<ReimbursementDetail>>, AppError> {
    let manage = has_permission(&auth.grants, "finance:reimburse_manage");
    if !manage
        && !has_permission(&auth.grants, "finance:reimburse_view")
        && !has_permission(&auth.grants, "finance:reimburse_approve")
    {
        return Err(AppError::Forbidden);
    }

    let row: Option<ReimbursementDetailRow> = sqlx::query_as(
        "SELECT r.employee_id, e.name AS employee_name, r.department_id, d.name AS department_name, \
         r.title, r.category, CAST(r.amount AS DOUBLE) AS amount, r.currency, r.reason, r.status, \
         r.approver_id, ae.name AS approver_name, r.approve_comment, r.approved_at, \
         r.finance_reviewer_id, af.name AS finance_reviewer_name, r.finance_comment, \
         r.finance_reviewed_at, r.paid_at, r.created_at \
         FROM reimbursements r \
         JOIN employees e ON e.id = r.employee_id \
         JOIN departments d ON d.id = r.department_id \
         LEFT JOIN employees ae ON ae.id = r.approver_id \
         LEFT JOIN employees af ON af.id = r.finance_reviewer_id \
         WHERE r.id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?;

    let Some(row) = row else {
        return Err(AppError::NotFound);
    };

    if !can_view_reimbursement(&state, &auth, &row.employee_id, &row.department_id).await? {
        return Err(AppError::Forbidden);
    }

    let invoices: Vec<InvoiceRow> = sqlx::query_as(
        "SELECT i.id, i.invoice_code, i.invoice_type, CAST(i.amount AS DOUBLE) AS amount, \
         CAST(i.tax_amount AS DOUBLE) AS tax_amount, i.issued_at, i.issuer_name, i.buyer_name, \
         i.image_url, i.employee_id, e.name AS employee_name, i.status, i.created_at \
         FROM reimbursement_invoices ri \
         JOIN invoices i ON i.id = ri.invoice_id \
         JOIN employees e ON e.id = i.employee_id \
         WHERE ri.reimbursement_id = ? ORDER BY ri.created_at",
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;

    let logs: Vec<ReimbursementLogRow> = sqlx::query_as(
        "SELECT l.id, l.action, l.actor_id, e.name AS actor_name, l.comment, l.created_at \
         FROM reimbursement_logs l JOIN employees e ON e.id = l.actor_id \
         WHERE l.reimbursement_id = ? ORDER BY l.created_at",
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(ApiResponse::ok(ReimbursementDetail {
        id,
        employee_id: row.employee_id,
        employee_name: row.employee_name,
        department_id: row.department_id,
        department_name: row.department_name,
        title: row.title,
        category: row.category,
        amount: row.amount,
        currency: row.currency,
        reason: row.reason,
        status: row.status,
        approver_id: row.approver_id,
        approver_name: row.approver_name,
        approve_comment: row.approve_comment,
        approved_at: row.approved_at,
        finance_reviewer_id: row.finance_reviewer_id,
        finance_reviewer_name: row.finance_reviewer_name,
        finance_comment: row.finance_comment,
        finance_reviewed_at: row.finance_reviewed_at,
        paid_at: row.paid_at,
        created_at: row.created_at,
        invoices,
        logs,
    })))
}

/// 提交报销（创建即进入待部门审批，记录流水）。
pub async fn create_reimbursement(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Json(body): Json<NewReimbursement>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "finance:reimburse_create")?;

    let ip = client_ip.0;
    let title = body.title.trim().to_string();
    if title.is_empty() || title.chars().count() > 128 {
        return Err(AppError::ValidationError(
            "报销事由不能为空且不能超过 128 字符".to_string(),
        ));
    }
    let category = body.category.trim().to_string();
    if category.is_empty() || category.chars().count() > 32 {
        return Err(AppError::ValidationError(
            "费用类型不能为空且不能超过 32 字符".to_string(),
        ));
    }
    validate_amount(body.amount)?;
    if let Some(reason) = &body.reason {
        if reason.chars().count() > 1000 {
            return Err(AppError::ValidationError(
                "报销说明不能超过 1000 字符".to_string(),
            ));
        }
    }
    let invoice_ids = validate_invoice_ids(&state.pool, &body.invoice_ids, None).await?;

    let department_id = employee_primary_department(&state.pool, &auth.id).await?;
    let id = Uuid::new_v4().to_string();

    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO reimbursements (id, employee_id, department_id, title, category, amount, currency, reason, status) \
         VALUES (?, ?, ?, ?, ?, ?, 'CNY', ?, 'pending_leader')",
    )
    .bind(&id)
    .bind(&auth.id)
    .bind(&department_id)
    .bind(&title)
    .bind(&category)
    .bind(body.amount)
    .bind(&body.reason)
    .execute(&mut *tx)
    .await?;

    insert_log(&mut *tx, &id, "submit", &auth.id, None).await?;

    if !invoice_ids.is_empty() {
        for iid in &invoice_ids {
            sqlx::query(
                "INSERT INTO reimbursement_invoices (reimbursement_id, invoice_id) VALUES (?, ?)",
            )
            .bind(&id)
            .bind(iid)
            .execute(&mut *tx)
            .await?;
        }
        sync_invoice_status(&mut *tx, &invoice_ids).await?;
    }
    tx.commit().await?;

    append_log(
        &state.config.log_file,
        &format!(
            "User {} submitted reimbursement {} ({}, {:.2} CNY)",
            user_tag(&auth.name, &auth.username),
            title,
            category,
            body.amount
        ),
        &ip,
    );

    Ok(Json(ApiResponse::created(serde_json::json!({ "id": id }))))
}

/// 编辑报销单：提交人可在待审批/已驳回/已撤回时编辑（驳回/撤回后编辑视为重新提交）；
/// 财务（reimburse_manage）可编辑任意非已付款状态。
pub async fn update_reimbursement(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateReimbursement>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let ip = client_ip.0;
    let manage = has_permission(&auth.grants, "finance:reimburse_manage");
    if !manage {
        require_permission(&auth.permissions, "finance:reimburse_create")?;
    }

    let current: Option<(String, String, String)> = sqlx::query_as(
        "SELECT employee_id, department_id, status FROM reimbursements WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((employee_id, department_id, status)) = current else {
        return Err(AppError::NotFound);
    };

    if status == "paid" {
        return Err(AppError::BadRequest("已付款的报销单不可编辑".to_string()));
    }
    if !manage {
        // 提交人仅可编辑自己的可编辑状态。
        if auth.id != employee_id || !is_owner_editable(&status) {
            return Err(AppError::Forbidden);
        }
        if !can_view_reimbursement(&state, &auth, &employee_id, &department_id).await? {
            return Err(AppError::Forbidden);
        }
    }

    if let Some(ref title) = body.title {
        let title = title.trim();
        if title.is_empty() || title.chars().count() > 128 {
            return Err(AppError::ValidationError(
                "报销事由不能为空且不能超过 128 字符".to_string(),
            ));
        }
    }
    if let Some(ref category) = body.category {
        let category = category.trim();
        if category.is_empty() || category.chars().count() > 32 {
            return Err(AppError::ValidationError(
                "费用类型不能为空且不能超过 32 字符".to_string(),
            ));
        }
    }
    if let Some(amount) = body.amount {
        validate_amount(amount)?;
    }
    if let Some(reason) = &body.reason {
        if reason.as_ref().map_or(false, |r| r.chars().count() > 1000) {
            return Err(AppError::ValidationError(
                "报销说明不能超过 1000 字符".to_string(),
            ));
        }
    }

    let mut tx = state.pool.begin().await?;
    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> =
        sqlx::QueryBuilder::new("UPDATE reimbursements SET ");
    let mut has_fields = false;
    if let Some(title) = &body.title {
        qb.push("title = ").push_bind(title.trim());
        has_fields = true;
    }
    if let Some(category) = &body.category {
        if has_fields {
            qb.push(", ");
        }
        qb.push("category = ").push_bind(category.trim());
        has_fields = true;
    }
    if let Some(amount) = body.amount {
        if has_fields {
            qb.push(", ");
        }
        qb.push("amount = ").push_bind(amount);
        has_fields = true;
    }
    if let Some(reason) = &body.reason {
        if has_fields {
            qb.push(", ");
        }
        qb.push("reason = ").push_bind(reason);
        has_fields = true;
    }
    // 提交人编辑驳回/撤回的单 → 重新提交回待审批；财务编辑不改变状态。
    if !manage && status != "pending_leader" && is_owner_editable(&status) {
        if has_fields {
            qb.push(", ");
        }
        qb.push("status = 'pending_leader'");
        has_fields = true;
    }
    if has_fields {
        qb.push(" WHERE id = ").push_bind(&id);
        qb.build().execute(&mut *tx).await?;
    }

    if let Some(ref invoice_ids) = body.invoice_ids {
        let validated = validate_invoice_ids(&state.pool, invoice_ids, Some(&id)).await?;
        // 事务内替换关联：先取旧关联，删除后插入新关联，再同步发票状态。
        let old: Vec<String> = sqlx::query_scalar(
            "SELECT invoice_id FROM reimbursement_invoices WHERE reimbursement_id = ?",
        )
        .bind(&id)
        .fetch_all(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM reimbursement_invoices WHERE reimbursement_id = ?")
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        for iid in &validated {
            sqlx::query(
                "INSERT INTO reimbursement_invoices (reimbursement_id, invoice_id) VALUES (?, ?)",
            )
            .bind(&id)
            .bind(iid)
            .execute(&mut *tx)
            .await?;
        }
        let mut affected = old;
        affected.extend(validated.iter().cloned());
        affected.sort();
        affected.dedup();
        sync_invoice_status(&mut *tx, &affected).await?;
    }

    // 提交人对驳回/撤回的单编辑 = 重新提交；其余编辑（含财务维护）记为 edit。
    let action = if !manage && is_owner_editable(&status) && status != "pending_leader" {
        "resubmit"
    } else {
        "edit"
    };
    insert_log(&mut *tx, &id, action, &auth.id, None).await?;
    tx.commit().await?;

    append_log(
        &state.config.log_file,
        &format!(
            "User {} updated reimbursement {} ({})",
            user_tag(&auth.name, &auth.username),
            id,
            action
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("保存成功")))
}

/// 删除报销单：提交人可删自己的已驳回/已撤回单；财务可删任意非已付款单。
pub async fn delete_reimbursement(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let ip = client_ip.0;
    let manage = has_permission(&auth.grants, "finance:reimburse_manage");
    if !manage {
        require_permission(&auth.permissions, "finance:reimburse_create")?;
    }

    let current: Option<(String, String, String)> = sqlx::query_as(
        "SELECT employee_id, department_id, status FROM reimbursements WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((employee_id, department_id, status)) = current else {
        return Err(AppError::NotFound);
    };

    if status == "paid" {
        return Err(AppError::BadRequest("已付款的报销单不可删除".to_string()));
    }
    if !manage {
        if auth.id != employee_id || !matches!(status.as_str(), "rejected" | "withdrawn") {
            return Err(AppError::Forbidden);
        }
        if !can_view_reimbursement(&state, &auth, &employee_id, &department_id).await? {
            return Err(AppError::Forbidden);
        }
    }

    let invoice_ids: Vec<String> =
        sqlx::query_scalar("SELECT invoice_id FROM reimbursement_invoices WHERE reimbursement_id = ?")
            .bind(&id)
            .fetch_all(&state.pool)
            .await?;

    let mut tx = state.pool.begin().await?;
    // 先删关联表（外键级联删除 logs/links），再删报销单本身。
    sqlx::query("DELETE FROM reimbursement_invoices WHERE reimbursement_id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM reimbursements WHERE id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    if !invoice_ids.is_empty() {
        sync_invoice_status(&state.pool, &invoice_ids).await?;
    }

    append_log(
        &state.config.log_file,
        &format!(
            "User {} deleted reimbursement {}",
            user_tag(&auth.name, &auth.username),
            id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("删除成功")))
}

/// 部门负责人审批：pending_leader → pending_finance（approve）/ rejected（reject）。
pub async fn approve_reimbursement(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<ReviewAction>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "finance:reimburse_approve")?;
    let ip = client_ip.0;

    let current: Option<(String, String, String)> = sqlx::query_as(
        "SELECT employee_id, department_id, status FROM reimbursements WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((employee_id, department_id, status)) = current else {
        return Err(AppError::NotFound);
    };
    if status != "pending_leader" {
        return Err(AppError::BadRequest("该报销单当前状态不可审批".to_string()));
    }
    if employee_id == auth.id {
        return Err(AppError::BadRequest("不能审批自己提交的报销单".to_string()));
    }
    let Some(scope) = build_scope(&state.pool, &auth.grants, "finance:reimburse_approve", &auth.id).await?
    else {
        return Err(AppError::Forbidden);
    };
    if !dept_in_scope(&scope, &department_id) {
        return Err(AppError::Forbidden);
    }

    let action = body.action.trim().to_lowercase();
    let comment = body.comment.as_ref().map(|c| c.trim().to_string());
    if action != "approve" && action != "reject" {
        return Err(AppError::ValidationError(
            "action 必须是 approve 或 reject".to_string(),
        ));
    }
    if action == "reject" && comment.as_ref().map_or(true, |c| c.is_empty()) {
        return Err(AppError::ValidationError(
            "驳回时必须填写审批意见".to_string(),
        ));
    }

    let new_status = if action == "approve" {
        "pending_finance"
    } else {
        "rejected"
    };
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE reimbursements SET status = ?, approver_id = ?, approve_comment = ?, \
         approved_at = NOW() WHERE id = ?",
    )
    .bind(new_status)
    .bind(&auth.id)
    .bind(&comment)
    .bind(&id)
    .execute(&mut *tx)
    .await?;
    insert_log(&mut *tx, &id, &action, &auth.id, comment.as_deref()).await?;
    tx.commit().await?;

    append_log(
        &state.config.log_file,
        &format!(
            "User {} {} reimbursement {} ({})",
            user_tag(&auth.name, &auth.username),
            action,
            id,
            comment.clone().unwrap_or_default()
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg(if action == "approve" {
        "审批通过，已转交财务复核"
    } else {
        "已驳回"
    })))
}

/// 财务复核：pending_finance → approved（approve）/ rejected（reject）。
pub async fn review_reimbursement(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<ReviewAction>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "finance:reimburse_manage")?;
    let ip = client_ip.0;

    let current: Option<(String, String)> = sqlx::query_as(
        "SELECT employee_id, status FROM reimbursements WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((employee_id, status)) = current else {
        return Err(AppError::NotFound);
    };
    if status != "pending_finance" {
        return Err(AppError::BadRequest(
            "该报销单当前状态不可复核".to_string(),
        ));
    }
    if employee_id == auth.id {
        return Err(AppError::BadRequest("不能复核自己提交的报销单".to_string()));
    }

    let action = body.action.trim().to_lowercase();
    let comment = body.comment.as_ref().map(|c| c.trim().to_string());
    if action != "approve" && action != "reject" {
        return Err(AppError::ValidationError(
            "action 必须是 approve 或 reject".to_string(),
        ));
    }
    if action == "reject" && comment.as_ref().map_or(true, |c| c.is_empty()) {
        return Err(AppError::ValidationError(
            "驳回时必须填写复核意见".to_string(),
        ));
    }

    let new_status = if action == "approve" {
        "approved"
    } else {
        "rejected"
    };
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE reimbursements SET status = ?, finance_reviewer_id = ?, finance_comment = ?, \
         finance_reviewed_at = NOW() WHERE id = ?",
    )
    .bind(new_status)
    .bind(&auth.id)
    .bind(&comment)
    .bind(&id)
    .execute(&mut *tx)
    .await?;
    insert_log(&mut *tx, &id, "review", &auth.id, comment.as_deref()).await?;
    tx.commit().await?;

    append_log(
        &state.config.log_file,
        &format!(
            "User {} finance {} reimbursement {} ({})",
            user_tag(&auth.name, &auth.username),
            action,
            id,
            comment.clone().unwrap_or_default()
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg(if action == "approve" {
        "复核通过"
    } else {
        "已驳回"
    })))
}

/// 标记付款：approved → paid，并自动生成一条支出收付款记录（全程留痕）。
pub async fn pay_reimbursement(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "finance:reimburse_manage")?;
    let ip = client_ip.0;

    let current: Option<(String, String, String, String, String, f64)> = sqlx::query_as(
        "SELECT r.employee_id, r.department_id, r.status, r.title, e.name, CAST(r.amount AS DOUBLE) \
         FROM reimbursements r JOIN employees e ON e.id = r.employee_id WHERE r.id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((_employee_id, department_id, status, title, employee_name, amount)) = current else {
        return Err(AppError::NotFound);
    };
    if status != "approved" {
        return Err(AppError::BadRequest(
            "仅已通过复核的报销单可标记付款".to_string(),
        ));
    }

    let payment_id = Uuid::new_v4().to_string();
    let today = Local::now().date_naive();

    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "UPDATE reimbursements SET status = 'paid', paid_at = NOW() WHERE id = ?",
    )
    .bind(&id)
    .execute(&mut *tx)
    .await?;
    // 自动生成支出记录（reimbursement_id 关联，预算聚合时排除避免重复计入）。
    sqlx::query(
        "INSERT INTO payments (id, direction, category, amount, counterparty, occurred_at, \
         department_id, remark, reimbursement_id, created_by) \
         VALUES (?, 'expense', '报销', ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&payment_id)
    .bind(amount)
    .bind(&employee_name)
    .bind(today)
    .bind(&department_id)
    .bind(&title)
    .bind(&id)
    .bind(&auth.id)
    .execute(&mut *tx)
    .await?;
    insert_log(&mut *tx, &id, "pay", &auth.id, None).await?;
    tx.commit().await?;

    append_log(
        &state.config.log_file,
        &format!(
            "User {} marked reimbursement {} as paid ({:.2} CNY)",
            user_tag(&auth.name, &auth.username),
            id,
            amount
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("已标记付款")))
}

/// 撤回：提交人可在待审批/待财务复核时撤回；财务可代为撤回。
pub async fn withdraw_reimbursement(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let ip = client_ip.0;
    let manage = has_permission(&auth.grants, "finance:reimburse_manage");
    if !manage {
        require_permission(&auth.permissions, "finance:reimburse_create")?;
    }

    let current: Option<(String, String, String)> = sqlx::query_as(
        "SELECT employee_id, department_id, status FROM reimbursements WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((employee_id, department_id, status)) = current else {
        return Err(AppError::NotFound);
    };
    if !matches!(status.as_str(), "pending_leader" | "pending_finance") {
        return Err(AppError::BadRequest("当前状态不可撤回".to_string()));
    }
    if !manage && auth.id != employee_id {
        return Err(AppError::Forbidden);
    }
    if !can_view_reimbursement(&state, &auth, &employee_id, &department_id).await? {
        return Err(AppError::Forbidden);
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query("UPDATE reimbursements SET status = 'withdrawn' WHERE id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    insert_log(&mut *tx, &id, "withdraw", &auth.id, None).await?;
    tx.commit().await?;

    append_log(
        &state.config.log_file,
        &format!(
            "User {} withdrew reimbursement {}",
            user_tag(&auth.name, &auth.username),
            id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("已撤回")))
}

// ============================================================================
// 发票管理
// ============================================================================

/// 发票列表。
pub async fn list_invoices(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<InvoiceQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "finance:invoice_manage")?;

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT i.id, i.invoice_code, i.invoice_type, CAST(i.amount AS DOUBLE) AS amount, \
         CAST(i.tax_amount AS DOUBLE) AS tax_amount, i.issued_at, i.issuer_name, i.buyer_name, \
         i.image_url, i.employee_id, e.name AS employee_name, i.status, i.created_at \
         FROM invoices i JOIN employees e ON e.id = i.employee_id WHERE 1 = 1",
    );
    if let Some(status) = &query.status {
        if !status.is_empty() {
            qb.push(" AND i.status = ").push_bind(status);
        }
    }
    if let Some(keyword) = &query.keyword {
        let kw = keyword.trim();
        if !kw.is_empty() {
            qb.push(" AND (i.invoice_code LIKE ")
                .push_bind(format!("%{}%", kw))
                .push(" OR i.issuer_name LIKE ")
                .push_bind(format!("%{}%", kw))
                .push(")");
        }
    }
    qb.push(" ORDER BY i.created_at DESC LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind((page - 1) * page_size);
    let items: Vec<InvoiceRow> = qb.build_query_as().fetch_all(&state.pool).await?;

    let mut cqb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT COUNT(*) FROM invoices i JOIN employees e ON e.id = i.employee_id WHERE 1 = 1",
    );
    if let Some(status) = &query.status {
        if !status.is_empty() {
            cqb.push(" AND i.status = ").push_bind(status);
        }
    }
    if let Some(keyword) = &query.keyword {
        let kw = keyword.trim();
        if !kw.is_empty() {
            cqb.push(" AND (i.invoice_code LIKE ")
                .push_bind(format!("%{}%", kw))
                .push(" OR i.issuer_name LIKE ")
                .push_bind(format!("%{}%", kw))
                .push(")");
        }
    }
    let total: i64 = cqb.build_query_scalar().fetch_one(&state.pool).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// 录入发票（发票号码唯一查重）。
pub async fn create_invoice(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Json(body): Json<NewInvoice>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "finance:invoice_manage")?;
    let ip = client_ip.0;

    let code = body.invoice_code.trim().to_string();
    if code.is_empty() || code.chars().count() > 64 {
        return Err(AppError::ValidationError(
            "发票号码不能为空且不能超过 64 字符".to_string(),
        ));
    }
    validate_amount(body.amount)?;
    if let Some(tax) = body.tax_amount {
        if !tax.is_finite() || tax < 0.0 || tax > body.amount {
            return Err(AppError::ValidationError(
                "税额必须为非负数且不超过价税合计".to_string(),
            ));
        }
    }
    let issuer = body.issuer_name.trim().to_string();
    if issuer.is_empty() || issuer.chars().count() > 128 {
        return Err(AppError::ValidationError(
            "开票方名称不能为空且不能超过 128 字符".to_string(),
        ));
    }
    let invoice_type = body
        .invoice_type
        .as_deref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "普通发票".to_string());

    // 查重：发票号码唯一。
    let exists: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM invoices WHERE invoice_code = ?")
            .bind(&code)
            .fetch_one(&state.pool)
            .await?;
    if exists > 0 {
        return Err(AppError::BadRequest("发票号码已存在，请勿重复录入".to_string()));
    }

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO invoices (id, invoice_code, invoice_type, amount, tax_amount, issued_at, \
         issuer_name, buyer_name, image_url, employee_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&code)
    .bind(&invoice_type)
    .bind(body.amount)
    .bind(body.tax_amount)
    .bind(body.issued_at)
    .bind(&issuer)
    .bind(&body.buyer_name)
    .bind(&body.image_url)
    .bind(&auth.id)
    .execute(&state.pool)
    .await?;

    append_log(
        &state.config.log_file,
        &format!(
            "User {} created invoice {} ({})",
            user_tag(&auth.name, &auth.username),
            code,
            issuer
        ),
        &ip,
    );

    Ok(Json(ApiResponse::created(serde_json::json!({ "id": id }))))
}

/// 更新发票（已关联报销单的发票不可修改号码/金额等核心字段）。
pub async fn update_invoice(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateInvoice>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "finance:invoice_manage")?;
    let ip = client_ip.0;

    let current: Option<(String, String)> =
        sqlx::query_as("SELECT invoice_code, status FROM invoices WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.pool)
            .await?;
    let Some((old_code, status)) = current else {
        return Err(AppError::NotFound);
    };
    if status == "claimed" {
        return Err(AppError::BadRequest(
            "该发票已关联报销单，不可修改；如需调整请先在报销单中移除".to_string(),
        ));
    }

    let new_code = body
        .invoice_code
        .as_deref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(old_code.clone());
    if new_code.chars().count() > 64 {
        return Err(AppError::ValidationError(
            "发票号码不能超过 64 字符".to_string(),
        ));
    }
    if new_code != old_code {
        let exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM invoices WHERE invoice_code = ? AND id != ?")
                .bind(&new_code)
                .bind(&id)
                .fetch_one(&state.pool)
                .await?;
        if exists > 0 {
            return Err(AppError::BadRequest("发票号码已存在，请勿重复录入".to_string()));
        }
    }
    if let Some(amount) = body.amount {
        validate_amount(amount)?;
    }
    if let Some(tax) = body.tax_amount {
        if let Some(tax) = tax {
            let amount = body.amount.unwrap_or_else(|| {
                // 不重复查询：从旧值兜底（仅当未同时更新金额时）。
                0.0
            });
            if amount > 0.0 && (tax < 0.0 || tax > amount) {
                return Err(AppError::ValidationError(
                    "税额必须为非负数且不超过价税合计".to_string(),
                ));
            }
        }
    }
    if let Some(issuer) = &body.issuer_name {
        let issuer = issuer.trim();
        if issuer.is_empty() || issuer.chars().count() > 128 {
            return Err(AppError::ValidationError(
                "开票方名称不能为空且不能超过 128 字符".to_string(),
            ));
        }
    }

    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> =
        sqlx::QueryBuilder::new("UPDATE invoices SET ");
    let mut has_fields = false;
    if let Some(code) = &body.invoice_code {
        qb.push("invoice_code = ").push_bind(code.trim());
        has_fields = true;
    }
    if let Some(t) = &body.invoice_type {
        let t = t.trim();
        if !t.is_empty() {
            if has_fields {
                qb.push(", ");
            }
            qb.push("invoice_type = ").push_bind(t);
            has_fields = true;
        }
    }
    if let Some(amount) = body.amount {
        if has_fields {
            qb.push(", ");
        }
        qb.push("amount = ").push_bind(amount);
        has_fields = true;
    }
    if let Some(tax) = body.tax_amount {
        if has_fields {
            qb.push(", ");
        }
        qb.push("tax_amount = ").push_bind(tax);
        has_fields = true;
    }
    if let Some(issued) = body.issued_at {
        if has_fields {
            qb.push(", ");
        }
        qb.push("issued_at = ").push_bind(issued);
        has_fields = true;
    }
    if let Some(issuer) = &body.issuer_name {
        if has_fields {
            qb.push(", ");
        }
        qb.push("issuer_name = ").push_bind(issuer.trim());
        has_fields = true;
    }
    if let Some(buyer) = &body.buyer_name {
        if has_fields {
            qb.push(", ");
        }
        qb.push("buyer_name = ").push_bind(buyer);
        has_fields = true;
    }
    if let Some(image) = &body.image_url {
        if has_fields {
            qb.push(", ");
        }
        qb.push("image_url = ").push_bind(image);
        has_fields = true;
    }
    if has_fields {
        qb.push(" WHERE id = ").push_bind(&id);
        qb.build().execute(&state.pool).await?;
    }

    append_log(
        &state.config.log_file,
        &format!(
            "User {} updated invoice {} ({})",
            user_tag(&auth.name, &auth.username),
            id,
            new_code
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("保存成功")))
}

/// 删除发票（仅未关联报销单的发票可删除）。
pub async fn delete_invoice(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "finance:invoice_manage")?;
    let ip = client_ip.0;

    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM invoices WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.pool)
            .await?;
    let Some(status) = status else {
        return Err(AppError::NotFound);
    };
    if status == "claimed" {
        return Err(AppError::BadRequest(
            "该发票已关联报销单，不可删除".to_string(),
        ));
    }
    sqlx::query("DELETE FROM invoices WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;

    append_log(
        &state.config.log_file,
        &format!(
            "User {} deleted invoice {}",
            user_tag(&auth.name, &auth.username),
            id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("删除成功")))
}

// ============================================================================
// 收付款记录
// ============================================================================

fn validate_direction(direction: &str) -> Result<(), AppError> {
    if direction != "income" && direction != "expense" {
        return Err(AppError::ValidationError(
            "direction 必须是 income（收款）或 expense（付款）".to_string(),
        ));
    }
    Ok(())
}

/// 收付款记录列表。
pub async fn list_payments(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<PaymentQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "finance:payment_manage")?;

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT p.id, p.direction, p.category, CAST(p.amount AS DOUBLE) AS amount, \
         p.counterparty, p.occurred_at, p.department_id, d.name AS department_name, \
         p.remark, p.reimbursement_id, p.created_by, e.name AS creator_name, p.created_at \
         FROM payments p \
         LEFT JOIN departments d ON d.id = p.department_id \
         JOIN employees e ON e.id = p.created_by \
         WHERE 1 = 1",
    );
    if let Some(direction) = &query.direction {
        if !direction.is_empty() {
            qb.push(" AND p.direction = ").push_bind(direction);
        }
    }
    if let Some(dept) = &query.department_id {
        if !dept.is_empty() {
            qb.push(" AND p.department_id = ").push_bind(dept);
        }
    }
    if let Some(from) = query.from {
        qb.push(" AND p.occurred_at >= ").push_bind(from);
    }
    if let Some(to) = query.to {
        qb.push(" AND p.occurred_at <= ").push_bind(to);
    }
    if let Some(keyword) = &query.keyword {
        let kw = keyword.trim();
        if !kw.is_empty() {
            qb.push(" AND (p.category LIKE ")
                .push_bind(format!("%{}%", kw))
                .push(" OR p.counterparty LIKE ")
                .push_bind(format!("%{}%", kw))
                .push(" OR p.remark LIKE ")
                .push_bind(format!("%{}%", kw))
                .push(")");
        }
    }
    qb.push(" ORDER BY p.occurred_at DESC, p.created_at DESC LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind((page - 1) * page_size);
    let items: Vec<PaymentRow> = qb.build_query_as().fetch_all(&state.pool).await?;

    let mut cqb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT COUNT(*) FROM payments p LEFT JOIN departments d ON d.id = p.department_id \
         JOIN employees e ON e.id = p.created_by WHERE 1 = 1",
    );
    if let Some(direction) = &query.direction {
        if !direction.is_empty() {
            cqb.push(" AND p.direction = ").push_bind(direction);
        }
    }
    if let Some(dept) = &query.department_id {
        if !dept.is_empty() {
            cqb.push(" AND p.department_id = ").push_bind(dept);
        }
    }
    if let Some(from) = query.from {
        cqb.push(" AND p.occurred_at >= ").push_bind(from);
    }
    if let Some(to) = query.to {
        cqb.push(" AND p.occurred_at <= ").push_bind(to);
    }
    if let Some(keyword) = &query.keyword {
        let kw = keyword.trim();
        if !kw.is_empty() {
            cqb.push(" AND (p.category LIKE ")
                .push_bind(format!("%{}%", kw))
                .push(" OR p.counterparty LIKE ")
                .push_bind(format!("%{}%", kw))
                .push(" OR p.remark LIKE ")
                .push_bind(format!("%{}%", kw))
                .push(")");
        }
    }
    let total: i64 = cqb.build_query_scalar().fetch_one(&state.pool).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// 新增收付款记录。
pub async fn create_payment(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Json(body): Json<NewPayment>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "finance:payment_manage")?;
    let ip = client_ip.0;

    validate_direction(&body.direction)?;
    let category = body.category.trim().to_string();
    if category.is_empty() || category.chars().count() > 32 {
        return Err(AppError::ValidationError(
            "收支类别不能为空且不能超过 32 字符".to_string(),
        ));
    }
    validate_amount(body.amount)?;
    if let Some(cp) = &body.counterparty {
        if cp.chars().count() > 128 {
            return Err(AppError::ValidationError(
                "往来方名称不能超过 128 字符".to_string(),
            ));
        }
    }
    if let Some(dept) = &body.department_id {
        if !dept.is_empty() {
            let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE id = ?")
                .bind(dept)
                .fetch_one(&state.pool)
                .await?;
            if n == 0 {
                return Err(AppError::BadRequest("指定的部门不存在".to_string()));
            }
        }
    }

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO payments (id, direction, category, amount, counterparty, occurred_at, \
         department_id, remark, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&body.direction)
    .bind(&category)
    .bind(body.amount)
    .bind(&body.counterparty)
    .bind(body.occurred_at)
    .bind(body.department_id.as_deref().filter(|d| !d.is_empty()))
    .bind(&body.remark)
    .bind(&auth.id)
    .execute(&state.pool)
    .await?;

    append_log(
        &state.config.log_file,
        &format!(
            "User {} created payment {} {} ({:.2} CNY)",
            user_tag(&auth.name, &auth.username),
            body.direction,
            category,
            body.amount
        ),
        &ip,
    );

    Ok(Json(ApiResponse::created(serde_json::json!({ "id": id }))))
}

/// 更新收付款记录（报销自动生成的记录仅财务可改备注等）。
pub async fn update_payment(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdatePayment>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "finance:payment_manage")?;
    let ip = client_ip.0;

    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM payments WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }
    if let Some(direction) = &body.direction {
        validate_direction(direction)?;
    }
    if let Some(amount) = body.amount {
        validate_amount(amount)?;
    }
    if let Some(category) = &body.category {
        let category = category.trim();
        if category.is_empty() || category.chars().count() > 32 {
            return Err(AppError::ValidationError(
                "收支类别不能为空且不能超过 32 字符".to_string(),
            ));
        }
    }
    if let Some(dept) = &body.department_id {
        if let Some(dept) = dept {
            if !dept.is_empty() {
                let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE id = ?")
                    .bind(dept)
                    .fetch_one(&state.pool)
                    .await?;
                if n == 0 {
                    return Err(AppError::BadRequest("指定的部门不存在".to_string()));
                }
            }
        }
    }

    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> =
        sqlx::QueryBuilder::new("UPDATE payments SET ");
    let mut has_fields = false;
    if let Some(direction) = &body.direction {
        qb.push("direction = ").push_bind(direction);
        has_fields = true;
    }
    if let Some(category) = &body.category {
        if has_fields {
            qb.push(", ");
        }
        qb.push("category = ").push_bind(category.trim());
        has_fields = true;
    }
    if let Some(amount) = body.amount {
        if has_fields {
            qb.push(", ");
        }
        qb.push("amount = ").push_bind(amount);
        has_fields = true;
    }
    if let Some(counterparty) = &body.counterparty {
        if has_fields {
            qb.push(", ");
        }
        qb.push("counterparty = ").push_bind(counterparty);
        has_fields = true;
    }
    if let Some(occurred) = body.occurred_at {
        if has_fields {
            qb.push(", ");
        }
        qb.push("occurred_at = ").push_bind(occurred);
        has_fields = true;
    }
    if let Some(dept) = &body.department_id {
        if has_fields {
            qb.push(", ");
        }
        qb.push("department_id = ").push_bind(dept);
        has_fields = true;
    }
    if let Some(remark) = &body.remark {
        if has_fields {
            qb.push(", ");
        }
        qb.push("remark = ").push_bind(remark);
        has_fields = true;
    }
    if has_fields {
        qb.push(" WHERE id = ").push_bind(&id);
        qb.build().execute(&state.pool).await?;
    }

    append_log(
        &state.config.log_file,
        &format!(
            "User {} updated payment {}",
            user_tag(&auth.name, &auth.username),
            id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("保存成功")))
}

/// 删除收付款记录。
pub async fn delete_payment(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "finance:payment_manage")?;
    let ip = client_ip.0;

    let result = sqlx::query("DELETE FROM payments WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    append_log(
        &state.config.log_file,
        &format!(
            "User {} deleted payment {}",
            user_tag(&auth.name, &auth.username),
            id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("删除成功")))
}

// ============================================================================
// 预算管理
// ============================================================================

fn validate_period(period_type: &str, period_value: &str) -> Result<(), AppError> {
    match period_type {
        "year" => {
            if period_value.len() == 4 && period_value.chars().all(|c| c.is_ascii_digit()) {
                Ok(())
            } else {
                Err(AppError::ValidationError(
                    "年度预算期间格式应为 4 位年份（如 2025）".to_string(),
                ))
            }
        }
        "month" => {
            let valid = period_value.len() == 7
                && period_value.as_bytes().get(4) == Some(&b'-')
                && period_value[..4].chars().all(|c| c.is_ascii_digit())
                && period_value[5..].chars().all(|c| c.is_ascii_digit());
            if valid {
                Ok(())
            } else {
                Err(AppError::ValidationError(
                    "月度预算期间格式应为 YYYY-MM（如 2025-06）".to_string(),
                ))
            }
        }
        _ => Err(AppError::ValidationError(
            "period_type 必须是 year 或 month".to_string(),
        )),
    }
}

/// 预算期间与业务日期是否匹配。
fn period_matches_sql(period_type: &str, period_value: &str, date_expr: &str) -> String {
    match period_type {
        "year" => format!("YEAR({}) = {}", date_expr, period_value),
        _ => format!("DATE_FORMAT({}, '%Y-%m') = '{}'", date_expr, period_value),
    }
}

/// 预算列表（含已用额：已通过/已付款报销 + 非报销关联的支出付款）。
pub async fn list_budgets(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<BudgetQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "finance:budget_manage")?;

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT b.id, b.department_id, d.name AS department_name, b.period_type, \
         b.period_value, CAST(b.amount AS DOUBLE) AS amount, b.created_at \
         FROM budgets b JOIN departments d ON d.id = b.department_id WHERE 1 = 1",
    );
    if let Some(pt) = &query.period_type {
        if !pt.is_empty() {
            qb.push(" AND b.period_type = ").push_bind(pt);
        }
    }
    if let Some(pv) = &query.period_value {
        if !pv.is_empty() {
            qb.push(" AND b.period_value = ").push_bind(pv);
        }
    }
    if let Some(dept) = &query.department_id {
        if !dept.is_empty() {
            qb.push(" AND b.department_id = ").push_bind(dept);
        }
    }
    qb.push(" ORDER BY b.period_value DESC, d.name LIMIT ")
        .push_bind(page_size)
        .push(" OFFSET ")
        .push_bind((page - 1) * page_size);
    let raw: Vec<BudgetRawRow> = qb.build_query_as().fetch_all(&state.pool).await?;

    // 逐行聚合已用额（预算行量级小，逐行两次聚合可接受）。
    let mut items: Vec<serde_json::Value> = Vec::new();
    for row in raw {
        let spent_reimb: f64 = sqlx::query_scalar(&format!(
            "SELECT CAST(COALESCE(SUM(r.amount), 0) AS DOUBLE) FROM reimbursements r \
             WHERE r.department_id = ? AND r.status IN ('approved', 'paid') AND {}",
            period_matches_sql(&row.period_type, &row.period_value, "r.created_at")
        ))
        .bind(&row.department_id)
        .fetch_one(&state.pool)
        .await?;
        let spent_payment: f64 = sqlx::query_scalar(&format!(
            "SELECT CAST(COALESCE(SUM(p.amount), 0) AS DOUBLE) FROM payments p \
             WHERE p.direction = 'expense' AND p.department_id = ? AND p.reimbursement_id IS NULL AND {}",
            period_matches_sql(&row.period_type, &row.period_value, "p.occurred_at")
        ))
        .bind(&row.department_id)
        .fetch_one(&state.pool)
        .await?;
        let spent = spent_reimb + spent_payment;
        items.push(serde_json::json!({
            "id": row.id,
            "department_id": row.department_id,
            "department_name": row.department_name,
            "period_type": row.period_type,
            "period_value": row.period_value,
            "amount": row.amount,
            "spent": spent,
            "remaining": row.amount - spent,
            "created_at": row.created_at,
        }));
    }

    let mut cqb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT COUNT(*) FROM budgets b JOIN departments d ON d.id = b.department_id WHERE 1 = 1",
    );
    if let Some(pt) = &query.period_type {
        if !pt.is_empty() {
            cqb.push(" AND b.period_type = ").push_bind(pt);
        }
    }
    if let Some(pv) = &query.period_value {
        if !pv.is_empty() {
            cqb.push(" AND b.period_value = ").push_bind(pv);
        }
    }
    if let Some(dept) = &query.department_id {
        if !dept.is_empty() {
            cqb.push(" AND b.department_id = ").push_bind(dept);
        }
    }
    let total: i64 = cqb.build_query_scalar().fetch_one(&state.pool).await?;

    Ok(Json(ApiResponse::ok(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// 新增预算（同部门+期间唯一，重复创建返回冲突）。
pub async fn create_budget(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Json(body): Json<NewBudget>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "finance:budget_manage")?;
    let ip = client_ip.0;

    validate_period(&body.period_type, &body.period_value)?;
    validate_amount(body.amount)?;
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE id = ?")
        .bind(&body.department_id)
        .fetch_one(&state.pool)
        .await?;
    if n == 0 {
        return Err(AppError::BadRequest("指定的部门不存在".to_string()));
    }
    let dup: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM budgets WHERE department_id = ? AND period_type = ? AND period_value = ?",
    )
    .bind(&body.department_id)
    .bind(&body.period_type)
    .bind(&body.period_value)
    .fetch_one(&state.pool)
    .await?;
    if dup > 0 {
        return Err(AppError::BadRequest(
            "该部门该期间已存在预算，请直接编辑或删除后重建".to_string(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO budgets (id, department_id, period_type, period_value, amount) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&body.department_id)
    .bind(&body.period_type)
    .bind(&body.period_value)
    .bind(body.amount)
    .execute(&state.pool)
    .await?;

    append_log(
        &state.config.log_file,
        &format!(
            "User {} created budget {}-{} for dept {} ({:.2} CNY)",
            user_tag(&auth.name, &auth.username),
            body.period_type,
            body.period_value,
            body.department_id,
            body.amount
        ),
        &ip,
    );

    Ok(Json(ApiResponse::created(serde_json::json!({ "id": id }))))
}

/// 更新预算。
pub async fn update_budget(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateBudget>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "finance:budget_manage")?;
    let ip = client_ip.0;

    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM budgets WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }

    // 期间唯一性校验：取最终生效的 (period_type, period_value, department_id)。
    let current: Option<(String, String, String)> = sqlx::query_as(
        "SELECT period_type, period_value, department_id FROM budgets WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await?;
    let Some((cur_pt, cur_pv, cur_dept)) = current else {
        return Err(AppError::NotFound);
    };
    let final_pt = body.period_type.clone().unwrap_or(cur_pt.clone());
    let final_pv = body.period_value.clone().unwrap_or(cur_pv.clone());
    let final_dept = body
        .department_id
        .clone()
        .unwrap_or_else(|| cur_dept.clone());
    if body.period_type.is_some() || body.period_value.is_some() {
        validate_period(&final_pt, &final_pv)?;
    }
    if !final_dept.is_empty() {
        let dup: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM budgets WHERE department_id = ? AND period_type = ? \
             AND period_value = ? AND id != ?",
        )
        .bind(&final_dept)
        .bind(&final_pt)
        .bind(&final_pv)
        .bind(&id)
        .fetch_one(&state.pool)
        .await?;
        if dup > 0 {
            return Err(AppError::BadRequest(
                "该部门该期间已存在预算，请直接编辑或删除后重建".to_string(),
            ));
        }
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM departments WHERE id = ?")
            .bind(&final_dept)
            .fetch_one(&state.pool)
            .await?;
        if n == 0 {
            return Err(AppError::BadRequest("指定的部门不存在".to_string()));
        }
    }
    if let Some(amount) = body.amount {
        validate_amount(amount)?;
    }

    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> =
        sqlx::QueryBuilder::new("UPDATE budgets SET ");
    let mut has_fields = false;
    if let Some(dept) = &body.department_id {
        qb.push("department_id = ").push_bind(dept);
        has_fields = true;
    }
    if let Some(pt) = &body.period_type {
        if has_fields {
            qb.push(", ");
        }
        qb.push("period_type = ").push_bind(pt);
        has_fields = true;
    }
    if let Some(pv) = &body.period_value {
        if has_fields {
            qb.push(", ");
        }
        qb.push("period_value = ").push_bind(pv);
        has_fields = true;
    }
    if let Some(amount) = body.amount {
        if has_fields {
            qb.push(", ");
        }
        qb.push("amount = ").push_bind(amount);
        has_fields = true;
    }
    if has_fields {
        qb.push(" WHERE id = ").push_bind(&id);
        qb.build().execute(&state.pool).await?;
    }

    append_log(
        &state.config.log_file,
        &format!(
            "User {} updated budget {}",
            user_tag(&auth.name, &auth.username),
            id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("保存成功")))
}

/// 删除预算。
pub async fn delete_budget(
    State(state): State<AppState>,
    client_ip: ClientIp,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_permission(&auth.permissions, "finance:budget_manage")?;
    let ip = client_ip.0;

    let result = sqlx::query("DELETE FROM budgets WHERE id = ?")
        .bind(&id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    append_log(
        &state.config.log_file,
        &format!(
            "User {} deleted budget {}",
            user_tag(&auth.name, &auth.username),
            id
        ),
        &ip,
    );

    Ok(Json(ApiResponse::ok_msg("删除成功")))
}

// ============================================================================
// 财务报表
// ============================================================================

/// 期间内收/支汇总与待付报销统计。
pub async fn report_summary(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ReportQuery>,
) -> Result<Json<ApiResponse<ReportSummary>>, AppError> {
    require_permission(&auth.permissions, "finance:report_view")?;

    let from = query.from;
    let to = query.to;

    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT CAST(COALESCE(SUM(CASE WHEN direction = 'income' THEN amount ELSE 0 END), 0) AS DOUBLE), \
         CAST(COALESCE(SUM(CASE WHEN direction = 'expense' THEN amount ELSE 0 END), 0) AS DOUBLE), \
         COUNT(CASE WHEN direction = 'income' THEN 1 END), \
         COUNT(CASE WHEN direction = 'expense' THEN 1 END) \
         FROM payments WHERE 1 = 1",
    );
    if let Some(f) = from {
        qb.push(" AND occurred_at >= ").push_bind(f);
    }
    if let Some(t) = to {
        qb.push(" AND occurred_at <= ").push_bind(t);
    }
    let (income, expense, income_count, expense_count): (f64, f64, i64, i64) = qb
        .build_query_as()
        .fetch_one(&state.pool)
        .await?;

    let mut rqb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT CAST(COALESCE(SUM(amount), 0) AS DOUBLE), COUNT(*) FROM reimbursements \
         WHERE status IN ('pending_leader', 'pending_finance', 'approved')",
    );
    if let Some(f) = from {
        rqb.push(" AND created_at >= ").push_bind(f);
    }
    if let Some(t) = to {
        rqb.push(" AND created_at <= ").push_bind(t);
    }
    let (reimb_pending, reimb_count): (f64, i64) = rqb
        .build_query_as()
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(ApiResponse::ok(ReportSummary {
        income,
        expense,
        net: income - expense,
        income_count,
        expense_count,
        reimbursement_pending: reimb_pending,
        reimbursement_pending_count: reimb_count,
    })))
}

/// 部门费用排行（支出付款 + 已通过/已付款报销）。
pub async fn report_departments(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ReportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "finance:report_view")?;

    let from = query.from;
    let to = query.to;

    let mut pqb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT p.department_id, d.name AS department_name, \
         CAST(COALESCE(SUM(p.amount), 0) AS DOUBLE) AS expense \
         FROM payments p LEFT JOIN departments d ON d.id = p.department_id \
         WHERE p.direction = 'expense'",
    );
    if let Some(f) = from {
        pqb.push(" AND p.occurred_at >= ").push_bind(f);
    }
    if let Some(t) = to {
        pqb.push(" AND p.occurred_at <= ").push_bind(t);
    }
    pqb.push(" GROUP BY p.department_id, d.name");
    let payment_rows: Vec<(Option<String>, Option<String>, f64)> = pqb
        .build_query_as()
        .fetch_all(&state.pool)
        .await?;

    let mut rqb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT r.department_id, d.name AS department_name, \
         CAST(COALESCE(SUM(r.amount), 0) AS DOUBLE) AS expense \
         FROM reimbursements r JOIN departments d ON d.id = r.department_id \
         WHERE r.status IN ('approved', 'paid')",
    );
    if let Some(f) = from {
        rqb.push(" AND r.created_at >= ").push_bind(f);
    }
    if let Some(t) = to {
        rqb.push(" AND r.created_at <= ").push_bind(t);
    }
    rqb.push(" GROUP BY r.department_id, d.name");
    let reimb_rows: Vec<(String, String, f64)> = rqb
        .build_query_as()
        .fetch_all(&state.pool)
        .await?;

    let mut map: std::collections::BTreeMap<String, (String, f64)> = std::collections::BTreeMap::new();
    for (did, name, expense) in payment_rows {
        let key = did.unwrap_or_default();
        let entry = map
            .entry(key)
            .or_insert_with(|| (name.clone().unwrap_or_else(|| "未指定部门".to_string()), 0.0));
        entry.1 += expense;
    }
    for (did, name, expense) in reimb_rows {
        let entry = map.entry(did).or_insert_with(|| (name, 0.0));
        entry.1 += expense;
    }

    let mut rows: Vec<DepartmentReportRow> = map
        .into_iter()
        .map(|(department_id, (department_name, expense))| DepartmentReportRow {
            department_id,
            department_name,
            expense,
        })
        .collect();
    rows.sort_by(|a, b| b.expense.partial_cmp(&a.expense).unwrap_or(std::cmp::Ordering::Equal));

    let total_expense: f64 = rows.iter().map(|r| r.expense).sum();
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "items": rows,
        "total_expense": total_expense,
    }))))
}

/// 收支趋势（按月/年聚合收付款）。
pub async fn report_trend(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ReportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    require_permission(&auth.permissions, "finance:report_view")?;

    let from = query.from;
    let to = query.to;
    let granularity = query.granularity.as_deref().unwrap_or("month");
    let fmt = if granularity == "year" { "%Y" } else { "%Y-%m" };

    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(&format!(
        "SELECT DATE_FORMAT(occurred_at, '{}') AS period, \
         CAST(COALESCE(SUM(CASE WHEN direction = 'income' THEN amount ELSE 0 END), 0) AS DOUBLE), \
         CAST(COALESCE(SUM(CASE WHEN direction = 'expense' THEN amount ELSE 0 END), 0) AS DOUBLE) \
         FROM payments WHERE 1 = 1",
        fmt
    ));
    if let Some(f) = from {
        qb.push(" AND occurred_at >= ").push_bind(f);
    }
    if let Some(t) = to {
        qb.push(" AND occurred_at <= ").push_bind(t);
    }
    qb.push(" GROUP BY DATE_FORMAT(occurred_at, '").push(fmt.to_string()).push("') ORDER BY period");
    let rows: Vec<(String, f64, f64)> = qb.build_query_as().fetch_all(&state.pool).await?;

    let items: Vec<TrendRow> = rows
        .into_iter()
        .map(|(period, income, expense)| TrendRow {
            period,
            income,
            expense,
        })
        .collect();

    Ok(Json(ApiResponse::ok(serde_json::json!({ "items": items }))))
}

/// CSV 单元格转义（引号包裹 + 内部引号翻倍）。
///
/// F-26: CSV 公式注入（CWE-1236）防护——Excel / Google Sheets 会把以 `= + - @`
/// 或制表符/回车开头的单元格当作公式执行（用户可控内容如报销事由、备注可构造
/// `=HYPERLINK(...)` / `=CMD(...)` 等恶意公式，导出后在财务人员本机触发）。
/// 统一策略：剥离行首制表符/回车后，若首个有效字符命中危险字符集，则前缀单引号
/// （Excel 将 `'` 开头的单元格视为纯文本），其余单元格保持原样。
fn csv_cell(value: &str) -> String {
    let trimmed = value.trim_start_matches(['\t', '\r']);
    let mut escaped = trimmed.replace('"', "\"\"");
    if matches!(
        escaped.chars().next(),
        Some('=') | Some('+') | Some('-') | Some('@')
    ) {
        escaped = format!("'{}", escaped);
    }
    format!("\"{}\"", escaped)
}

/// 报销单 CSV 导出。
pub async fn export_reimbursements(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ReportQuery>,
) -> Result<Response, AppError> {
    require_permission(&auth.permissions, "finance:report_view")?;
    let from = query.from;
    let to = query.to;

    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT r.title, e.name, d.name, r.category, CAST(r.amount AS DOUBLE), r.currency, \
         r.status, r.created_at \
         FROM reimbursements r \
         JOIN employees e ON e.id = r.employee_id \
         JOIN departments d ON d.id = r.department_id \
         WHERE 1 = 1",
    );
    if let Some(f) = from {
        qb.push(" AND r.created_at >= ").push_bind(f);
    }
    if let Some(t) = to {
        qb.push(" AND r.created_at <= ").push_bind(t);
    }
    qb.push(" ORDER BY r.created_at DESC");
    let rows: Vec<(String, String, String, String, f64, String, String, chrono::NaiveDateTime)> =
        qb.build_query_as().fetch_all(&state.pool).await?;

    let mut csv = String::from("\u{feff}");
    csv.push_str("事由,提交人,部门,费用类型,金额,币种,状态,提交时间\r\n");
    for (title, name, dept, category, amount, currency, status, created_at) in rows {
        csv.push_str(&format!(
            "{},{},{},{},{:.2},{},{},{}\r\n",
            csv_cell(&title),
            csv_cell(&name),
            csv_cell(&dept),
            csv_cell(&category),
            amount,
            csv_cell(&currency),
            csv_cell(&status),
            created_at.format("%Y-%m-%d %H:%M:%S")
        ));
    }

    Ok(csv_response("reimbursements.csv", &csv))
}

/// 收付款记录 CSV 导出。
pub async fn export_payments(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ReportQuery>,
) -> Result<Response, AppError> {
    require_permission(&auth.permissions, "finance:report_view")?;
    let from = query.from;
    let to = query.to;

    let mut qb: sqlx::QueryBuilder<'_, sqlx::MySql> = sqlx::QueryBuilder::new(
        "SELECT p.direction, p.category, CAST(p.amount AS DOUBLE), p.counterparty, \
         p.occurred_at, d.name, p.remark \
         FROM payments p LEFT JOIN departments d ON d.id = p.department_id \
         WHERE 1 = 1",
    );
    if let Some(f) = from {
        qb.push(" AND p.occurred_at >= ").push_bind(f);
    }
    if let Some(t) = to {
        qb.push(" AND p.occurred_at <= ").push_bind(t);
    }
    qb.push(" ORDER BY p.occurred_at DESC");
    let rows: Vec<(String, String, f64, Option<String>, NaiveDate, Option<String>, Option<String>)> =
        qb.build_query_as().fetch_all(&state.pool).await?;

    let mut csv = String::from("\u{feff}");
    csv.push_str("方向,类别,金额,往来方,业务日期,部门,备注\r\n");
    for (direction, category, amount, counterparty, occurred_at, dept, remark) in rows {
        csv.push_str(&format!(
            "{},{},{:.2},{},{},{},{}\r\n",
            csv_cell(if direction == "income" { "收款" } else { "付款" }),
            csv_cell(&category),
            amount,
            csv_cell(&counterparty.unwrap_or_default()),
            occurred_at.format("%Y-%m-%d"),
            csv_cell(&dept.unwrap_or_default()),
            csv_cell(&remark.unwrap_or_default())
        ));
    }

    Ok(csv_response("payments.csv", &csv))
}

fn csv_response(filename: &str, csv: &str) -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(axum::body::Body::from(csv.to_string()))
        .expect("csv response")
}
