<script lang="ts">
  // Tag：标签（antd 视觉；color 为预设色名或任意颜色）
  import { Icon } from '$lib/icons'
  import type { Snippet } from 'svelte'

  const PRESET: Record<string, [string, string]> = {
    blue: ['rgba(22,119,255,0.1)', 'var(--ant-color-primary)'],
    green: ['rgba(82,196,26,0.1)', 'var(--ant-color-success)'],
    red: ['rgba(255,77,79,0.1)', 'var(--ant-color-error)'],
    orange: ['rgba(250,140,22,0.1)', 'var(--ant-color-warning)'],
    gold: ['rgba(250,173,20,0.1)', '#d48806'],
    cyan: ['rgba(19,194,194,0.1)', '#13c2c2'],
    purple: ['rgba(114,46,209,0.1)', '#722ed1'],
    magenta: ['rgba(235,47,150,0.1)', '#eb2f96'],
    default: ['var(--ant-color-fill-quaternary)', 'var(--ant-color-text)'],
  }

  let {
    color = 'default',
    closable = false,
    onClose,
    children,
    style = '',
  }: {
    color?: string
    closable?: boolean
    onClose?: () => void
    children?: Snippet
    style?: string
  } = $props()

  let palette = $derived(PRESET[color] ?? [color + '1f', color] as [string, string])
</script>

<span class="ant-tag" style="background:{palette[0]};color:{palette[1]};{style}">
  {#if children}{@render children()}{/if}
  {#if closable}
    <span
      class="ant-tag-close-icon"
      role="button"
      tabindex={0}
      aria-label="关闭"
      onclick={onClose}
      onkeydown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault()
          onClose?.()
        }
      }}
    ><Icon name="close" style="font-size:10px" /></span>
  {/if}
</span>

<style>
  .ant-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    box-sizing: border-box;
    margin: 0 8px 0 0;
    padding: 0 7px;
    font-size: var(--ant-font-size-sm);
    line-height: 20px;
    white-space: nowrap;
    border-radius: var(--ant-border-radius-sm);
    cursor: default;
    vertical-align: middle;
  }
  .ant-tag-close-icon {
    display: inline-flex;
    cursor: pointer;
    opacity: 0.65;
  }
  .ant-tag-close-icon:hover {
    opacity: 1;
  }
</style>
