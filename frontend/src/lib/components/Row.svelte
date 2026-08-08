<script lang="ts">
  // Row：flex 栅格行
  import type { Snippet } from 'svelte'

  let {
    gutter = 0,
    justify = 'start',
    align = 'top',
    children,
    style = '',
  }: {
    gutter?: number | [number, number]
    justify?: 'start' | 'center' | 'end' | 'space-between' | 'space-around'
    align?: 'top' | 'middle' | 'bottom'
    children?: Snippet
    style?: string
  } = $props()
</script>

<div
  class="ant-row"
  style="display:flex;flex-wrap:wrap;gap:0;justify-content:{justify === 'space-between' ? 'space-between' : justify === 'space-around' ? 'space-around' : justify === 'center' ? 'center' : justify === 'end' ? 'flex-end' : 'flex-start'};align-items:{align === 'middle' ? 'center' : align === 'bottom' ? 'flex-end' : 'flex-start'};margin-left:{-(typeof gutter === 'number' ? gutter : gutter[0]) / 2}px;margin-right:{-(typeof gutter === 'number' ? gutter : gutter[0]) / 2}px;{style}"
>
  {#if children}{@render children()}{/if}
</div>

<style>
  .ant-row {
    box-sizing: border-box;
  }
</style>
