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
  // Popconfirm：气泡确认（点击触发，二次确认）
  import type { Snippet } from 'svelte'
  import { onMount } from 'svelte'
  import { t } from '$lib/i18n'
  import Button from './Button.svelte'
  import { Icon } from '$lib/icons'

  let {
    title = t('common.confirmOp'),
    description = '',
    onConfirm,
    onCancel,
    okText = t('common.ok'),
    cancelText = t('common.cancel'),
    okDanger = false,
    children,
    disabled = false,
  }: {
    title?: string
    description?: string
    onConfirm?: () => void | Promise<void>
    onCancel?: () => void
    okText?: string
    cancelText?: string
    okDanger?: boolean
    children?: Snippet
    disabled?: boolean
  } = $props()

  let open = $state(false)
  let rootEl: HTMLSpanElement | undefined = $state()
  let panelEl: HTMLDivElement | undefined = $state()
  let loading = $state(false)

  function toggle() {
    if (!disabled) open = !open
  }

  async function confirm() {
    loading = true
    try {
      await onConfirm?.()
      open = false
    } finally {
      loading = false
    }
  }

  function cancel() {
    open = false
    onCancel?.()
  }

  function onDocClick(e: MouseEvent) {
    if (rootEl && !rootEl.contains(e.target as Node)) open = false
  }

  onMount(() => {
    document.addEventListener('click', onDocClick)
    return () => document.removeEventListener('click', onDocClick)
  })

  $effect(() => {
    if (open && panelEl && rootEl) {
      const r = rootEl.getBoundingClientRect()
      panelEl.style.top = r.bottom + 6 + 'px'
      panelEl.style.left = r.left + 'px'
    }
  })
</script>

<span
  bind:this={rootEl}
  style="display:inline-flex"
  role="button"
  tabindex={0}
  aria-haspopup="dialog"
  aria-expanded={open}
  onclick={toggle}
  onkeydown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      toggle()
    }
  }}
>
  {#if children}{@render children()}{/if}
</span>

{#if open}
  <div class="ant-popover ant-popover-placement-bottom" bind:this={panelEl} style="position:fixed;z-index:1060;min-width:180px">
    <div class="ant-popover-content">
      <div class="ant-popover-arrow"></div>
      <div class="ant-popover-inner">
        <div class="ant-popover-inner-content">
          <div class="ant-popconfirm-message" style="display:flex;gap:8px;align-items:flex-start">
            <span style="color:var(--ant-color-warning);display:inline-flex;margin-top:2px"><Icon name="exclamation-circle" /></span>
            <div>
              <div style="color:var(--ant-color-text);font-size:14px;line-height:1.5715">{title}</div>
              {#if description}<div style="color:var(--ant-color-text-secondary);font-size:14px;margin-top:4px">{description}</div>{/if}
            </div>
          </div>
          <div style="display:flex;justify-content:flex-end;gap:8px;margin-top:12px">
            <Button size="small" tooltip={t('common.cancelOp')} onClick={cancel}>{cancelText}</Button>
            <Button size="small" type="primary" danger={okDanger} loading={loading} tooltip={t('common.confirmOp')} onClick={confirm}>{okText}</Button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .ant-popover-inner {
    background: var(--ant-color-bg-elevated);
    border-radius: var(--ant-border-radius-lg);
    box-shadow: var(--ant-box-shadow);
    padding: 12px 16px;
  }
  .ant-popover-arrow {
    position: absolute;
    top: -4px;
    left: 24px;
    width: 10px;
    height: 10px;
    background: var(--ant-color-bg-elevated);
    transform: rotate(45deg);
  }
</style>
