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
  // Row：flex 栅格行
  import type { Snippet } from 'svelte'

  let {
    gutter = 0,
    justify = 'start',
    align = 'top',
    children,
    style = '',
  }: {
    gutter?: number | [number, number]
    justify?: 'start' | 'center' | 'end' | 'space-between' | 'space-around'
    align?: 'top' | 'middle' | 'bottom'
    children?: Snippet
    style?: string
  } = $props()
</script>

<div
  class="ant-row"
  style="display:flex;flex-wrap:wrap;gap:0;justify-content:{justify === 'space-between' ? 'space-between' : justify === 'space-around' ? 'space-around' : justify === 'center' ? 'center' : justify === 'end' ? 'flex-end' : 'flex-start'};align-items:{align === 'middle' ? 'center' : align === 'bottom' ? 'flex-end' : 'flex-start'};margin-left:{-(typeof gutter === 'number' ? gutter : gutter[0]) / 2}px;margin-right:{-(typeof gutter === 'number' ? gutter : gutter[0]) / 2}px;{style}"
>
  {#if children}{@render children()}{/if}
</div>

<style>
  .ant-row {
    box-sizing: border-box;
  }
</style>
