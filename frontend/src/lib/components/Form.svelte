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
