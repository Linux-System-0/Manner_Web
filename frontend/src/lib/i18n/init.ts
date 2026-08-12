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
