<script lang="ts">
  // FormItem：表单项（label + required 星号 + 错误提示）
  import type { Snippet } from 'svelte'

  let {
    label = '',
    htmlFor = '',
    required = false,
    error = '',
    children,
    style = '',
    extra = '',
  }: {
    label?: string
    htmlFor?: string
    required?: boolean
    error?: string
    children?: Snippet
    style?: string
    extra?: string
  } = $props()
</script>

<div class="ant-form-item" class:ant-form-item-has-error={!!error} style="margin-bottom:24px;display:flex;{style}">
  {#if label}
    <div class="ant-form-item-label" style="flex:0 0 100px;padding-right:8px;text-align:right;line-height:32px">
      <!-- 通用容器：label 关联由调用方通过 htmlFor 控制，未传时无法静态关联 -->
      <!-- svelte-ignore a11y_label_has_associated_control -->
      <label class="ant-form-item-required" class:ant-form-item-required-mark={required} for={htmlFor || undefined}>
        {#if required}<span style="color:var(--ant-color-error);margin-right:4px">*</span>{/if}
        {label}
      </label>
    </div>
  {/if}
  <div class="ant-form-item-control" style="flex:1;min-width:0">
    <div class="ant-form-item-control-input">
      {#if children}{@render children()}{/if}
    </div>
    {#if error}
      <div class="ant-form-item-explain ant-form-item-explain-error" style="color:var(--ant-color-error);font-size:14px;line-height:1.5;margin-top:4px">
        {error}
      </div>
    {:else if extra}
      <div class="ant-form-item-extra" style="color:var(--ant-color-text-tertiary);font-size:14px;margin-top:4px">
        {extra}
      </div>
    {/if}
  </div>
</div>
