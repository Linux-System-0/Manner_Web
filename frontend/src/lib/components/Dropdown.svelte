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
  // Dropdown：下拉菜单（点击触发，外部点击关闭）
  import type { Snippet } from 'svelte'
  import { onMount } from 'svelte'
  import { Icon } from '$lib/icons'

  export interface DropdownMenuItem {
    key: string
    label: string
    icon?: string
    danger?: boolean
    divider?: boolean
  }

  let {
    items = [] as DropdownMenuItem[],
    onClick,
    children,
    placement = 'bottomRight',
    disabled = false,
  }: {
    items?: DropdownMenuItem[]
    onClick?: (key: string) => void
    children?: Snippet
    placement?: string
    disabled?: boolean
  } = $props()

  let open = $state(false)
  let rootEl: HTMLSpanElement | undefined = $state()
  let panelEl: HTMLDivElement | undefined = $state()

  function toggle() {
    if (!disabled) open = !open
  }

  function pick(key: string) {
    open = false
    onClick?.(key)
  }

  function onDocClick(e: MouseEvent) {
    if (rootEl && !rootEl.contains(e.target as Node)) open = false
  }

  onMount(() => {
    document.addEventListener('click', onDocClick)
    return () => document.removeEventListener('click', onDocClick)
  })

  $effect(() => {
    if (open && panelEl && rootEl) {
      const r = rootEl.getBoundingClientRect()
      panelEl.style.top = r.bottom + 4 + 'px'
      panelEl.style.right = placement.includes('Right') ? window.innerWidth - r.right + 'px' : 'auto'
      panelEl.style.left = placement.includes('Right') ? 'auto' : r.left + 'px'
    }
  })
</script>

<span class="ant-dropdown-trigger" bind:this={rootEl} style="display:inline-flex">
  <span
    role="button"
    tabindex={disabled ? -1 : 0}
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={toggle}
    onkeydown={(e) => {
      if (disabled) return
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault()
        toggle()
      }
    }}
  >{#if children}{@render children()}{/if}</span>
</span>

{#if open}
  <div class="ant-dropdown" bind:this={panelEl} style="position:fixed;z-index:1050;min-width:160px">
    <ul class="ant-dropdown-menu" role="menu">
      {#each items as item (item.key)}
        {#if item.divider}
          <li class="ant-dropdown-menu-item-divider" role="separator"></li>
        {:else}
          <li
            class="ant-dropdown-menu-item"
            class:ant-dropdown-menu-item-danger={item.danger}
            role="menuitem"
            tabindex={-1}
            onclick={() => pick(item.key)}
            onkeydown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault()
                pick(item.key)
              }
            }}
          >
            {#if item.icon}<span class="anticon" style="margin-right:8px;display:inline-flex"><Icon name={item.icon} style="font-size:14px" /></span>{/if}
            {item.label}
          </li>
        {/if}
      {/each}
    </ul>
  </div>
{/if}

<style>
  .ant-dropdown-menu {
    animation: ant-dropdown-in 0.15s ease-out;
    margin: 0;
    padding: 4px;
    list-style: none;
    background: var(--ant-dropdown-bg);
    border-radius: var(--ant-border-radius-lg);
    box-shadow: var(--ant-box-shadow);
    font-size: 14px;
    /* 显式声明行高：line-height 可继承，否则会沿用触发容器
       （如 header 的 line-height:64px）导致每行被撑得过大 */
    line-height: 22px;
    color: var(--ant-color-text);
  }
  .ant-dropdown-menu-item {
    display: flex;
    align-items: center;
    padding: 5px 12px;
    line-height: 22px;
    min-height: 32px;
    border-radius: var(--ant-border-radius-sm);
    cursor: pointer;
    transition: background 0.2s;
    white-space: nowrap;
  }
  .ant-dropdown-menu-item:hover {
    background: var(--ant-dropdown-item-hover-bg);
  }
  .ant-dropdown-menu-item-danger {
    color: var(--ant-color-error);
  }
  .ant-dropdown-menu-item-divider {
    height: 1px;
    margin: 4px 0;
    background: var(--ant-color-split);
  }

  @keyframes ant-dropdown-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

</style>
