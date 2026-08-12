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
  // TreeSelect：树形选择（上级部门选择用；复用 Tree 的层级渲染）
  import { Icon } from '$lib/icons'
  import { t } from '$lib/i18n'
  import { onMount } from 'svelte'
  import type { TreeNode } from './Tree.svelte'

  let {
    treeData = [] as TreeNode[],
    value = '',
    placeholder = t('common.selectPlaceholder'),
    onChange,
    disabled = false,
    allowClear = false,
  }: {
    treeData?: TreeNode[]
    value?: string
    placeholder?: string
    onChange?: (v: string) => void
    disabled?: boolean
    allowClear?: boolean
  } = $props()

  let open = $state(false)
  let rootEl: HTMLSpanElement | undefined = $state()
  let panelEl: HTMLDivElement | undefined = $state()
  // 初始化时读取 treeData 一次（有意为之），后续变化由下方 $effect 补全
  // svelte-ignore state_referenced_locally
  let expanded = $state<string[]>(collectAll(treeData))
  let treeInitialized = $state(false)

  $effect(() => {
    // treeData 可能是异步加载：首次拿到数据后展开全部节点
    if (!treeInitialized && treeData.length > 0) {
      treeInitialized = true
      expanded = collectAll(treeData)
    }
  })

  let label = $derived(findLabel(treeData, value))

  function collectAll(nodes: TreeNode[]): string[] {
    const keys: string[] = []
    for (const n of nodes) {
      keys.push(n.key)
      if (n.children) keys.push(...collectAll(n.children))
    }
    return keys
  }

  function findLabel(nodes: TreeNode[], key: string): string {
    for (const n of nodes) {
      if (n.key === key) return n.title
      if (n.children) {
        const r = findLabel(n.children, key)
        if (r) return r
      }
    }
    return ''
  }

  function pick(key: string) {
    onChange?.(key)
    open = false
  }

  function toggle(key: string) {
    expanded = expanded.includes(key) ? expanded.filter((k) => k !== key) : [...expanded, key]
  }

  function clear(e: MouseEvent | KeyboardEvent) {
    e.stopPropagation()
    onChange?.('')
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
      panelEl.style.left = r.left + 'px'
      panelEl.style.minWidth = r.width + 'px'
    }
  })
</script>

<span class="ant-select" bind:this={rootEl} style="display:inline-block;width:100%;position:relative">
  <div
    class="ant-select-selector"
    style="display:flex;align-items:center;height:32px;padding:0 11px;border:1px solid var(--ant-color-border);border-radius:var(--ant-border-radius);background:var(--ant-color-bg-container);cursor:pointer"
    role="combobox"
    aria-haspopup="tree"
    aria-expanded={open}
    aria-controls="ant-treeselect-listbox"
    tabindex={disabled ? -1 : 0}
    onclick={() => !disabled && (open = !open)}
    onkeydown={(e) => {
      if (disabled) return
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault()
        open = !open
      }
      if (e.key === 'Escape') open = false
    }}
  >
    <span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:{label ? 'var(--ant-color-text)' : 'var(--ant-color-text-quaternary)'}">{label || placeholder}</span>
    {#if allowClear && value}
      <span
        role="button"
        tabindex={-1}
        aria-label={t('common.clear')}
        onclick={clear}
        onkeydown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault()
            clear(e)
          }
        }}
        style="display:inline-flex;color:var(--ant-color-text-quaternary);cursor:pointer"
      ><Icon name="close" style="font-size:12px" /></span>
    {:else}
      <span style="display:inline-flex;color:var(--ant-color-text-quaternary)"><Icon name="down" style="font-size:12px" /></span>
    {/if}
  </div>
</span>

{#if open}
  <div class="ant-select-dropdown" id="ant-treeselect-listbox" role="tree" bind:this={panelEl} style="position:fixed;z-index:1050;background:var(--ant-color-bg-elevated);border-radius:var(--ant-border-radius-lg);box-shadow:var(--ant-box-shadow);padding:4px;max-height:256px;overflow-y:auto">
    {#each treeData as node}
      {@render NodeView(node, 0)}
    {/each}
  </div>
{/if}

{#snippet NodeView(node: TreeNode, depth: number)}
  <div style="padding-left:{depth * 16}px">
    <div
      style="display:flex;align-items:center;gap:4px;padding:5px 8px;border-radius:var(--ant-border-radius-sm);cursor:pointer;color:var(--ant-color-text)"
      class:ant-select-item-option-selected={value === node.key}
      role="button"
      tabindex={0}
      onclick={() => pick(node.key)}
      onkeydown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault()
          pick(node.key)
        }
      }}
    >
      <span
        role="button"
        tabindex={-1}
        aria-label={expanded.includes(node.key) ? t('common.collapse') : t('common.expand')}
        style="width:18px;display:inline-flex;justify-content:center"
        onclick={(e) => { e.stopPropagation(); toggle(node.key) }}
        onkeydown={(e) => {
          e.stopPropagation()
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault()
            toggle(node.key)
          }
        }}
      >
        {#if node.children?.length}<span style="display:inline-flex;transform:{expanded.includes(node.key) ? 'rotate(90deg)' : 'none'}"><Icon name="right" style="font-size:10px" /></span>{/if}
      </span>
      <span style="flex:1">{node.title}</span>
      {#if value === node.key}<span style="display:inline-flex;color:var(--ant-color-primary)"><Icon name="check" style="font-size:12px" /></span>{/if}
    </div>
    {#if node.children?.length && expanded.includes(node.key)}
      {#each node.children as child}{@render NodeView(child, depth + 1)}{/each}
    {/if}
  </div>
{/snippet}

<style>
  .ant-select-item-option-selected {
    background: rgba(22, 119, 255, 0.08);
    color: var(--ant-color-primary);
    font-weight: 600;
  }
  .ant-select-dropdown div:hover {
    background: var(--ant-color-fill-secondary);
  }
</style>
