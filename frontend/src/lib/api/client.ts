// 与 axios 版（原 src/api/client.ts）语义等价的自研 fetch 封装：
// - credentials: 'include' —— 浏览器自动携带 httpOnly Cookie（manner_token / manner_refresh），
//   前端不持有 JWT，也无需手工附加 Authorization。
// - 令牌失效自动续期：
//   * 任意业务接口 401 时，静默调用 /auth/refresh 用 refresh Cookie 换取全新会话；
//   * 并发多个 401 只触发一次刷新，其余请求等待同一 Promise；
//   * 刷新成功后重放原请求（新 Cookie 自动携带）；
//   * 刷新失败（refresh 令牌失效/被踢/改密后版本不符）才清空登录态并强制回到登录页。
// - 30s 超时（AbortController）。
import { get } from 'svelte/store'
import { authStore } from '@/stores/auth'
import type { ApiResponse } from '@/types'

export interface RequestOptions {
  /** 查询参数（追加到 URL） */
  params?: Record<string, string | number | boolean | null | undefined>
  /** JSON 请求体 */
  body?: unknown
  /** multipart/form-data 请求体（上传） */
  formData?: FormData
  headers?: Record<string, string>
  /**
   * 401 时跳过自动续期与强制跳转登录页（保持原 401 响应返回给调用方）。
   * 用于未登录场景也应容忍失败的请求（如登录页拉取个人偏好），
   * 否则 refresh 失败后 location.href 整页刷新会与布局 onMount 形成无限循环。
   */
  skipAuthRedirect?: boolean
}

const TIMEOUT_MS = 30000
const AUTH_ENDPOINTS = ['/auth/login', '/auth/first-login', '/auth/refresh']

function isAuthEndpoint(url: string): boolean {
  return AUTH_ENDPOINTS.some((s) => url.includes(s))
}

let refreshing: Promise<boolean> | null = null

async function tryRefresh(): Promise<boolean> {
  if (!refreshing) {
    refreshing = fetch('/api/auth/refresh', {
      method: 'POST',
      credentials: 'include',
    })
      .then(async (res) => {
        if (!res.ok) return false
        const body = (await res.json().catch(() => null)) as ApiResponse<{ user?: unknown }> | null
        // 会话已恢复，同步最新用户信息（权限、头像等）到 store
        const user = body?.data?.user
        if (user) {
          authStore.setUser(user as never)
        }
        return true
      })
      .catch(() => false)
      .finally(() => {
        refreshing = null
      })
  }
  return refreshing
}

function buildUrl(url: string, params?: RequestOptions['params']): string {
  // 统一补全 /api 前缀：后端路由均为 /api/*（见 backend/src/handlers/mod.rs），
  // dev 下由 vite 代理 /api 转发到 8080；已带 /api 或完整 http(s) URL 则原样保留。
  const apiUrl = /^https?:\/\//.test(url) || url.startsWith('/api') ? url : `/api${url}`
  if (!params) return apiUrl
  const search = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '') {
      search.append(key, String(value))
    }
  }
  const qs = search.toString()
  return qs ? `${apiUrl}${apiUrl.includes('?') ? '&' : '?'}${qs}` : apiUrl
}

async function doFetch(
  method: string,
  url: string,
  options: RequestOptions,
): Promise<Response> {
  const headers: Record<string, string> = { ...(options.headers || {}) }
  let body: BodyInit | undefined
  if (options.formData) {
    body = options.formData
  } else if (options.body !== undefined) {
    headers['Content-Type'] = 'application/json'
    body = JSON.stringify(options.body)
  }

  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), TIMEOUT_MS)
  try {
    return await fetch(buildUrl(url, options.params), {
      method,
      credentials: 'include',
      headers,
      body,
      signal: controller.signal,
    })
  } finally {
    clearTimeout(timer)
  }
}

async function request<T>(
  method: string,
  url: string,
  options: RequestOptions = {},
  retried = false,
): Promise<ApiResponse<T>> {
  const res = await doFetch(method, url, options)

  // 401 且非认证端点自身、且未重放过：尝试静默续期后重放
  if (res.status === 401 && !retried && !isAuthEndpoint(url) && !options.skipAuthRedirect) {
    const refreshed = await tryRefresh()
    if (refreshed) {
      return request<T>(method, url, options, true)
    }
    // 续期失败：前端已无有效会话，强制回到登录页
    authStore.logout()
    if (typeof window !== 'undefined') {
      window.location.href = '/login'
    }
  }

  let data: ApiResponse<T>
  try {
    data = (await res.json()) as ApiResponse<T>
  } catch {
    data = { code: res.status, message: res.statusText || '请求失败', data: null as T }
  }
  return data
}

export const client = {
  get: <T>(url: string, params?: RequestOptions['params'], options?: Omit<RequestOptions, 'params'>) =>
    request<T>('GET', url, { ...options, params }),
  post: <T>(url: string, body?: unknown, options?: Omit<RequestOptions, 'body'>) =>
    request<T>('POST', url, { ...options, body }),
  put: <T>(url: string, body?: unknown) =>
    request<T>('PUT', url, { body }),
  delete: <T>(url: string) =>
    request<T>('DELETE', url),
  /** 上传：multipart/form-data */
  upload: <T>(url: string, formData: FormData) =>
    request<T>('POST', url, { formData }),
}

/** 从错误响应中提取后端 message，无则用 fallback */
export function getApiError(err: unknown, fallback: string): string {
  const candidate = (err as { response?: { data?: { message?: string } } })?.response?.data
    ?.message
  if (candidate) return candidate
  const e = err as { data?: { message?: string } } | undefined
  if (e?.data?.message) return e.data.message
  return fallback
}

/** 供页面直接使用：getApiError 兼容 Promise 拒绝对象 */
export function extractApiError(err: unknown, fallback: string): string {
  return getApiError(err, fallback)
}

// 保持与既有调用习惯一致的默认导出（部分页面直接 import client）
export default client

// 类型辅助：供依赖 get(store) 的场景使用（避免未使用告警）
export { get }
