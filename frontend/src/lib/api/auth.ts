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
