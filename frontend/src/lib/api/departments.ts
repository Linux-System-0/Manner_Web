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
