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
  // List：列表容器（Spin + 空态 + 自定义列表体）；行渲染由 ListItem.svelte 提供
  import type { Snippet } from 'svelte'
  import { t } from '$lib/i18n'
  import Spin from './Spin.svelte'
  import Empty from './Empty.svelte'

  let {
    loading = false,
    emptyText = t('common.noData'),
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
