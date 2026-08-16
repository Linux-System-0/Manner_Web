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
import type { ApiResponse, PaginatedData } from '@/types'

export interface Task {
  id: string
  title: string
  description: string | null
  assignee_id: string
  assignee_name: string
  creator_id: string
  creator_name: string
  status: 'todo' | 'done'
  due_date: string | null
  completed_at: string | null
  created_at: string
}

export interface TaskListData extends PaginatedData<Task> {
  can_view_all: boolean
}

export interface TaskStats {
  total: number
  todo: number
  done: number
  overdue: number
  can_view_all: boolean
}

export interface TaskQuery {
  page?: number
  page_size?: number
  status?: 'todo' | 'done' | ''
  assignee_id?: string
  scope?: 'all' | 'mine'
}

export async function getTasks(
  params: TaskQuery = {},
): Promise<ApiResponse<TaskListData>> {
  const res = await client.get<TaskListData>('/tasks', params as Record<string, never>)
  return res
}

export async function getTaskStats(): Promise<ApiResponse<TaskStats>> {
  const res = await client.get<TaskStats>('/tasks/stats')
  return res
}

export async function createTask(data: {
  title: string
  description?: string
  assignee_id?: string
  due_date?: string
}): Promise<ApiResponse<{ id: string }>> {
  const res = await client.post<{ id: string }>('/tasks', data)
  return res
}

export async function updateTask(
  id: string,
  data: {
    title?: string
    description?: string | null
    assignee_id?: string
    status?: 'todo' | 'done'
    due_date?: string | null
  },
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(`/tasks/${id}`, data)
  return res
}

export async function deleteTask(id: string): Promise<ApiResponse<null>> {
  const res = await client.delete<null>(`/tasks/${id}`)
  return res
}
