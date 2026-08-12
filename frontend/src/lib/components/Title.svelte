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
  // Typography：Title / Text / Link（antd 视觉）
  import type { Snippet } from 'svelte'

  let {
    level = 1,
    children,
    style = '',
  }: { level?: 1 | 2 | 3 | 4 | 5; children?: Snippet; style?: string } = $props()
</script>

{#if level === 1}
  <h1 class="ant-typography" style="font-size:38px;line-height:1.23;margin:0 0 0.5em;color:var(--ant-color-text);font-weight:600;{style}">{@render children?.()}</h1>
{:else if level === 2}
  <h2 class="ant-typography" style="font-size:30px;line-height:1.35;margin:0 0 0.5em;color:var(--ant-color-text);font-weight:600;{style}">{@render children?.()}</h2>
{:else if level === 3}
  <h3 class="ant-typography" style="font-size:24px;line-height:1.35;margin:0 0 0.5em;color:var(--ant-color-text);font-weight:600;{style}">{@render children?.()}</h3>
{:else if level === 4}
  <h4 class="ant-typography" style="font-size:20px;line-height:1.4;margin:0 0 0.5em;color:var(--ant-color-text);font-weight:600;{style}">{@render children?.()}</h4>
{:else}
  <h5 class="ant-typography" style="font-size:16px;line-height:1.5;margin:0 0 0.5em;color:var(--ant-color-text);font-weight:600;{style}">{@render children?.()}</h5>
{/if}
