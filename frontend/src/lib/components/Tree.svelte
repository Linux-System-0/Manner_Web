<script lang="ts">
  // Tree：树形（部门架构用；支持 action snippet 与选中）
  import type { Snippet } from 'svelte'
  import { t } from '$lib/i18n'
  import { Icon } from '$lib/icons'

  export interface TreeNode {
    key: string
    title: string
    children?: TreeNode[]
  }

  let {
    treeData = [] as TreeNode[],
    selectedKeys = [] as string[],
    onSelect,
    defaultExpandAll = false,
    action,
    expandedKeys,
    onExpand,
  }: {
    treeData?: TreeNode[]
    selectedKeys?: string[]
    onSelect?: (key: string) => void
    defaultExpandAll?: boolean
    action?: Snippet<[TreeNode]>
    expandedKeys?: string[]
    onExpand?: (keys: string[]) => void
  } = $props()

  // 初始化时读取 defaultExpandAll/treeData 一次（有意为之），后续变化由下方 $effect 补全
  // svelte-ignore state_referenced_locally
  let internalExpanded = $state<string[]>(defaultExpandAll ? collectAll(treeData) : [])
  let expanded = $derived(expandedKeys ?? internalExpanded)
  let treeInitialized = $state(false)

  $effect(() => {
    // treeData 可能是异步加载：首次拿到数据时若 defaultExpandAll，补上默认展开
    if (!treeInitialized && treeData.length > 0) {
      treeInitialized = true
      if (defaultExpandAll) internalExpanded = collectAll(treeData)
    }
  })

  function collectAll(nodes: TreeNode[]): string[] {
    const keys: string[] = []
    for (const n of nodes) {
      keys.push(n.key)
      if (n.children) keys.push(...collectAll(n.children))
    }
    return keys
  }

  function toggle(key: string) {
    const next = expanded.includes(key) ? expanded.filter((k) => k !== key) : [...expanded, key]
    if (onExpand) onExpand(next)
    else internalExpanded = next
  }
</script>

<div class="ant-tree">
  {#each treeData as node}
    {@render TreeNodeView(node, 0)}
  {/each}
</div>

{#snippet TreeNodeView(node: TreeNode, depth: number)}
  <div class="ant-tree-treenode" style="padding-left:{depth * 20}px">
    <div
      class="ant-tree-node-content-wrapper"
      class:ant-tree-node-selected={selectedKeys.includes(node.key)}
      style="display:flex;align-items:center;gap:4px;padding:2px 8px;border-radius:var(--ant-border-radius-sm);cursor:pointer;min-height:28px"
      role="button"
      tabindex={0}
      onclick={() => onSelect?.(node.key)}
      onkeydown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault()
          onSelect?.(node.key)
        }
      }}
    >
      <span
        class="ant-tree-switcher"
        role="button"
        tabindex={-1}
        aria-label={expanded.includes(node.key) ? t('common.collapse') : t('common.expand')}
        style="width:20px;display:inline-flex;justify-content:center;cursor:pointer"
        onclick={(e) => { e.stopPropagation(); if (node.children?.length) toggle(node.key) }}
        onkeydown={(e) => {
          e.stopPropagation()
          if ((e.key === 'Enter' || e.key === ' ') && node.children?.length) {
            e.preventDefault()
            toggle(node.key)
          }
        }}
      >
        {#if node.children?.length}
          <span style="display:inline-flex;transition:transform 0.2s;transform:{expanded.includes(node.key) ? 'rotate(90deg)' : 'none'}"><Icon name="right" style="font-size:10px" /></span>
        {/if}
      </span>
      <span class="ant-tree-title" style="font-size:14px;color:var(--ant-color-text)">{node.title}</span>
      {#if action}
        <!-- 操作区仅阻止冒泡（避免误触选中），交互在 action snippet 内部 -->
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <span class="ant-tree-actions" style="margin-left:auto" onclick={(e) => e.stopPropagation()}>{@render action(node)}</span>
      {/if}
    </div>
    {#if node.children?.length && expanded.includes(node.key)}
      <div class="ant-tree-child-tree">
        {#each node.children as child}
          {@render TreeNodeView(child, depth + 1)}
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

<style>
  .ant-tree-node-selected {
    background: rgba(22, 119, 255, 0.1);
  }
  .ant-tree-node-content-wrapper:hover {
    background: var(--ant-color-fill-tertiary);
  }
</style>
