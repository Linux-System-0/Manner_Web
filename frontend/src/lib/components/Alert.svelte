<!--
Manner_Web - 可以在 Linux 系统上运行的企业管理系统
Copyright (C) 2026 Linux-System-0(Github) / 一架在Linux上起飞的A320(Bilibili) <ls0_1@qq.com>

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
-->

<script lang="ts">
  // Alert：警告提示（antd 视觉）
  import type { Snippet } from 'svelte'
  import { t } from '$lib/i18n'
  import { Icon } from '$lib/icons'

  let {
    type = 'info',
    message = '',
    description,
    showIcon = true,
    closable = false,
    onClose,
    children,
    style = '',
  }: {
    type?: 'success' | 'info' | 'warning' | 'error'
    message?: string
    description?: string
    showIcon?: boolean
    closable?: boolean
    onClose?: () => void
    children?: Snippet
    style?: string
  } = $props()

  const ICONS: Record<string, string> = {
    success: 'check-circle',
    info: 'info-circle',
    warning: 'exclamation-circle',
    error: 'close-circle',
  }
  const ICON_NAMES: Record<string, string> = {
    success: 'check',
    info: 'info-circle',
    warning: 'exclamation-circle',
    error: 'close',
  }
  const COLORS: Record<string, string> = {
    success: 'var(--ant-color-success)',
    info: 'var(--ant-color-primary)',
    warning: 'var(--ant-color-warning)',
    error: 'var(--ant-color-error)',
  }
</script>

<div
  class="ant-alert ant-alert-{type}"
  style="display:flex;gap:8px;padding:8px 15px;border-radius:var(--ant-border-radius);border:1px solid {COLORS[type]}33;background:{COLORS[type]}0d;color:var(--ant-color-text);{style}"
>
  {#if showIcon}
    <span style="color:{COLORS[type]};line-height:1.4;display:inline-flex">
      <Icon name={ICON_NAMES[type]} style="font-size:14px" />
    </span>
  {/if}
  <div style="flex:1;min-width:0">
    <div class="ant-alert-message" style="font-size:14px;line-height:1.5715">{message}</div>
    {#if description}
      <div class="ant-alert-description" style="font-size:14px;line-height:1.5715;color:var(--ant-color-text-secondary)">{description}</div>
    {/if}
    {#if children}
      <div class="ant-alert-action" style="margin-top:8px">{@render children()}</div>
    {/if}
  </div>
  {#if closable}
    <span
      class="ant-alert-close-icon"
      role="button"
      tabindex={0}
      aria-label={t('common.closeBtn')}
      style="cursor:pointer;color:var(--ant-color-text-tertiary);display:inline-flex"
      onclick={onClose}
      onkeydown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault()
          onClose?.()
        }
      }}
    >
      <Icon name="close" style="font-size:12px" />
    </span>
  {/if}
</div>
