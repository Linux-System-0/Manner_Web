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
