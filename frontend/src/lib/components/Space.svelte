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
  // Space：间距容器
  import type { Snippet } from 'svelte'

  let {
    direction = 'horizontal',
    size = 'middle',
    wrap = false,
    children,
    style = '',
    align = 'center',
  }: {
    direction?: 'horizontal' | 'vertical'
    size?: 'small' | 'middle' | 'large' | number
    wrap?: boolean
    children?: Snippet
    style?: string
    align?: 'start' | 'center' | 'end' | 'baseline' | 'stretch'
  } = $props()

  let gap = $derived(
    typeof size === 'number' ? `${size}px` : size === 'small' ? '8px' : size === 'large' ? '24px' : '16px',
  )
</script>

<div
  class="ant-space"
  style="display:flex;flex-direction:{direction === 'vertical' ? 'column' : 'row'};align-items:{align === 'start' ? 'flex-start' : align === 'end' ? 'flex-end' : align};gap:{gap};flex-wrap:{wrap ? 'wrap' : 'nowrap'};{style}"
>
  {#if children}{@render children()}{/if}
</div>

<style>
  .ant-space {
    box-sizing: border-box;
  }
</style>
