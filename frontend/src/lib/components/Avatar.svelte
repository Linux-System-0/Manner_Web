<script lang="ts">
  // Avatar：头像（图片或文字首字）
  import type { Snippet } from 'svelte'

  let {
    src,
    size = 32,
    shape = 'circle',
    children,
    style = '',
  }: {
    src?: string | null
    size?: number | 'small' | 'large'
    shape?: 'circle' | 'square'
    children?: Snippet
    style?: string
  } = $props()

  let px = $derived(
    typeof size === 'number' ? size : size === 'large' ? 40 : size === 'small' ? 24 : 32,
  )
</script>

<span
  class="ant-avatar ant-avatar-{shape}"
  style="width:{px}px;height:{px}px;font-size:{px / 2}px;line-height:{px}px;background:var(--ant-color-primary);color:#fff;{style}"
>
  {#if src}
    <img src={src} alt="avatar" style="width:100%;height:100%;object-fit:cover" />
  {:else if children}
    {@render children()}
  {/if}
</span>

<style>
  .ant-avatar {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    vertical-align: middle;
    text-align: center;
    background: #ccc;
    color: #fff;
    white-space: nowrap;
    position: relative;
    font-weight: 400;
  }
  .ant-avatar-circle {
    border-radius: 50%;
  }
  .ant-avatar-square {
    border-radius: var(--ant-border-radius-sm);
  }
</style>
