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
