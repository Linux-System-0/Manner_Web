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
