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
  LoginResponseData,
  RegisterRequest,
  ChangePasswordRequest,
  FirstLoginRequest,
  PrecheckResponse,
  User,
} from '@/types'

export async function login(
  username: string,
  password: string,
): Promise<ApiResponse<LoginResponseData>> {
  const res = await client.post<LoginResponseData>('/auth/login', { username, password })
  return res
}

export async function logout(): Promise<ApiResponse<null>> {
  const res = await client.post<null>('/auth/logout')
  return res
}

export async function register(
  data: RegisterRequest,
): Promise<ApiResponse<{ id: string; username: string; name: string }>> {
  const res = await client.post<{ id: string; username: string; name: string }>('/auth/register', data)
  return res
}

export async function changePassword(
  old_password: string,
  new_password: string,
): Promise<ApiResponse<null>> {
  const res = await client.put<null>('/auth/password', {
    old_password,
    new_password,
  } as ChangePasswordRequest)
  return res
}

export async function getMe(): Promise<ApiResponse<User>> {
  const res = await client.get<User>('/auth/me')
  return res
}

export async function precheck(
  username: string,
): Promise<ApiResponse<PrecheckResponse>> {
  const res = await client.post<PrecheckResponse>('/auth/precheck', { username })
  return res
}

export async function firstLogin(
  data: FirstLoginRequest,
): Promise<ApiResponse<LoginResponseData>> {
  const res = await client.post<LoginResponseData>('/auth/first-login', data)
  return res
}
