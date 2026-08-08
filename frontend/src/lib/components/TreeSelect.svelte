<script lang="ts">
  // TreeSelect：树形选择（上级部门选择用；复用 Tree 的层级渲染）
  import { Icon } from '$lib/icons'
  import { onMount } from 'svelte'
  import type { TreeNode } from './Tree.svelte'

  let {
    treeData = [] as TreeNode[],
    value = '',
    placeholder = '请选择',
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
        aria-label="清除"
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
        aria-label={expanded.includes(node.key) ? '折叠' : '展开'}
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
