// 认证状态（替代原 zustand authStore，语义等价）：
// - localStorage 键名沿用 manner-auth-storage，格式兼容 zustand persist 产物
//   （{"state":{"user":{...}},"version":0}），已登录浏览器可直接无缝升级。
// - F1/F4：仅持久化 user。permissions 由 user.permissions 派生，不单独落盘——
//   刷新后由根布局挂载时通过 /auth/me 从服务端拉取并覆盖，
//   杜绝攻击者直接篡改 localStorage 中的权限列表/登录态。
import { writable, get } from 'svelte/store'
import type { User } from '@/types'

const STORAGE_KEY = 'manner-auth-storage'

interface PersistedShape {
  state?: { user?: User | null }
}

function loadUser(): User | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw) as PersistedShape
    return parsed?.state?.user ?? null
  } catch {
    return null
  }
}

function saveUser(user: User | null) {
  try {
    if (user) {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ state: { user }, version: 0 }))
    } else {
      localStorage.removeItem(STORAGE_KEY)
    }
  } catch {
    /* storage 不可用（隐私模式等）时静默降级为纯内存态 */
  }
}

function createAuthStore() {
  const { subscribe, set, update } = writable<{
    user: User | null
    permissions: string[]
    isAuthenticated: boolean
  }>({
    user: null,
    permissions: [],
    isAuthenticated: false,
  })

  return {
    subscribe,
    /** 登录/首登成功：写入完整用户与会话态 */
    setAuth(user: User) {
      set({ user, permissions: user.permissions, isAuthenticated: true })
      saveUser(user)
    },
    /** refresh 续期 / getMe 成功：同步最新用户信息并恢复登录态 */
    setUser(user: User) {
      set({ user, permissions: user.permissions, isAuthenticated: true })
      saveUser(user)
    },
    logout() {
      set({ user: null, permissions: [], isAuthenticated: false })
      saveUser(null)
    },
    hasPermission(code: string): boolean {
      return get({ subscribe }).permissions.includes(code)
    },
    /** 应用启动时从本地恢复（仅 user），登录态最终以 /auth/me 为准 */
    restoreLocal() {
      const user = loadUser()
      if (user) {
        set({ user, permissions: user.permissions, isAuthenticated: false })
      }
    },
  }
}

export const authStore = createAuthStore()

export type AuthState = ReturnType<typeof createAuthStore>
