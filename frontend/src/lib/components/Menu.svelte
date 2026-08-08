<script lang="ts">
  // Menu：导航菜单（dark inline 风格，侧边栏用）
  import { Icon } from '$lib/icons'

  export interface MenuItem {
    key: string
    label: string
    icon?: string
    danger?: boolean
  }

  let {
    items = [] as MenuItem[],
    selectedKeys = [] as string[],
    theme = 'dark',
    onClick,
    style = '',
  }: {
    items?: MenuItem[]
    selectedKeys?: string[]
    theme?: 'dark' | 'light'
    onClick?: (key: string) => void
    style?: string
  } = $props()
</script>

<ul class="ant-menu ant-menu-root ant-menu-inline ant-menu-{theme}" style="{style}">
  {#each items as item (item.key)}
    <li
      class="ant-menu-item"
      class:ant-menu-item-selected={selectedKeys.includes(item.key)}
      class:ant-menu-item-danger={item.danger}
      role="menuitem"
      title={item.label}
      tabindex={selectedKeys.includes(item.key) ? 0 : -1}
      onclick={() => onClick?.(item.key)}
      onkeydown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault()
          onClick?.(item.key)
        }
      }}
    >
      {#if item.icon}<span class="anticon" style="font-size:16px;margin-right:10px;display:inline-flex"><Icon name={item.icon} /></span>{/if}
      <span class="ant-menu-title-content">{item.label}</span>
    </li>
  {/each}
</ul>

<style>
  .ant-menu {
    box-sizing: border-box;
    margin: 0;
    padding: 4px;
    list-style: none;
    font-size: 14px;
    line-height: 1.5715;
    outline: none;
  }
  .ant-menu-dark {
    background: var(--ant-menu-dark-bg);
    color: var(--ant-menu-dark-item-color);
  }
  .ant-menu-item {
    display: flex;
    align-items: center;
    padding: 0 16px;
    height: 40px;
    line-height: 40px;
    margin: 4px 0;
    border-radius: var(--ant-border-radius);
    cursor: pointer;
    transition: all 0.2s;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ant-menu-dark .ant-menu-item {
    color: var(--ant-menu-dark-item-color);
  }
  .ant-menu-dark .ant-menu-item:hover {
    color: var(--ant-menu-dark-item-hover-color);
  }
  .ant-menu-dark .ant-menu-item-selected {
    background: var(--ant-menu-dark-item-selected-bg);
    color: var(--ant-menu-dark-item-selected-color);
  }
  .ant-menu-item-danger {
    color: var(--ant-color-error) !important;
  }
  .ant-menu-title-content {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ant-menu-light .ant-menu-item:hover {
    background: var(--ant-color-fill-secondary);
  }
</style>
