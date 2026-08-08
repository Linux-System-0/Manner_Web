<script lang="ts">
  // Divider：分割线
  import type { Snippet } from 'svelte'

  let {
    dashed = false,
    orientation = 'center',
    children,
    style = '',
  }: {
    dashed?: boolean
    orientation?: 'left' | 'center' | 'right'
    children?: Snippet
    style?: string
  } = $props()
</script>

<div class="ant-divider ant-divider-horizontal ant-divider-with-text ant-divider-{orientation}" style={style}>
  {#if children}
    <span class="ant-divider-inner-text">{@render children()}</span>
  {/if}
</div>

<style>
  .ant-divider {
    box-sizing: border-box;
    margin: 24px 0;
    padding: 0;
    color: var(--ant-color-text);
    font-size: 14px;
    line-height: 1.5715;
    border-top: 1px solid var(--ant-color-split);
  }
  .ant-divider-dashed {
    border-top-style: dashed;
  }
  .ant-divider-with-text {
    display: flex;
    align-items: center;
    border-top: 0;
  }
  .ant-divider-with-text::before,
  .ant-divider-with-text::after {
    content: '';
    flex: 1;
    border-top: 1px solid var(--ant-color-split);
  }
  .ant-divider-dashed::before,
  .ant-divider-dashed::after {
    border-top-style: dashed;
  }
  .ant-divider-inner-text {
    padding: 0 1em;
    color: var(--ant-color-text-secondary);
    font-size: 14px;
  }
  .ant-divider-left::before {
    flex: 0.05;
  }
  .ant-divider-right::after {
    flex: 0.05;
  }
</style>
