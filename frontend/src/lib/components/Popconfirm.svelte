<script lang="ts">
  // Popconfirm：气泡确认（点击触发，二次确认）
  import type { Snippet } from 'svelte'
  import { onMount } from 'svelte'
  import Button from './Button.svelte'
  import { Icon } from '$lib/icons'

  let {
    title = '确定执行此操作吗？',
    description = '',
    onConfirm,
    onCancel,
    okText = '确定',
    cancelText = '取消',
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
            <Button size="small" tooltip="取消操作" onClick={cancel}>{cancelText}</Button>
            <Button size="small" type="primary" danger={okDanger} loading={loading} tooltip="确认执行该操作" onClick={confirm}>{okText}</Button>
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
