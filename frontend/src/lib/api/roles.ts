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

// 角色管理 API（RBAC + 数据范围 + 部门角色继承）
import { client } from './client'
import type { ApiResponse, Role } from '@/types'

export interface RoleListData {
  items: Role[]
  total: number
}

export async function getRoles(): Promise<ApiResponse<RoleListData>> {
  const res = await client.get<RoleListData>('/roles')
  return res
}

export interface CreateRoleRequest {
  name: string
  parent_id?: string
  scope_type: string
  description?: string
  permission_codes: string[]
  scope_department_ids: string[]
}

export async function createRole(data: CreateRoleRequest): Promise<ApiResponse<{ id: string }>> {
  const res = await client.post<{ id: string }>('/roles', data)
  return res
}

export interface UpdateRoleRequest {
  name?: string
  parent_id?: string | null
  scope_type?: string
  description?: string | null
  permission_codes?: string[]
  scope_department_ids?: string[]
}

export async function updateRole(
  id: string,
  data: UpdateRoleRequest,
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(`/roles/${id}`, data)
  return res
}

export async function deleteRole(id: string): Promise<ApiResponse<null>> {
  const res = await client.delete<null>(`/roles/${id}`)
  return res
}

/** 整体替换员工分配的角色 */
export async function updateEmployeeRoles(
  employeeId: string,
  role_ids: string[],
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(`/employees/${employeeId}/roles`, { role_ids })
  return res
}

export interface DepartmentRoleData {
  items: { id: string; name: string }[]
  total: number
}

/** 部门绑定的角色列表 */
export async function getDepartmentRoles(
  departmentId: string,
): Promise<ApiResponse<DepartmentRoleData>> {
  const res = await client.get<DepartmentRoleData>(`/departments/${departmentId}/roles`)
  return res
}

/** 整体替换部门绑定的角色 */
export async function updateDepartmentRoles(
  departmentId: string,
  role_ids: string[],
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(`/departments/${departmentId}/roles`, { role_ids })
  return res
}
