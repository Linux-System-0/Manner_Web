<script lang="ts">
  // Result：结果页（403/404/500/success）
  import type { Snippet } from 'svelte'
  import { Icon } from '$lib/icons'

  let {
    status = 'info',
    title = '',
    subTitle = '',
    extra,
    children,
  }: {
    status?: '403' | '404' | '500' | 'success' | 'info' | 'warning' | 'error'
    title?: string
    subTitle?: string
    extra?: Snippet
    children?: Snippet
  } = $props()

  let icon = $derived(
    status === '403' ? { name: 'stop', color: 'var(--ant-color-warning)' }
      : status === '404' ? { name: 'search', color: 'var(--ant-color-warning)' }
      : status === '500' ? { name: 'close-circle', color: 'var(--ant-color-error)' }
      : status === 'success' ? { name: 'check-circle', color: 'var(--ant-color-success)' }
      : status === 'warning' ? { name: 'exclamation-circle', color: 'var(--ant-color-warning)' }
      : status === 'error' ? { name: 'close-circle', color: 'var(--ant-color-error)' }
      : { name: 'info-circle', color: 'var(--ant-color-primary)' },
  )
</script>

<div class="ant-result" style="padding:48px 32px;text-align:center">
  <div class="ant-result-icon" style="margin-bottom:24px;line-height:1">
    <span style="color:{icon.color};display:inline-flex">
      <Icon name={icon.name} style="font-size:72px" />
    </span>
  </div>
  <div class="ant-result-title" style="font-size:24px;color:var(--ant-color-text);font-weight:600;line-height:1.8">{title}</div>
  {#if subTitle}
    <div class="ant-result-subtitle" style="font-size:14px;color:var(--ant-color-text-secondary);margin-top:8px">{subTitle}</div>
  {/if}
  {#if extra}
    <div class="ant-result-extra" style="margin-top:24px">{@render extra()}</div>
  {/if}
  {#if children}<div class="ant-result-content" style="margin-top:24px">{@render children()}</div>{/if}
</div>
