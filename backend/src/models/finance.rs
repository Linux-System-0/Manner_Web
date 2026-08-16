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

//! 财务模块数据模型：报销单 / 发票 / 收付款记录 / 预算。
//! 金额列在库中为 DECIMAL(12,2)，读取时经 `CAST(... AS DOUBLE)` 映射为 f64
//! （sqlx 不对 DECIMAL 直接解码 f64），写入时绑定 f64（MySQL 隐式转换）。

use chrono::NaiveDate;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ---- 报销单 ----

/// 报销单列表行。
#[derive(Debug, Serialize, FromRow)]
pub struct ReimbursementListRow {
    pub id: String,
    pub employee_id: String,
    pub employee_name: String,
    pub department_id: String,
    pub department_name: String,
    pub title: String,
    pub category: String,
    pub amount: f64,
    pub currency: String,
    pub status: String,
    pub approver_id: Option<String>,
    pub approver_name: Option<String>,
    pub approve_comment: Option<String>,
    pub finance_reviewer_id: Option<String>,
    pub finance_reviewer_name: Option<String>,
    pub finance_comment: Option<String>,
    pub paid_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

/// 报销单详情内部行（FromRow，供详情查询使用）。
#[derive(Debug, FromRow)]
pub struct ReimbursementDetailRow {
    pub employee_id: String,
    pub employee_name: String,
    pub department_id: String,
    pub department_name: String,
    pub title: String,
    pub category: String,
    pub amount: f64,
    pub currency: String,
    pub reason: Option<String>,
    pub status: String,
    pub approver_id: Option<String>,
    pub approver_name: Option<String>,
    pub approve_comment: Option<String>,
    pub approved_at: Option<NaiveDateTime>,
    pub finance_reviewer_id: Option<String>,
    pub finance_reviewer_name: Option<String>,
    pub finance_comment: Option<String>,
    pub finance_reviewed_at: Option<NaiveDateTime>,
    pub paid_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

/// 报销单详情（含发票与审批流水）。
#[derive(Debug, Serialize)]
pub struct ReimbursementDetail {
    pub id: String,
    pub employee_id: String,
    pub employee_name: String,
    pub department_id: String,
    pub department_name: String,
    pub title: String,
    pub category: String,
    pub amount: f64,
    pub currency: String,
    pub reason: Option<String>,
    pub status: String,
    pub approver_id: Option<String>,
    pub approver_name: Option<String>,
    pub approve_comment: Option<String>,
    pub approved_at: Option<NaiveDateTime>,
    pub finance_reviewer_id: Option<String>,
    pub finance_reviewer_name: Option<String>,
    pub finance_comment: Option<String>,
    pub finance_reviewed_at: Option<NaiveDateTime>,
    pub paid_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub invoices: Vec<InvoiceRow>,
    pub logs: Vec<ReimbursementLogRow>,
}

/// 审批/状态流水行。
#[derive(Debug, Serialize, FromRow)]
pub struct ReimbursementLogRow {
    pub id: String,
    pub action: String,
    pub actor_id: String,
    pub actor_name: String,
    pub comment: Option<String>,
    pub created_at: NaiveDateTime,
}

/// 新建报销单请求。
#[derive(Debug, Deserialize)]
pub struct NewReimbursement {
    pub title: String,
    pub category: String,
    pub amount: f64,
    pub reason: Option<String>,
    /// 关联发票 id 列表（可空；invoice 须存在且未被其他报销单占用）。
    #[serde(default)]
    pub invoice_ids: Vec<String>,
}

/// 更新报销单请求（仅 pending_leader / rejected / withdrawn 状态可编辑并重新提交）。
#[derive(Debug, Deserialize)]
pub struct UpdateReimbursement {
    pub title: Option<String>,
    pub category: Option<String>,
    pub amount: Option<f64>,
    pub reason: Option<Option<String>>,
    #[serde(default)]
    pub invoice_ids: Option<Vec<String>>,
}

/// 审批/复核动作请求（approve 通过 / reject 驳回）。
#[derive(Debug, Deserialize)]
pub struct ReviewAction {
    pub action: String,
    pub comment: Option<String>,
}

/// 报销单查询参数。
#[derive(Debug, Deserialize)]
pub struct ReimbursementQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub department_id: Option<String>,
}

// ---- 发票 ----

/// 发票行。
#[derive(Debug, Serialize, FromRow, Clone)]
pub struct InvoiceRow {
    pub id: String,
    pub invoice_code: String,
    pub invoice_type: String,
    pub amount: f64,
    pub tax_amount: Option<f64>,
    pub issued_at: Option<NaiveDate>,
    pub issuer_name: String,
    pub buyer_name: Option<String>,
    pub image_url: Option<String>,
    pub employee_id: String,
    pub employee_name: String,
    /// unused 未关联 | claimed 已关联报销单。
    pub status: String,
    pub created_at: NaiveDateTime,
}

/// 新建发票请求。
#[derive(Debug, Deserialize)]
pub struct NewInvoice {
    pub invoice_code: String,
    pub invoice_type: Option<String>,
    pub amount: f64,
    pub tax_amount: Option<f64>,
    pub issued_at: Option<NaiveDate>,
    pub issuer_name: String,
    pub buyer_name: Option<String>,
    pub image_url: Option<String>,
}

/// 更新发票请求。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateInvoice {
    pub invoice_code: Option<String>,
    pub invoice_type: Option<String>,
    pub amount: Option<f64>,
    pub tax_amount: Option<Option<f64>>,
    pub issued_at: Option<Option<NaiveDate>>,
    pub issuer_name: Option<String>,
    pub buyer_name: Option<Option<String>>,
    pub image_url: Option<Option<String>>,
}

