<script lang="ts">
  // Space：间距容器
  import type { Snippet } from 'svelte'

  let {
    direction = 'horizontal',
    size = 'middle',
    wrap = false,
    children,
    style = '',
    align = 'center',
  }: {
    direction?: 'horizontal' | 'vertical'
    size?: 'small' | 'middle' | 'large' | number
    wrap?: boolean
    children?: Snippet
    style?: string
    align?: 'start' | 'center' | 'end' | 'baseline' | 'stretch'
  } = $props()

  let gap = $derived(
    typeof size === 'number' ? `${size}px` : size === 'small' ? '8px' : size === 'large' ? '24px' : '16px',
  )
</script>

<div
  class="ant-space"
  style="display:flex;flex-direction:{direction === 'vertical' ? 'column' : 'row'};align-items:{align === 'start' ? 'flex-start' : align === 'end' ? 'flex-end' : align};gap:{gap};flex-wrap:{wrap ? 'wrap' : 'nowrap'};{style}"
>
  {#if children}{@render children()}{/if}
</div>

<style>
  .ant-space {
    box-sizing: border-box;
  }
</style>
