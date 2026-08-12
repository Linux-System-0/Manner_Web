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
  // Menu：导航菜单（dark inline 风格，侧边栏用）
  import { Icon } from '$lib/icons'
  import Tooltip from './Tooltip.svelte'

  export interface MenuItem {
    key: string
    label: string
    icon?: string
    danger?: boolean
  }

  let {
    items = [] as MenuItem[],
    selectedKeys = [] as string[],
    theme = 'dark',
    onClick,
    style = '',
    collapsed = false,
  }: {
    items?: MenuItem[]
    selectedKeys?: string[]
    theme?: 'dark' | 'light'
    onClick?: (key: string) => void
    style?: string
    collapsed?: boolean
  } = $props()
</script>

<ul class="ant-menu ant-menu-root ant-menu-inline ant-menu-{theme}" class:ant-menu-collapsed={collapsed} style="{style}">
  {#each items as item (item.key)}
    {#snippet itemContent()}
      <li
        class="ant-menu-item"
        class:ant-menu-item-selected={selectedKeys.includes(item.key)}
        class:ant-menu-item-danger={item.danger}
        role="menuitem"
        tabindex={selectedKeys.includes(item.key) ? 0 : -1}
        onclick={() => onClick?.(item.key)}
        onkeydown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault()
            onClick?.(item.key)
          }
        }}
      >
        {#if item.icon}<span class="anticon" style="font-size:16px;margin-right:{collapsed ? 0 : 10}px;display:inline-flex"><Icon name={item.icon} /></span>{/if}
        {#if !collapsed}<span class="ant-menu-title-content">{item.label}</span>{/if}
      </li>
    {/snippet}
    {#if collapsed}
      <Tooltip title={item.label} position="right" wrapperStyle="display:block">{@render itemContent()}</Tooltip>
    {:else}
      {@render itemContent()}
    {/if}
  {/each}
</ul>

<style>
  .ant-menu {
    box-sizing: border-box;
    margin: 0;
    padding: 4px;
    list-style: none;
    font-size: 14px;
    line-height: 1.5715;
    outline: none;
  }
  .ant-menu-dark {
    background: var(--ant-menu-dark-bg);
    color: var(--ant-menu-dark-item-color);
  }
  .ant-menu-item {
    display: flex;
    align-items: center;
    padding: 0 16px;
    height: 40px;
    line-height: 40px;
    margin: 4px 0;
    border-radius: var(--ant-border-radius);
    cursor: pointer;
    transition: all 0.2s;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ant-menu-collapsed .ant-menu-item {
    padding: 0;
    justify-content: center;
  }
  .ant-menu-collapsed .ant-menu-item .anticon {
    margin-right: 0;
  }
  .ant-menu-dark .ant-menu-item {
    color: var(--ant-menu-dark-item-color);
  }
  .ant-menu-dark .ant-menu-item:hover {
    color: var(--ant-menu-dark-item-hover-color);
  }
  .ant-menu-dark .ant-menu-item-selected {
    background: var(--ant-menu-dark-item-selected-bg);
    color: var(--ant-menu-dark-item-selected-color);
  }
  .ant-menu-item-danger {
    color: var(--ant-color-error) !important;
  }
  .ant-menu-title-content {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ant-menu-light {
    background: transparent;
    color: var(--ant-color-text);
  }
  .ant-menu-light .ant-menu-item {
    color: var(--ant-color-text);
  }
  .ant-menu-light .ant-menu-item:hover {
    background: var(--ant-color-fill-secondary);
    color: var(--ant-color-text);
  }
  .ant-menu-light .ant-menu-item-selected {
    background: var(--ant-menu-light-item-selected-bg);
    color: var(--ant-color-primary);
    font-weight: 600;
  }
</style>
