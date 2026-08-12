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
  // Badge：角标（count / dot）
  import type { Snippet } from 'svelte'

  let {
    count,
    dot = false,
    color = 'var(--ant-color-error)',
    children,
    overflowCount = 99,
  }: {
    count?: number
    dot?: boolean
    color?: string
    children?: Snippet
    overflowCount?: number
  } = $props()

  let display = $derived(
    count === undefined || count === null || count === 0 ? '' : count > overflowCount ? `${overflowCount}+` : String(count),
  )
</script>

<span class="ant-badge" style="position:relative;display:inline-flex">
  {#if children}{@render children()}{/if}
  {#if dot && !display}
    <sup class="ant-badge-dot" style="background:{color}"></sup>
  {:else if display}
    <sup class="ant-badge-count" style="background:{color}">{display}</sup>
  {/if}
</span>

<style>
  .ant-badge-count,
  .ant-badge-dot {
    position: absolute;
    top: -8px;
    right: -10px;
    z-index: 1;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    color: #fff;
    font-size: 12px;
    line-height: 18px;
    text-align: center;
    border-radius: 9px;
    box-shadow: 0 0 0 1px var(--ant-color-bg-container);
    box-sizing: border-box;
  }
  .ant-badge-dot {
    width: 8px;
    min-width: 8px;
    height: 8px;
    padding: 0;
    border-radius: 50%;
    top: -2px;
    right: -4px;
  }
</style>
