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

// i18n 初始化：语言优先级 = 个人设置（preferences.language）> 系统默认（default_language）> 跟随系统。
// 登录页与受保护页共用（登录页不经 Layout 包裹，故在此集中处理）。
import { setLocale, type LanguageMode } from './index'
import { getLoginPage } from '@/api/system'
import { getGlobalPrefs, defaultPrefs } from '@/stores/preferences'

let initialized = false
let applying = false

/**
 * 应用语言：
 * - 个人设置 preferences.language 优先（仅针对该员工本人，覆盖系统默认）；
 * - 否则取系统默认 default_language（system=跟随系统，具体语言代码=手动）；
 * - 均未配置时回退「跟随系统」。
 *
 * 登录前（登录页）：本地可能有历史个人偏好；登录后 preferencesStore.refresh()
 * 会拉取该用户服务端偏好，此时再次调用 initI18n(true) 以个人设置为准。
 */
export async function initI18n(force = false): Promise<void> {
  if (initialized && !force) return
  if (applying) return
  applying = true
  try {
    // 个人偏好（本地已加载的最新值）。
    const prefs = getGlobalPrefs()
    let mode: LanguageMode =
      prefs && prefs.language ? (prefs.language as LanguageMode) : (defaultPrefs.language as LanguageMode)

    if (mode === 'system') {
      // 未设个人语言 → 系统默认。
      const res = await getLoginPage()
      mode = (res.data?.default_language as LanguageMode | undefined) || 'system'
    }
    setLocale(mode)
    initialized = true
  } catch {
    const prefs = getGlobalPrefs()
    const mode: LanguageMode =
      prefs && prefs.language ? (prefs.language as LanguageMode) : 'system'
    setLocale(mode)
    initialized = true
  } finally {
    applying = false
  }
}
