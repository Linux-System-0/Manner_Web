// i18n 入口：统一从这里导出（runes 状态实际在 state.svelte.ts 中）
export {
  i18nState,
  setLocale,
  resolveLocale,
  supportedLocales,
  localeDisplayName,
  t,
  currentMessages,
} from './state.svelte'
export type { Locale, LanguageMode } from './state.svelte'
