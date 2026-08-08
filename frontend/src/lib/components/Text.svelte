<script lang="ts">
  // Text：文本（type 着色 / ellipsis 省略）
  import type { Snippet } from 'svelte'

  let {
    type,
    ellipsis = false,
    children,
    style = '',
    title = '',
  }: {
    type?: 'secondary' | 'success' | 'warning' | 'danger'
    ellipsis?: boolean
    children?: Snippet
    style?: string
    title?: string
  } = $props()

  let color = $derived(
    type === 'secondary'
      ? 'var(--ant-color-text-secondary)'
      : type === 'success'
        ? 'var(--ant-color-success)'
        : type === 'warning'
          ? 'var(--ant-color-warning)'
          : type === 'danger'
            ? 'var(--ant-color-error)'
            : 'inherit',
  )
</script>

<span
  class="ant-typography"
  class:ant-typography-ellipsis={ellipsis}
  style="color:{color};{ellipsis ? 'display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;' : ''}{style}"
  {title}
>
  {#if children}{@render children()}{/if}
</span>
