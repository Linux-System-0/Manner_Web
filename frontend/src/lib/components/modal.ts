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

// modal.confirm：承诺式确认弹窗（antd 视觉）
// 用法：const ok = await modal.confirm({ title, content, onOk?, danger? })
import { writable } from 'svelte/store'
import { mount } from 'svelte'
import ConfirmModal from './ConfirmModal.svelte'

export interface ConfirmOptions {
  title: string
  content?: string
  okText?: string
  cancelText?: string
  okDanger?: boolean
  onOk?: () => Promise<void> | void
}

interface ConfirmState extends ConfirmOptions {
  id: number
}

export const confirmState = writable<ConfirmState | null>(null)

let resolver: ((ok: boolean) => void) | null = null
let mounted = false
let seq = 0

export function confirm(opts: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    resolver = resolve
    if (!mounted) {
      const el = document.createElement('div')
      document.body.appendChild(el)
      mount(ConfirmModal, { target: el })
      mounted = true
    }
    confirmState.set({ ...opts, id: ++seq })
  })
}

/** 供 ConfirmModal 内部调用：结算 Promise 并关闭 */
export function settleConfirm(ok: boolean): void {
  confirmState.set(null)
  resolver?.(ok)
  resolver = null
}

export const modal = { confirm }
