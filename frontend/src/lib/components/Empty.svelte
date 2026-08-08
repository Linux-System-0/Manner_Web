<script lang="ts">
  // Empty：空态占位
  import { Icon } from '$lib/icons'
  import type { Snippet } from 'svelte'

  let {
    description = '暂无数据',
    image,
    children,
  }: {
    description?: string
    image?: Snippet
    children?: Snippet
  } = $props()
</script>

<div class="ant-empty">
  <div class="ant-empty-image">
    {#if image}{@render image()}{:else}<Icon name="message" style="font-size:48px;color:var(--ant-color-text-quaternary)" />{/if}
  </div>
  <p class="ant-empty-description" style="color:var(--ant-color-text-tertiary);font-size:14px;margin:8px 0 0">{description}</p>
  {#if children}
    <div class="ant-empty-footer" style="margin-top:16px">{@render children()}</div>
  {/if}
</div>

<style>
  .ant-empty {
    text-align: center;
    padding: 16px 0;
  }
  .ant-empty-image {
    display: flex;
    justify-content: center;
    line-height: 0;
  }
</style>
