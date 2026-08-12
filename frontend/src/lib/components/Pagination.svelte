<script lang="ts">
  // Pagination：分页（antd 视觉：页码方块、当前页主色）
  import { t } from '$lib/i18n'
  import { Icon } from '$lib/icons'

  let {
    current = 1,
    pageSize = 10,
    total = 0,
    simple = false,
    showTotal,
    onChange,
  }: {
    current?: number
    pageSize?: number
    total?: number
    simple?: boolean
    showTotal?: (total: number) => string
    onChange?: (page: number, pageSize?: number) => void
  } = $props()

  let pageCount = $derived(Math.max(1, Math.ceil(total / Math.max(1, pageSize))))
  let pages = $derived.by(() => {
    const count = pageCount
    const cur = Math.min(current, count)
    const arr: number[] = []
    const push = (n: number) => {
      if (!arr.includes(n) && n >= 1 && n <= count) arr.push(n)
    }
    if (count <= 7) {
      for (let i = 1; i <= count; i++) push(i)
    } else {
      push(1)
      if (cur > 3) push(-1)
      for (let i = Math.max(2, cur - 1); i <= Math.min(count - 1, cur + 1); i++) push(i)
      if (cur < count - 2) push(-2)
      push(count)
    }
    return arr
  })

  function go(p: number) {
    if (p >= 1 && p <= pageCount && p !== current) onChange?.(p, pageSize)
  }
</script>

<ul class="ant-pagination">
  {#if showTotal}
    <li class="ant-pagination-total-text" style="margin-right:8px;color:var(--ant-color-text-secondary)">
      {showTotal(total)}
    </li>
  {/if}
  <li
    class="ant-pagination-prev"
    class:ant-pagination-disabled={current <= 1}
    title={t('common.prevPage')}
  >
    <button class="ant-pagination-item-link" type="button" aria-label={t('common.prevPage')} tabindex={current <= 1 ? -1 : 0} onclick={() => go(current - 1)}>
      <Icon name="left" style="font-size:12px" />
    </button>
  </li>
  {#each pages as p (p)}
    {#if p < 0}
      <li class="ant-pagination-jump-prev ant-pagination-jump-next">
        <span class="ant-pagination-item-ellipsis">•••</span>
      </li>
    {:else}
      <li
        class="ant-pagination-item"
        class:ant-pagination-item-active={p === current}
        title={String(p)}
      >
        <a
          rel="nofollow"
          href={`javascript:void(0)`}
          aria-label={t('common.page', { page: p })}
          onclick={(e) => { e.preventDefault(); go(p) }}
        >{p}</a>
      </li>
    {/if}
  {/each}
  <li
    class="ant-pagination-next"
    class:ant-pagination-disabled={current >= pageCount}
    title={t('common.nextPage')}
  >
    <button class="ant-pagination-item-link" type="button" aria-label={t('common.nextPage')} tabindex={current >= pageCount ? -1 : 0} onclick={() => go(current + 1)}>
      <Icon name="right" style="font-size:12px" />
    </button>
  </li>
</ul>

<style>
  .ant-pagination {
    display: flex;
    align-items: center;
    list-style: none;
    margin: 0;
    padding: 0;
    gap: 8px;
    font-size: var(--ant-font-size);
  }
  .ant-pagination-item,
  .ant-pagination-prev,
  .ant-pagination-next {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 32px;
    height: 32px;
    border-radius: var(--ant-border-radius);
    border: 1px solid var(--ant-color-border-secondary);
    background: var(--ant-pagination-item-bg);
    cursor: pointer;
    transition: all 0.2s;
    color: var(--ant-color-text);
  }
  .ant-pagination-item a {
    color: inherit;
    text-decoration: none;
    display: block;
    padding: 0 6px;
    line-height: 30px;
  }
  .ant-pagination-item:hover {
    border-color: var(--ant-color-primary);
    color: var(--ant-color-primary);
  }
  .ant-pagination-item-active {
    border-color: var(--ant-color-primary);
    background: var(--ant-color-primary);
    color: #fff;
  }
  .ant-pagination-item-active a {
    color: #fff;
  }
  .ant-pagination-item-active:hover {
    background: var(--ant-color-primary-hover);
    border-color: var(--ant-color-primary-hover);
  }
  .ant-pagination-prev .ant-pagination-item-link,
  .ant-pagination-next .ant-pagination-item-link {
    border: none;
    background: transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--ant-color-text);
    padding: 0;
    width: 100%;
    height: 100%;
  }
  .ant-pagination-prev:hover,
  .ant-pagination-next:hover {
    border-color: var(--ant-color-primary);
  }
  .ant-pagination-disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
  .ant-pagination-disabled:hover {
    border-color: var(--ant-color-border-secondary);
  }
  .ant-pagination-jump-prev,
  .ant-pagination-jump-next {
    color: var(--ant-color-text-tertiary);
    padding: 0 4px;
    user-select: none;
  }
</style>
