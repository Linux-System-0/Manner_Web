import { client } from './client'
import type { ApiResponse, Department, DepartmentMember } from '@/types'

export interface DepartmentListData {
  items: Department[]
  total: number
}

export async function getDepartments(): Promise<ApiResponse<DepartmentListData>> {
  const res = await client.get<DepartmentListData>('/departments')
  return res
}

export async function createDepartment(data: {
  name: string
  parent_id?: string
  leader_ids?: string[]
  sort_order?: number
}): Promise<ApiResponse<{ id: string }>> {
  const res = await client.post<{ id: string }>('/departments', data)
  return res
}

export async function updateDepartment(
  id: string,
  data: {
    name?: string
    parent_id?: string | null
    leader_ids?: string[]
    sort_order?: number
  },
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(`/departments/${id}`, data)
  return res
}

export async function deleteDepartment(id: string): Promise<ApiResponse<null>> {
  const res = await client.delete<null>(`/departments/${id}`)
  return res
}

export async function getDepartmentMembers(
  id: string,
): Promise<ApiResponse<{ items: DepartmentMember[]; total: number }>> {
  const res = await client.get<{ items: DepartmentMember[]; total: number }>(
    `/departments/${id}/members`,
  )
  return res
}

export async function updateEmployeeDepartments(
  employeeId: string,
  department_ids: string[],
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(`/employees/${employeeId}/departments`, { department_ids })
  return res
}
