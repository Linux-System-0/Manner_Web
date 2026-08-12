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

// message：轻量 toast 提示（antd 视觉：顶部居中、自动消失）
// 用法：message.success('操作成功') / message.error(msg) / message.warning(msg) / message.info(msg)

type MessageType = 'success' | 'error' | 'warning' | 'info'

interface Notice {
  id: number
  type: MessageType
  text: string
}

let container: HTMLDivElement | null = null
let notices: Notice[] = []
let seq = 0

const TYPE_ICON: Record<MessageType, string> = {
  success: '<svg viewBox="64 64 896 896" width="16" height="16" fill="currentColor"><path d="M912 190h-69.9c-9.8 0-19.1 4.5-25.1 12.2L404.7 724.5 207 474a32 32 0 00-25.1-12.2H112c-6.7 0-10.4 7.7-6.3 12.9l273.9 347c12.8 16.2 37.4 16.2 50.3 0l488.4-618.9c4.1-5.1.4-12.8-6.3-12.8z"/></svg>',
  error: '<svg viewBox="64 64 896 896" width="16" height="16" fill="currentColor"><path d="M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64zm165.4 618.2l-66-.3L512 563.4l-99.3 118.4-66.1.3c-4.4 0-8-3.6-8-8 0-1.9.7-3.7 1.9-5.2l130.1-155L340.5 359a8.32 8.32 0 01-1.9-5.2c0-4.4 3.6-8 8-8l66.1.3L512 464.6l99.3-118.4 66-.3c4.4 0 8 3.6 8 8 0 1.9-.7 3.7-1.9 5.2L553.5 514l130 155c1.2 1.5 1.9 3.3 1.9 5.2 0 4.4-3.6 8-8 8z"/></svg>',
  warning: '<svg viewBox="64 64 896 896" width="16" height="16" fill="currentColor"><path d="M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64zm-32 232c0-4.4 3.6-8 8-8h48c4.4 0 8 3.6 8 8v272c0 4.4-3.6 8-8 8h-48c-4.4 0-8-3.6-8-8V296zm32 440a48.01 48.01 0 010-96 48.01 48.01 0 010 96z"/></svg>',
  info: '<svg viewBox="64 64 896 896" width="16" height="16" fill="currentColor"><path d="M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64zm32 664c0 4.4-3.6 8-8 8h-48c-4.4 0-8-3.6-8-8V456c0-4.4 3.6-8 8-8h48c4.4 0 8 3.6 8 8v272zm-32-344a48.01 48.01 0 010-96 48.01 48.01 0 010 96z"/></svg>',
}

function ensureContainer(): HTMLDivElement {
  if (container && document.body.contains(container)) return container
  container = document.createElement('div')
  container.className = 'ant-message'
  container.style.cssText =
    'position:fixed;top:16px;left:0;right:0;z-index:1010;display:flex;flex-direction:column;align-items:center;pointer-events:none;'
  document.body.appendChild(container)
  return container
}

function render() {
  const c = ensureContainer()
  c.innerHTML = ''
  for (const n of notices) {
    const el = document.createElement('div')
    el.className = 'ant-message-notice'
    el.style.cssText =
      'pointer-events:auto;margin-bottom:8px;animation:antMessageIn 0.2s ease;'
    el.innerHTML = `<div class="ant-message-notice-content" style="display:flex;align-items:center;gap:8px;padding:9px 12px;background:var(--ant-color-bg-elevated);border-radius:var(--ant-border-radius-lg);box-shadow:var(--ant-box-shadow);font-size:14px;color:var(--ant-color-text);">
      <span class="ant-message-custom-content ant-message-${n.type}" style="display:flex;align-items:center;gap:8px;color:var(--ant-color-${n.type === 'success' ? 'success' : n.type === 'error' ? 'error' : n.type === 'warning' ? 'warning' : 'info'})">
        ${TYPE_ICON[n.type]}<span>${escapeHtml(n.text)}</span>
      </span>
    </div>`
    el.addEventListener('click', () => {
      remove(n.id)
    })
    c.appendChild(el)
  }
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (m) => {
    const map: Record<string, string> = {
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      '"': '&quot;',
      "'": '&#39;',
    }
    return map[m]
  })
}

function remove(id: number) {
  notices = notices.filter((n) => n.id !== id)
  if (notices.length === 0) {
    container?.remove()
    container = null
    return
  }
  render()
}

function show(type: MessageType, text: string, duration = 3): void {
  const id = ++seq
  notices.push({ id, type, text })
  render()
  if (duration > 0) {
    setTimeout(() => remove(id), duration * 1000)
  }
}

// 注入进场动画（一次）
if (typeof document !== 'undefined' && !document.getElementById('ant-message-anim')) {
  const style = document.createElement('style')
  style.id = 'ant-message-anim'
  style.textContent = '@keyframes antMessageIn{from{opacity:0;transform:translateY(-10px)}to{opacity:1;transform:translateY(0)}}'
  document.head.appendChild(style)
}

export const message = {
  success: (text: string, duration?: number) => show('success', text, duration),
  error: (text: string, duration?: number) => show('error', text, duration),
  warning: (text: string, duration?: number) => show('warning', text, duration),
  info: (text: string, duration?: number) => show('info', text, duration),
}
