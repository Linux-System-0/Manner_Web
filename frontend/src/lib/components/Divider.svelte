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
  // Divider：分割线
  import type { Snippet } from 'svelte'

  let {
    dashed = false,
    orientation = 'center',
    children,
    style = '',
  }: {
    dashed?: boolean
    orientation?: 'left' | 'center' | 'right'
    children?: Snippet
    style?: string
  } = $props()
</script>

<div class="ant-divider ant-divider-horizontal ant-divider-with-text ant-divider-{orientation}" style={style}>
  {#if children}
    <span class="ant-divider-inner-text">{@render children()}</span>
  {/if}
</div>

<style>
  .ant-divider {
    box-sizing: border-box;
    margin: 24px 0;
    padding: 0;
    color: var(--ant-color-text);
    font-size: 14px;
    line-height: 1.5715;
    border-top: 1px solid var(--ant-color-split);
  }
  .ant-divider-dashed {
    border-top-style: dashed;
  }
  .ant-divider-with-text {
    display: flex;
    align-items: center;
    border-top: 0;
  }
  .ant-divider-with-text::before,
  .ant-divider-with-text::after {
    content: '';
    flex: 1;
    border-top: 1px solid var(--ant-color-split);
  }
  .ant-divider-dashed::before,
  .ant-divider-dashed::after {
    border-top-style: dashed;
  }
  .ant-divider-inner-text {
    padding: 0 1em;
    color: var(--ant-color-text-secondary);
    font-size: 14px;
  }
  .ant-divider-left::before {
    flex: 0.05;
  }
  .ant-divider-right::after {
    flex: 0.05;
  }
</style>
