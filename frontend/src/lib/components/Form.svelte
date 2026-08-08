<script lang="ts">
  // Form：表单容器（不做校验引擎，仅布局）
  import type { Snippet } from 'svelte'

  let {
    onSubmit,
    children,
    style = '',
    labelCol,
    class: className = '',
  }: {
    onSubmit?: (e: SubmitEvent) => void
    children?: Snippet
    style?: string
    labelCol?: { span?: number }
    class?: string
  } = $props()
</script>

<form class="ant-form ant-form-horizontal {className}" style="{style}" onsubmit={onSubmit}>
  {#if children}{@render children()}{/if}
</form>

<style>
  .ant-form {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    color: var(--ant-color-text);
    font-size: 14px;
    line-height: 1.5715;
  }
</style>
