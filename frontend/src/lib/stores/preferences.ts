// 个人偏好（替代原 src/stores/preferences.ts，语义等价）：
// - localStorage 键名沿用 manner-preferences，服务端同步到 /auth/preferences
// - 主题：light/dark/system，通过 html[data-theme] 驱动全局样式
// - 时间格式化：system 时区用 toLocaleString；manual 时区按 timezoneOffset 小时偏移
// - 保留全局 pub/sub（getGlobalPrefs/subscribe），供 Chat 等非组件模块使用
import { writable, get } from 'svelte/store'
import { client } from '@/api/client'

export type ThemeMode = 'light' | 'dark' | 'system'
export type TimezoneMode = 'system' | 'manual'
export type NewConvPosition = 'first' | 'last'

export interface Preferences {
  theme: ThemeMode
  timezoneMode: TimezoneMode
  timezoneOffset: number
  newConvPosition: NewConvPosition
}

const STORAGE_KEY = 'manner-preferences'

export const defaultPrefs: Preferences = {
  theme: 'system',
  timezoneMode: 'system',
  timezoneOffset: 0,
  newConvPosition: 'last',
}

function loadLocal(): Preferences {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) return { ...defaultPrefs, ...JSON.parse(raw) }
  } catch {
    /* ignore */
  }
  return { ...defaultPrefs }
}

function saveLocal(prefs: Preferences) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs))
  } catch {
    /* ignore */
  }
}

export function getEffectiveTheme(theme: ThemeMode): 'light' | 'dark' {
  if (theme === 'system') {
    return typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light'
  }
  return theme
}

/** 将偏好写入 html[data-theme]，同时同步 color-scheme */
export function applyTheme(theme: ThemeMode) {
  if (typeof document === 'undefined') return
  const effective = getEffectiveTheme(theme)
  document.documentElement.setAttribute('data-theme', effective)
}

function parseUtc(iso: string): Date {
  if (/Z$|([+-]\d{2}:\d{2})$/.test(iso)) return new Date(iso)
  return new Date(iso.replace(' ', 'T') + 'Z')
}

function pad(n: number): string {
  return String(n).padStart(2, '0')
}

function formatUtcWall(ts: number): string {
  const d = new Date(ts)
  return `${d.getUTCFullYear()}-${pad(d.getUTCMonth() + 1)}-${pad(d.getUTCDate())} ${pad(d.getUTCHours())}:${pad(d.getUTCMinutes())}:${pad(d.getUTCSeconds())}`
}

function formatLocalDateTime(d: Date): string {
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

export function formatTimestamp(iso: string, prefs: Preferences): string {
  const d = parseUtc(iso)
  if (isNaN(d.getTime())) return iso
  if (prefs.timezoneMode === 'manual') {
    return formatUtcWall(d.getTime() + prefs.timezoneOffset * 3600000)
  }
  return d.toLocaleString('zh-CN', { hour12: false })
}

// 聊天消息/会话时间：统一 年-月-日 时:分:秒（manual 时区用 UTC 墙钟字段，其余用本地时区）
export function formatTime(iso: string, prefs: Preferences): string {
  const d = parseUtc(iso)
  if (isNaN(d.getTime())) return iso
  if (prefs.timezoneMode === 'manual') {
    return formatUtcWall(d.getTime() + prefs.timezoneOffset * 3600000)
  }
  return formatLocalDateTime(d)
}

// ---- 全局 pub/sub（供非组件模块读取最新偏好）----

let globalPrefs: Preferences = { ...defaultPrefs }
const listeners = new Set<() => void>()

export function getGlobalPrefs(): Preferences {
  return globalPrefs
}

export function subscribe(fn: () => void): () => void {
  listeners.add(fn)
  return () => listeners.delete(fn)
}

function notify() {
  listeners.forEach((fn) => fn())
}

// ---- Svelte store ----

function normalizePrefs(data: Record<string, unknown> | undefined | null): Preferences {
  if (!data) return { ...defaultPrefs }
  return {
    theme: (data.theme as ThemeMode) || 'system',
    timezoneMode: (data.timezoneMode as TimezoneMode) || 'system',
    timezoneOffset: typeof data.timezoneOffset === 'number' ? data.timezoneOffset : 0,
    newConvPosition: data.newConvPosition === 'first' ? 'first' : 'last',
  }
}

function createPreferencesStore() {
  const initial = loadLocal()
  const { subscribe, set } = writable<Preferences>(initial)

  function syncBackend(p: Preferences) {
    client.put('/auth/preferences', {
      preferences: {
        theme: p.theme,
        timezoneMode: p.timezoneMode,
        timezoneOffset: p.timezoneOffset,
        newConvPosition: p.newConvPosition,
      },
    }).catch(() => {})
  }

  /**
   * 从服务端拉取当前登录用户的偏好并覆盖本地（偏好按「用户」存储于服务端，
   * 而非按「设备」保存在 localStorage；登录成功后必须重新调用以拿到该用户的服务端偏好）。
   */
  async function refresh() {
    try {
      const res = await client.get<Record<string, unknown>>('/auth/preferences', undefined, { skipAuthRedirect: true })
      // 401（未登录）时 client 返回 { code: 40002, data: null } 而非抛错，
      // 此处必须跳过，否则会拿默认值覆盖本地已保存的偏好。
      if (res.data == null) return
      const next = normalizePrefs(res.data)
      set(next)
      globalPrefs = next
      saveLocal(next)
      notify()
      applyTheme(next.theme)
    } catch {
      /* 未登录/接口失败：保持本地偏好 */
    }
  }

  function update(partial: Partial<Preferences>) {
    const current = get({ subscribe })
    const next = { ...current, ...partial }
    set(next)
    globalPrefs = next
    saveLocal(next)
    notify()
    applyTheme(next.theme)
    syncBackend(next)
  }

  return {
    subscribe,
    /** 应用启动时调用：先应用本地偏好，再尝试从服务端拉取覆盖 */
    async initialize() {
      applyTheme(initial.theme)
      globalPrefs = initial
      await refresh()
    },
    refresh,
    updateTheme: (theme: ThemeMode) => update({ theme }),
    updateTimezoneMode: (timezoneMode: TimezoneMode) => update({ timezoneMode }),
    updateTimezoneOffset: (timezoneOffset: number) => update({ timezoneOffset }),
    updateNewConvPosition: (newConvPosition: NewConvPosition) => update({ newConvPosition }),
  }
}

export const preferencesStore = createPreferencesStore()
