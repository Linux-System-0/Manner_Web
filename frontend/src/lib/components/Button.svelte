<script lang="ts">
  // Button：复刻 antd 5 视觉（primary/default/dashed/text/link + danger + loading + block + size）
  import type { Snippet } from 'svelte'

  let {
    type = 'default',
    size = 'middle',
    danger = false,
    block = false,
    loading = false,
    disabled = false,
    htmlType = 'button',
    icon,
    onClick,
    children,
    class: className = '',
    style = '',
    title = '',
  }: {
    type?: 'default' | 'primary' | 'dashed' | 'text' | 'link'
    size?: 'large' | 'middle' | 'small'
    danger?: boolean
    block?: boolean
    loading?: boolean
    disabled?: boolean
    htmlType?: 'button' | 'submit' | 'reset'
    icon?: Snippet
    onClick?: (e: MouseEvent) => void
    children?: Snippet
    class?: string
    style?: string
    title?: string
  } = $props()

  let cls = $derived(
    `ant-btn ant-btn-${type} ant-btn-${size} ${danger ? 'ant-btn-dangerous' : ''} ${loading ? 'ant-btn-loading' : ''} ${className}`.trim(),
  )
</script>

<button
  type={htmlType}
  class={cls}
  class:ant-btn-block={block}
  {disabled}
  {title}
  style={style}
  onclick={onClick}
>
  {#if loading}
    <span class="ant-btn-icon">
      <span class="anticon anticon-spin" style="font-size:14px;line-height:0"><svg viewBox="64 64 896 896" width="1em" height="1em" fill="currentColor" focusable="false"><path d="M512 64a32 32 0 0122.6 54.6 32 32 0 01-45.3 0A32 32 0 01512 64zm0 768a32 32 0 0122.6 54.6 32 32 0 01-45.3 0A32 32 0 01512 832zM172 448a32 32 0 0122.6 54.6 32 32 0 01-45.3 0A32 32 0 01172 448zm680 0a32 32 0 0122.6 54.6 32 32 0 01-45.3 0A32 32 0 01852 448zM272 752a32 32 0 0122.6 54.6 32 32 0 01-45.3 0A32 32 0 01272 752zm480-480a32 32 0 0122.6 54.6 32 32 0 01-45.3 0A32 32 0 01752 272z" /></svg></span>
    </span>
  {:else if icon}
    <span class="ant-btn-icon">{@render icon()}</span>
  {/if}
  {#if children}<span class="ant-btn-text">{@render children()}</span>{/if}
</button>

<style>
  .ant-btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    font-weight: 400;
    white-space: nowrap;
    text-align: center;
    background-image: none;
    border: 1px solid transparent;
    cursor: pointer;
    transition: all 0.2s cubic-bezier(0.645, 0.045, 0.355, 1);
    user-select: none;
    touch-action: manipulation;
    border-radius: var(--ant-border-radius);
    font-size: var(--ant-font-size);
    height: 32px;
    padding: 4px 15px;
    color: var(--ant-color-text);
    background: var(--ant-color-bg-container);
    border-color: var(--ant-color-border);
  }
  .ant-btn:hover,
  .ant-btn:focus {
    color: var(--ant-color-primary-hover);
    border-color: var(--ant-color-primary-hover);
  }
  .ant-btn:active {
    color: var(--ant-color-primary-active);
    border-color: var(--ant-color-primary-active);
  }
  .ant-btn-primary {
    color: #fff;
    background: var(--ant-color-primary);
    border-color: var(--ant-color-primary);
    text-shadow: 0 -1px 0 rgba(0, 0, 0, 0.12);
    box-shadow: 0 2px 0 rgba(5, 145, 255, 0.1);
  }
  .ant-btn-primary:hover,
  .ant-btn-primary:focus {
    color: #fff;
    background: var(--ant-color-primary-hover);
    border-color: var(--ant-color-primary-hover);
  }
  .ant-btn-primary:active {
    color: #fff;
    background: var(--ant-color-primary-active);
    border-color: var(--ant-color-primary-active);
  }
  .ant-btn-dashed {
    border-style: dashed;
  }
  .ant-btn-text,
  .ant-btn-link {
    background: transparent;
    border-color: transparent;
  }
  .ant-btn-text:hover,
  .ant-btn-text:focus {
    background: var(--ant-color-fill-secondary);
    color: var(--ant-color-text);
  }
  .ant-btn-link {
    color: var(--ant-color-primary);
    box-shadow: none;
  }
  .ant-btn-link:hover,
  .ant-btn-link:focus {
    color: var(--ant-color-primary-hover);
    background: transparent;
  }
  .ant-btn-dangerous.ant-btn-primary {
    background: var(--ant-color-error);
    border-color: var(--ant-color-error);
  }
  .ant-btn-dangerous.ant-btn-primary:hover,
  .ant-btn-dangerous.ant-btn-primary:focus {
    background: var(--ant-color-error);
    opacity: 0.88;
    border-color: var(--ant-color-error);
  }
  .ant-btn-dangerous.ant-btn-default,
  .ant-btn-dangerous.ant-btn-dashed {
    color: var(--ant-color-error);
    border-color: var(--ant-color-error);
    background: var(--ant-color-bg-container);
  }
  .ant-btn-dangerous.ant-btn-default:hover,
  .ant-btn-dangerous.ant-btn-dashed:hover {
    background: var(--ant-color-error);
    color: #fff;
  }
  .ant-btn-dangerous.ant-btn-text,
  .ant-btn-dangerous.ant-btn-link {
    color: var(--ant-color-error);
    background: transparent;
  }
  .ant-btn-large {
    height: 40px;
    padding: 6.4px 15px;
    font-size: var(--ant-font-size-lg);
  }
  .ant-btn-small {
    height: 24px;
    padding: 0 7px;
    font-size: var(--ant-font-size-sm);
  }
  .ant-btn-block {
    width: 100%;
  }
  .ant-btn[disabled] {
    cursor: not-allowed;
    color: var(--ant-color-text-disabled);
    background: var(--ant-color-fill-tertiary);
    border-color: var(--ant-color-border-secondary);
    box-shadow: none;
    text-shadow: none;
  }
  .ant-btn-primary[disabled] {
    background: var(--ant-color-fill-tertiary);
    border-color: var(--ant-color-border-secondary);
    color: var(--ant-color-text-disabled);
  }
  .ant-btn-icon {
    display: inline-flex;
    align-items: center;
    line-height: 0;
  }
  .ant-btn-loading {
    cursor: default;
    pointer-events: none;
  }
</style>
