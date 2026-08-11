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
  code: string
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
