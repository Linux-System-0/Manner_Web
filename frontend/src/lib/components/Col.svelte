<script lang="ts">
  // Col：栅格列（基于 flex，span/24）
  import type { Snippet } from 'svelte'

  let {
    span,
    offset = 0,
    children,
    style = '',
  }: {
    span?: number
    offset?: number
    children?: Snippet
    style?: string
  } = $props()

  let width = $derived(span ? `width:${(span / 24) * 100}%;` : 'flex:1;')
</script>

<div class="ant-col" style="box-sizing:border-box;padding:0 {8}px;{width}margin-left:{offset ? `${(offset / 24) * 100}%` : 0};{style}">
  {#if children}{@render children()}{/if}
</div>
