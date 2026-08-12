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

// 国际化模块（Svelte 5 runes，模块级 $state 跨组件响应式）
// - 语言包：src/lib/i18n/locales/<locale>.json，添加语言包只需新增一个 JSON 文件
//   （文件名即语言代码，如 fr-FR.json），本模块用 import.meta.glob 自动加载。
// - en-US 为兜底语言：任何语言包缺失的 key 都回退到 en-US，再缺失则原样返回 key。
// - 语言模式：system（跟随系统/浏览器语言） | 具体语言代码，由系统设置 default_language 决定。
// - 使用：`import { t } from '$lib/i18n'`，模板中调用 t('key') 自动响应语言切换。
const modules = import.meta.glob('./locales/*.json', {
  eager: true,
  import: 'default',
}) as Record<string, Record<string, string>>

export type Locale = string
/** 语言模式：system（跟随系统） | 具体语言代码 */
export type LanguageMode = 'system' | Locale

const messages: Record<string, Record<string, string>> = {}
const SUPPORTED_LOCALES: string[] = []

for (const [path, dict] of Object.entries(modules)) {
  const code = path.split('/').pop()!.replace(/\.json$/, '')
  messages[code] = dict
  SUPPORTED_LOCALES.push(code)
}

// 兜底语言必须是 en-US（t() 缺 key 时回退到这里）
if (!SUPPORTED_LOCALES.includes('en-US')) {
  SUPPORTED_LOCALES.unshift('en-US')
}

// Svelte 5 禁止直接导出被重新赋值的 $state 变量（state_invalid_export），
// 故以对象承载：仅修改对象属性，跨组件读取自动响应。
export const i18nState = $state({
  /** 当前生效语言（仅含实际语言代码，不含 system） */
  locale: 'en-US' as Locale,
  /** 当前语言模式（system / 具体语言），与系统设置 default_language 对齐 */
  languageMode: 'system' as LanguageMode,
})

/** 支持的语言代码列表（由 locales/ 目录自动推导） */
export function supportedLocales(): Locale[] {
  return [...SUPPORTED_LOCALES]
}

/** 语言包在自身语言下的显示名（约定 key：i18n.languageName），缺失则回退语言代码 */
export function localeDisplayName(code: Locale): string {
  return messages[code]?.['i18n.languageName'] || code
}

/**
 * 从浏览器/系统语言解析目标语言代码：
 * 1) 精确匹配（navigator.language 与某语言包代码一致）；
 * 2) 语言前缀匹配（如 navigator.language=zh-TW 时匹配 zh-CN 语言包）；
 * 3) 均无匹配则回退到兜底语言 en-US。
 */
export function resolveLocale(mode: LanguageMode): Locale {
  if (mode !== 'system') {
    return SUPPORTED_LOCALES.includes(mode) ? mode : 'en-US'
  }
  if (typeof navigator === 'undefined') return 'en-US'
  const nav = (navigator.language || '').toLowerCase()
  if (!nav) return 'en-US'
  const exact = SUPPORTED_LOCALES.find((l) => l.toLowerCase() === nav)
  if (exact) return exact
  const base = nav.split('-')[0]
  const byPrefix = SUPPORTED_LOCALES.find((l) => l.toLowerCase().startsWith(base))
  return byPrefix || 'en-US'
}

/** 应用语言模式：写入响应式状态 + html[lang] */
export function setLocale(mode: LanguageMode) {
  i18nState.languageMode = mode
  i18nState.locale = resolveLocale(mode)
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('lang', i18nState.locale)
  }
}

/** 翻译函数：取当前语言包，缺失时回退 en-US，再缺失时原样返回 key */
export function t(key: string, params?: Record<string, string | number>): string {
  const dict = messages[i18nState.locale]
  let text: string = dict[key] ?? messages['en-US'][key] ?? key
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      text = text.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v))
    }
  }
  return text
}

/** 组件内获取当前语言包类型（供需要穷举 key 的场景） */
export function currentMessages(): Record<string, string> {
  return messages[i18nState.locale]
}
