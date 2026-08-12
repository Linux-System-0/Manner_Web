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
