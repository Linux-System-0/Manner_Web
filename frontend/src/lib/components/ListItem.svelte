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
  // ListItem：列表行（供 List children 内使用）
  import type { Snippet } from 'svelte'
  import Avatar from './Avatar.svelte'

  let {
    title = '',
    description = '',
    avatarSrc = null,
    avatarText = '',
    selected = false,
    onclick,
    children,
    extra,
  }: {
    title?: string
    description?: string
    avatarSrc?: string | null
    avatarText?: string
    selected?: boolean
    onclick?: () => void
    children?: Snippet
    extra?: Snippet
  } = $props()
</script>

<!-- 列表行可点击时按按钮语义暴露（li 自身无交互角色，用注释豁免 a11y 静态检查） -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_interactive_supports_focus, a11y_no_noninteractive_tabindex -->
<li
  class="ant-list-item"
  class:ant-list-item-selected={selected}
  style="display:flex;align-items:center;gap:12px;padding:12px 16px;cursor:{onclick ? 'pointer' : 'default'};transition:background 0.2s;border-bottom:1px solid var(--ant-list-item-border)"
  role={onclick ? 'button' : undefined}
  tabindex={onclick ? 0 : undefined}
  onclick={onclick}
  onkeydown={(e) => {
    if (!onclick) return
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      onclick()
    }
  }}
>
  {#if avatarSrc || avatarText}
    <Avatar src={avatarSrc} size={36}>
      {#if avatarText}<span style="font-size:16px">{avatarText}</span>{/if}
    </Avatar>
  {/if}
  <div style="flex:1;min-width:0">
    <div style="color:var(--ant-color-text);font-size:14px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{title}</div>
    {#if description}
      <div style="color:var(--ant-color-text-secondary);font-size:12px;margin-top:2px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{description}</div>
    {/if}
    {#if children}
      <div style="margin-top:4px">{@render children()}</div>
    {/if}
  </div>
  {#if extra}
    <div style="flex-shrink:0">{@render extra()}</div>
  {/if}
</li>

<style>
  .ant-list-item:hover {
    background: var(--ant-color-fill-quaternary);
  }
  .ant-list-item-selected {
    background: var(--chat-selected-bg) !important;
  }
</style>
