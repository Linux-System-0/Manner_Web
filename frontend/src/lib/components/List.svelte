<script lang="ts">
  // List：列表容器（Spin + 空态 + 自定义列表体）；行渲染由 ListItem.svelte 提供
  import type { Snippet } from 'svelte'
  import Spin from './Spin.svelte'
  import Empty from './Empty.svelte'

  let {
    loading = false,
    emptyText = '暂无数据',
    hasData = true,
    children,
    class: className = '',
  }: {
    loading?: boolean
    emptyText?: string
    hasData?: boolean
    children?: Snippet
    class?: string
  } = $props()
</script>

<Spin spinning={loading}>
  <div class="ant-list {className}">
    {#if !hasData}
      <Empty description={emptyText} />
    {:else if children}
      <ul class="ant-list-items" style="list-style:none;margin:0;padding:0">
        {@render children()}
      </ul>
    {/if}
  </div>
</Spin>

<style>
  .ant-list {
    font-size: var(--ant-font-size);
    color: var(--ant-color-text);
  }
</style>
