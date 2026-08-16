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

import { client } from './client'
import type {
  ApiResponse,
  Budget,
  DepartmentReportRow,
  Invoice,
  PaginatedData,
  Payment,
  Reimbursement,
  ReimbursementDetail,
  ReimbursementStatus,
  ReportSummary,
  TrendRow,
} from '@/types'

// ---- 报销单 ----

export interface ReimbursementQuery {
  page?: number
  page_size?: number
  status?: ReimbursementStatus | ''
  keyword?: string
  department_id?: string
}

export async function getReimbursements(
  params: ReimbursementQuery = {},
): Promise<ApiResponse<PaginatedData<Reimbursement>>> {
  const res = await client.get<PaginatedData<Reimbursement>>(
    '/finance/reimbursements',
    params as Record<string, never>,
  )
  return res
}

export async function getReimbursement(
  id: string,
): Promise<ApiResponse<ReimbursementDetail>> {
  const res = await client.get<ReimbursementDetail>(`/finance/reimbursements/${id}`)
  return res
}

export async function createReimbursement(data: {
  title: string
  category: string
  amount: number
  reason?: string
  invoice_ids?: string[]
}): Promise<ApiResponse<{ id: string }>> {
  const res = await client.post<{ id: string }>('/finance/reimbursements', data)
  return res
}

export async function updateReimbursement(
  id: string,
  data: {
    title?: string
    category?: string
    amount?: number
    reason?: string | null
    invoice_ids?: string[]
  },
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(`/finance/reimbursements/${id}`, data)
  return res
}

export async function deleteReimbursement(id: string): Promise<ApiResponse<null>> {
  const res = await client.delete<null>(`/finance/reimbursements/${id}`)
  return res
}

export async function approveReimbursement(
  id: string,
  action: 'approve' | 'reject',
  comment?: string,
): Promise<ApiResponse<null>> {
  const res = await client.post<null>(`/finance/reimbursements/${id}/approve`, {
    action,
    comment,
  })
  return res
}

export async function reviewReimbursement(
  id: string,
  action: 'approve' | 'reject',
  comment?: string,
): Promise<ApiResponse<null>> {
  const res = await client.post<null>(`/finance/reimbursements/${id}/review`, {
    action,
    comment,
  })
  return res
}

export async function payReimbursement(id: string): Promise<ApiResponse<null>> {
  const res = await client.post<null>(`/finance/reimbursements/${id}/pay`)
  return res
}

export async function withdrawReimbursement(id: string): Promise<ApiResponse<null>> {
  const res = await client.post<null>(`/finance/reimbursements/${id}/withdraw`)
  return res
}

// ---- 发票 ----

export interface InvoiceQuery {
  page?: number
  page_size?: number
  keyword?: string
  status?: string
}

export async function getInvoices(
  params: InvoiceQuery = {},
): Promise<ApiResponse<PaginatedData<Invoice>>> {
  const res = await client.get<PaginatedData<Invoice>>(
    '/finance/invoices',
    params as Record<string, never>,
  )
  return res
}

export async function createInvoice(data: {
  invoice_code: string
  invoice_type?: string
  amount: number
  tax_amount?: number
  issued_at?: string
  issuer_name: string
  buyer_name?: string
  image_url?: string
}): Promise<ApiResponse<{ id: string }>> {
  const res = await client.post<{ id: string }>('/finance/invoices', data)
  return res
}

export async function updateInvoice(
  id: string,
  data: Partial<{
    invoice_code: string
    invoice_type: string
    amount: number
    tax_amount: number | null
    issued_at: string | null
    issuer_name: string
    buyer_name: string | null
    image_url: string | null
  }>,
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(`/finance/invoices/${id}`, data)
  return res
}

export async function deleteInvoice(id: string): Promise<ApiResponse<null>> {
  const res = await client.delete<null>(`/finance/invoices/${id}`)
  return res
}

// ---- 收付款记录 ----

export interface PaymentQuery {
  page?: number
  page_size?: number
  direction?: string
  keyword?: string
  department_id?: string
  from?: string
  to?: string
}

export async function getPayments(
  params: PaymentQuery = {},
): Promise<ApiResponse<PaginatedData<Payment>>> {
  const res = await client.get<PaginatedData<Payment>>(
    '/finance/payments',
    params as Record<string, never>,
  )
  return res
}

export async function createPayment(data: {
  direction: 'income' | 'expense'
  category: string
  amount: number
  counterparty?: string
  occurred_at: string
  department_id?: string
  remark?: string
}): Promise<ApiResponse<{ id: string }>> {
  const res = await client.post<{ id: string }>('/finance/payments', data)
  return res
}

export async function updatePayment(
  id: string,
  data: Partial<{
    direction: 'income' | 'expense'
    category: string
    amount: number
    counterparty: string | null
    occurred_at: string
    department_id: string | null
    remark: string | null
  }>,
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(`/finance/payments/${id}`, data)
  return res
}

export async function deletePayment(id: string): Promise<ApiResponse<null>> {
  const res = await client.delete<null>(`/finance/payments/${id}`)
  return res
}

// ---- 预算 ----

export interface BudgetQuery {
  page?: number
  page_size?: number
  period_type?: string
  period_value?: string
  department_id?: string
}

export async function getBudgets(
  params: BudgetQuery = {},
): Promise<ApiResponse<PaginatedData<Budget>>> {
  const res = await client.get<PaginatedData<Budget>>(
    '/finance/budgets',
    params as Record<string, never>,
  )
  return res
}

export async function createBudget(data: {
  department_id: string
  period_type: 'year' | 'month'
  period_value: string
  amount: number
}): Promise<ApiResponse<{ id: string }>> {
  const res = await client.post<{ id: string }>('/finance/budgets', data)
  return res
}

export async function updateBudget(
  id: string,
  data: Partial<{
    department_id: string
    period_type: string
    period_value: string
    amount: number
  }>,
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(`/finance/budgets/${id}`, data)
  return res
}

export async function deleteBudget(id: string): Promise<ApiResponse<null>> {
  const res = await client.delete<null>(`/finance/budgets/${id}`)
  return res
}

// ---- 财务报表 ----

export interface ReportQuery {
  from?: string
  to?: string
  granularity?: string
}

export async function getReportSummary(
  params: ReportQuery = {},
): Promise<ApiResponse<ReportSummary>> {
  const res = await client.get<ReportSummary>(
    '/finance/reports/summary',
    params as Record<string, never>,
  )
  return res
}

export async function getReportDepartments(
  params: ReportQuery = {},
): Promise<ApiResponse<{ items: DepartmentReportRow[]; total_expense: number }>> {
  const res = await client.get<{ items: DepartmentReportRow[]; total_expense: number }>(
    '/finance/reports/departments',
    params as Record<string, never>,
  )
  return res
}

export async function getReportTrend(
  params: ReportQuery = {},
): Promise<ApiResponse<{ items: TrendRow[] }>> {
  const res = await client.get<{ items: TrendRow[] }>(
    '/finance/reports/trend',
    params as Record<string, never>,
  )
  return res
}

/** 打开 CSV 导出（浏览器直链下载，携带 Cookie 认证）。 */
export function exportReportUrl(
  type: 'reimbursements' | 'payments',
  params: ReportQuery = {},
): string {
  const search = new URLSearchParams()
  if (params.from) search.set('from', params.from)
  if (params.to) search.set('to', params.to)
  const qs = search.toString()
  return `/api/finance/reports/export/${type}${qs ? `?${qs}` : ''}`
}
