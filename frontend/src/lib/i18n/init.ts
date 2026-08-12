// i18n 初始化：从公开的登录页配置接口读取 default_language 并应用。
// 登录页与受保护页共用（登录页不经 Layout 包裹，故在此集中处理）。
import { setLocale, type LanguageMode } from './index'
import { getLoginPage } from '@/api/system'

let initialized = false
let applying = false

/**
 * 应用默认语言：
 * - 若后台已配置 default_language，则按其解析（system=跟随系统，具体语言代码=手动）；
 * - 未配置时回退为「跟随系统」。
 */
export async function initI18n(force = false): Promise<void> {
  if (initialized && !force) return
  if (applying) return
  applying = true
  try {
    const res = await getLoginPage()
    const mode = (res.data?.default_language as LanguageMode | undefined) || 'system'
    setLocale(mode)
    initialized = true
  } catch {
    setLocale('system')
    initialized = true
  } finally {
    applying = false
  }
}
