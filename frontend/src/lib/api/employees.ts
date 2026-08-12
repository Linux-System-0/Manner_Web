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
  PaginatedData,
  PaginatedResponse,
  Employee,
  EmployeeQueryParams,
  CreateEmployeeRequest,
  UpdateEmployeeRequest,
  SensitiveEmployeeInfo,
} from '@/types'

export async function getEmployees(
  params: EmployeeQueryParams = {},
): Promise<PaginatedResponse<Employee>> {
  const res = await client.get<PaginatedData<Employee>>('/employees', params as Record<string, never>)
  return res
}

export async function getEmployee(
  id: string,
): Promise<ApiResponse<Employee>> {
  const res = await client.get<Employee>(`/employees/${id}`)
  return res
}

/**
 * 查看员工敏感信息（解密明文）。
 * 需要 employee:view_sensitive 权限；每次调用后端强制记录审计日志。
 */
export async function viewSensitiveEmployee(
  id: string,
): Promise<ApiResponse<SensitiveEmployeeInfo>> {
  const res = await client.post<SensitiveEmployeeInfo>(`/employees/${id}/sensitive`)
  return res
}

/**
 * 查看员工敏感信息中的单个字段（解密明文）。
 * 后端按字段细粒度记录审计日志（如「查看了邮箱」），并追加访问 IP。
 * field 取值：email | phone | id_number | address
 */
export async function viewSensitiveField(
  id: string,
  field: string,
): Promise<ApiResponse<{ field: string; value: string | null }>> {
  const res = await client.post<{ field: string; value: string | null }>(
    `/employees/${id}/sensitive/${field}`,
  )
  return res
}

export async function createEmployee(
  data: CreateEmployeeRequest,
): Promise<ApiResponse<Employee>> {
  const res = await client.post<Employee>('/employees', data)
  return res
}

export async function updateEmployee(
  id: string,
  data: UpdateEmployeeRequest,
): Promise<ApiResponse<Employee>> {
  const res = await client.put<Employee>(`/employees/${id}`, data)
  return res
}

export async function deleteEmployee(
  id: string,
): Promise<ApiResponse<null>> {
  const res = await client.delete<null>(`/employees/${id}`)
  return res
}

export async function resetPassword(
  id: string,
  new_password: string,
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(`/employees/${id}/password`, {
    new_password,
  })
  return res
}
