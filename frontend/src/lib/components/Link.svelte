<script lang="ts">
  // Link：链接（antd Link 视觉）
  import type { Snippet } from 'svelte'

  let {
    href,
    onclick,
    children,
    style = '',
  }: {
    href?: string
    onclick?: (e: MouseEvent) => void
    children?: Snippet
    style?: string
  } = $props()
</script>

<a class="ant-typography" style="color:var(--ant-color-primary);cursor:pointer;text-decoration:none;transition:color 0.2s;{style}" href={href} onclick={onclick}>
  {#if children}{@render children()}{/if}
</a>

<style>
  a.ant-typography:hover {
    color: var(--ant-color-primary-hover);
  }
</style>
