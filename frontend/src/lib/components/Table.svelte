<script lang="ts">
  // Table：数据表格（复刻 antd 5 视觉；支持列 snippet 渲染与分页）
  import type { Snippet } from 'svelte'
  import { t } from '$lib/i18n'
  import Spin from './Spin.svelte'
  import Empty from './Empty.svelte'
  import Pagination from './Pagination.svelte'

  export interface TableColumn<T> {
    title: string
    dataIndex?: string
    key?: string
    width?: number | string
    align?: 'left' | 'center' | 'right'
    /** 简单渲染：返回纯文本（默认经 Svelte 插值自动转义；如需 HTML 请用 snippet 显式渲染） */
    render?: (row: T) => string
    /** 命名 snippet 渲染：对应调用方传入的 snippets[name] */
    snippet?: string
  }

  export interface TablePagination {
    current: number
    pageSize: number
    total: number
    onChange?: (page: number, pageSize?: number) => void
    showTotal?: (total: number) => string
    simple?: boolean
  }

  let {
    columns = [] as TableColumn<never>[],
    dataSource = [] as never[],
    rowKey,
    loading = false,
    pagination,
    emptyText = t('common.noData'),
    scroll,
    snippets = {} as Record<string, Snippet<[never]>>,
    size = 'default',
  }: {
    columns?: TableColumn<never>[]
    dataSource?: never[]
    rowKey?: string | ((row: never) => string)
    loading?: boolean
    pagination?: TablePagination
    emptyText?: string
    scroll?: { x?: number | string }
    snippets?: Record<string, Snippet<[never]>>
    size?: 'small' | 'middle' | 'large' | 'default'
  } = $props()

  let sizeCls = $derived(size === 'small' ? 'ant-table-small' : size === 'middle' ? 'ant-table-middle' : '')

  function keyOf(row: never, index: number): string {
    if (!rowKey) return String(index)
    if (typeof rowKey === 'function') return String(rowKey(row))
    return String((row as Record<string, unknown>)[rowKey] ?? index)
  }

  function cellValue(row: never, col: TableColumn<never>): string {
    if (col.render) return col.render(row)
    if (col.dataIndex) {
      const v = (row as Record<string, unknown>)[col.dataIndex]
      return v === null || v === undefined ? '' : String(v)
    }
    return ''
  }
</script>

<div class="ant-table-wrapper">
  <Spin spinning={loading}>
    <div
      class="ant-table {sizeCls}"
      style={scroll ? `min-width:${typeof scroll.x === 'number' ? scroll.x + 'px' : scroll.x};overflow:auto` : ''}
    >
      <div class="ant-table-container">
        <table style="width:100%;border-collapse:separate;border-spacing:0">
          <thead class="ant-table-thead">
            <tr>
              {#each columns as col}
                <th
                  class="ant-table-cell"
                  style="text-align:{col.align ?? 'left'};{col.width ? 'width:' + (typeof col.width === 'number' ? col.width + 'px' : col.width) : ''}"
                >
                  {col.title}
                </th>
              {/each}
            </tr>
          </thead>
          <tbody class="ant-table-tbody">
            {#if dataSource.length === 0}
              <tr class="ant-table-placeholder">
                <td class="ant-table-cell" colspan={columns.length} style="text-align:center;padding:32px 0">
                  <Empty description={emptyText} />
                </td>
              </tr>
            {:else}
              {#each dataSource as row, i (keyOf(row, i))}
                <tr class="ant-table-row">
                  {#each columns as col}
                    <td class="ant-table-cell" style="text-align:{col.align ?? 'left'}">
                      {#if col.snippet && snippets[col.snippet]}
                        {@render snippets[col.snippet]!(row)}
                      {:else}
                        {cellValue(row, col)}
                      {/if}
                    </td>
                  {/each}
                </tr>
              {/each}
            {/if}
          </tbody>
        </table>
      </div>
    </div>
    {#if pagination}
      <div style="display:flex;justify-content:flex-end;padding:16px 0 8px">
        <Pagination
          current={pagination.current}
          pageSize={pagination.pageSize}
          total={pagination.total}
          simple={pagination.simple}
          showTotal={pagination.showTotal}
          onChange={pagination.onChange}
        />
      </div>
    {/if}
  </Spin>
</div>

<style>
  .ant-table-wrapper {
    width: 100%;
  }
  .ant-table {
    font-size: var(--ant-font-size);
    line-height: 1.5715;
    color: var(--ant-color-text);
    background: var(--ant-color-bg-container);
    border: 1px solid var(--ant-color-border-secondary);
    border-radius: var(--ant-border-radius-lg);
  }
  .ant-table-container {
    border-radius: var(--ant-border-radius-lg);
  }
  .ant-table-thead > tr > th {
    position: relative;
    padding: 16px;
    text-align: left;
    font-weight: 600;
    background: var(--ant-table-header-bg);
    border-bottom: 1px solid var(--ant-color-border-secondary);
    color: var(--ant-color-text);
    white-space: nowrap;
  }
  .ant-table-tbody > tr > td {
    padding: 16px;
    border-bottom: 1px solid var(--ant-color-border-secondary);
    background: var(--ant-table-row-bg);
    transition: background 0.2s;
  }
  .ant-table-tbody > tr:hover > td {
    background: var(--ant-table-row-hover-bg) !important;
  }
  .ant-table-small .ant-table-thead > tr > th,
  .ant-table-small .ant-table-tbody > tr > td {
    padding: 8px;
  }
  .ant-table-middle .ant-table-thead > tr > th,
  .ant-table-middle .ant-table-tbody > tr > td {
    padding: 12px 8px;
  }
</style>
