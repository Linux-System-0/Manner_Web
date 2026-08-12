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
import { t } from '@/i18n'
import type { ApiResponse, LoginPageInfo, LogsData, PermissionModule, SystemSettings } from '@/types'

export async function getLoginPage(): Promise<ApiResponse<LoginPageInfo>> {
  const res = await client.get<LoginPageInfo>('/system/login-page')
  return res
}

export async function getSystemSettings(): Promise<ApiResponse<SystemSettings>> {
  const res = await client.get<SystemSettings>('/system/settings')
  return res
}

export async function updateSystemSettings(
  data: Partial<SystemSettings>,
): Promise<ApiResponse<null>> {
  const res = await client.put<null>('/system/settings', data)
  return res
}

export async function getSystemLogs(lines?: number): Promise<ApiResponse<LogsData>> {
  const res = await client.get<LogsData>('/system/logs', { lines })
  return res
}

/** 上传头像/图片（任意登录用户，图片白名单） */
export async function uploadImage(file: File): Promise<string> {
  const formData = new FormData()
  formData.append('file', file)
  const res = await client.upload<unknown>('/upload', formData)
  const uploadJson = res as { data?: unknown; message?: string }
  if (!uploadJson.data) throw new Error(uploadJson.message || t('settings.uploadFailed'))
  return uploadJson.data as string
}

/** 权限字典（按模块分组），供员工直接授权使用 */
export async function getPermissions(): Promise<ApiResponse<{ modules: PermissionModule[] }>> {
  const res = await client.get<{ modules: PermissionModule[] }>('/permissions')
  return res
}
