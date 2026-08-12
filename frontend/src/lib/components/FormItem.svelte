<!--
Manner_Web - 可以在 Linux 系统上运行的企业管理系统
Copyright (C) 2026 Linux-System-0(Github) / 一架在Linux上起飞的A320(Bilibili) <ls0_1@qq.com>

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
-->

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