/// 发票查询参数。
#[derive(Debug, Deserialize)]
pub struct InvoiceQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub keyword: Option<String>,
    pub status: Option<String>,
}

// ---- 收付款记录 ----

/// 收付款记录行。
#[derive(Debug, Serialize, FromRow)]
pub struct PaymentRow {
    pub id: String,
    /// income 收款 | expense 付款。
    pub direction: String,
    pub category: String,
    pub amount: f64,
    pub counterparty: Option<String>,
    pub occurred_at: NaiveDate,
    pub department_id: Option<String>,
    pub department_name: Option<String>,
    pub remark: Option<String>,
    /// 关联报销单 id（报销付款自动生成时非空）。
    pub reimbursement_id: Option<String>,
    pub created_by: String,
    pub creator_name: String,
    pub created_at: NaiveDateTime,
}

/// 新建收付款记录请求。
#[derive(Debug, Deserialize)]
pub struct NewPayment {
    pub direction: String,
    pub category: String,
    pub amount: f64,
    pub counterparty: Option<String>,
    pub occurred_at: NaiveDate,
    pub department_id: Option<String>,
    pub remark: Option<String>,
}

/// 更新收付款记录请求。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdatePayment {
    pub direction: Option<String>,
    pub category: Option<String>,
    pub amount: Option<f64>,
    pub counterparty: Option<Option<String>>,
    pub occurred_at: Option<NaiveDate>,
    pub department_id: Option<Option<String>>,
    pub remark: Option<Option<String>>,
}

/// 收付款查询参数。
#[derive(Debug, Deserialize)]
pub struct PaymentQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub direction: Option<String>,
    pub keyword: Option<String>,
    pub department_id: Option<String>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

// ---- 预算 ----

/// 预算原始行（不含聚合列，spent/remaining 由 handler 计算）。
#[derive(Debug, FromRow)]
pub struct BudgetRawRow {
    pub id: String,
    pub department_id: String,
    pub department_name: String,
    pub period_type: String,
    pub period_value: String,
    pub amount: f64,
    pub created_at: NaiveDateTime,
}

/// 新建预算请求。
#[derive(Debug, Deserialize)]
pub struct NewBudget {
    pub department_id: String,
    pub period_type: String,
    pub period_value: String,
    pub amount: f64,
}

/// 更新预算请求。
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct UpdateBudget {
    pub department_id: Option<String>,
    pub period_type: Option<String>,
    pub period_value: Option<String>,
    pub amount: Option<f64>,
}

/// 预算查询参数。
#[derive(Debug, Deserialize)]
pub struct BudgetQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub period_type: Option<String>,
    pub period_value: Option<String>,
    pub department_id: Option<String>,
}

// ---- 财务报表 ----

/// 报表通用日期范围参数。
#[derive(Debug, Deserialize)]
pub struct ReportQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// trend 粒度：month | year（默认 month）。
    pub granularity: Option<String>,
}

/// 汇总报表：期间内收/支/结余与报销统计。
#[derive(Debug, Serialize)]
pub struct ReportSummary {
    pub income: f64,
    pub expense: f64,
    pub net: f64,
    pub income_count: i64,
    pub expense_count: i64,
    pub reimbursement_pending: f64,
    pub reimbursement_pending_count: i64,
}

/// 部门费用排行行。
#[derive(Debug, Serialize)]
pub struct DepartmentReportRow {
    pub department_id: String,
    pub department_name: String,
    pub expense: f64,
}

/// 趋势行（按月份或年份聚合收/支）。
#[derive(Debug, Serialize)]
pub struct TrendRow {
    pub period: String,
    pub income: f64,
    pub expense: f64,
}
