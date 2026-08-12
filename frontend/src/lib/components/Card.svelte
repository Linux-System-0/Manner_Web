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
  // Card：卡片容器
  import type { Snippet } from 'svelte'

  let {
    title,
    extra,
    bordered = true,
    children,
    bodyStyle = '',
    style = '',
    class: className = '',
    hoverable = false,
  }: {
    title?: string | Snippet
    extra?: Snippet
    bordered?: boolean
    children?: Snippet
    bodyStyle?: string
    style?: string
    class?: string
    hoverable?: boolean
  } = $props()
</script>

<div class="ant-card {bordered ? '' : 'ant-card-bordered'} {hoverable ? 'ant-card-hoverable' : ''} {className}" style={style}>
  {#if title || extra}
    <div class="ant-card-head">
      <div class="ant-card-head-wrapper">
        {#if title}
          <div class="ant-card-head-title">
            {#if typeof title === 'string'}{title}{:else}{@render title()}{/if}
          </div>
        {/if}
        {#if extra}
          <div class="ant-card-extra">{@render extra()}</div>
        {/if}
      </div>
    </div>
  {/if}
  <div class="ant-card-body" style={bodyStyle}>
    {#if children}{@render children()}{/if}
  </div>
</div>

<style>
  .ant-card {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    color: var(--ant-color-text);
    font-size: var(--ant-font-size);
    border-radius: var(--ant-border-radius-lg);
    background: var(--ant-card-bg);
    border: 1px solid var(--ant-color-border-secondary);
  }
  .ant-card-bordered {
    border: none;
  }
  .ant-card-hoverable {
    cursor: pointer;
    transition: box-shadow 0.3s;
  }
  .ant-card-hoverable:hover {
    box-shadow: var(--ant-box-shadow);
  }
  .ant-card-head {
    min-height: 48px;
    margin-bottom: -1px;
    padding: 0 24px;
    border-bottom: 1px solid var(--ant-color-border-secondary);
    border-radius: var(--ant-border-radius-lg) var(--ant-border-radius-lg) 0 0;
  }
  .ant-card-head-wrapper {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 48px;
    gap: 12px;
  }
  .ant-card-head-title {
    flex: 1;
    font-size: 16px;
    font-weight: 600;
    color: var(--ant-color-text);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .ant-card-extra {
    color: var(--ant-color-text-secondary);
  }
  .ant-card-body {
    padding: 24px;
    border-radius: 0 0 var(--ant-border-radius-lg) var(--ant-border-radius-lg);
  }
</style>
