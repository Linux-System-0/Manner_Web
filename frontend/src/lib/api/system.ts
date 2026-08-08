import { client } from './client'
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
  if (!uploadJson.data) throw new Error(uploadJson.message || '上传失败')
  return uploadJson.data as string
}

/** 权限字典（按模块分组），供员工直接授权使用 */
export async function getPermissions(): Promise<ApiResponse<{ modules: PermissionModule[] }>> {
  const res = await client.get<{ modules: PermissionModule[] }>('/permissions')
  return res
}
