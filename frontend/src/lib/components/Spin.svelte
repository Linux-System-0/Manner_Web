<script lang="ts">
  // Spin：加载态（结构兼容根布局 +layout.svelte 的 .ant-spin 用法）
  import type { Snippet } from 'svelte'

  let {
    spinning = true,
    size = 'default',
    tip = '',
    children,
  }: {
    spinning?: boolean
    size?: 'small' | 'default' | 'large'
    tip?: string
    children?: Snippet
  } = $props()

  let sizeCls = $derived(
    size === 'large' ? 'ant-spin-lg' : size === 'small' ? 'ant-spin-sm' : '',
  )
</script>

{#if spinning}
  <div class="ant-spin-nested-loading">
    <div class="ant-spin-container ant-spin-blur">
      {#if children}{@render children()}{/if}
    </div>
    <div class="ant-spin ant-spin-spinning {sizeCls}" aria-busy="true">
      <span class="ant-spin-dot ant-spin-dot-spin">
        <i class="ant-spin-dot-item"></i>
        <i class="ant-spin-dot-item"></i>
        <i class="ant-spin-dot-item"></i>
        <i class="ant-spin-dot-item"></i>
      </span>
      {#if tip}<div class="ant-spin-text">{tip}</div>{/if}
    </div>
  </div>
{:else}
  {#if children}{@render children()}{/if}
{/if}

<style>
  .ant-spin-nested-loading {
    position: relative;
  }
  .ant-spin-container {
    position: relative;
    transition: opacity 0.3s;
  }
  .ant-spin-container.ant-spin-blur {
    opacity: 0.4;
    pointer-events: none;
  }
  .ant-spin {
    box-sizing: border-box;
    color: var(--ant-color-primary);
    font-size: 14px;
    line-height: 1.5715;
    list-style: none;
    vertical-align: middle;
    display: none;
  }
  .ant-spin-spinning {
    display: inline-block;
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 4;
  }
  .ant-spin-dot {
    position: relative;
    display: inline-block;
    width: 32px;
    height: 32px;
  }
  .ant-spin-sm .ant-spin-dot {
    width: 16px;
    height: 16px;
  }
  .ant-spin-lg .ant-spin-dot {
    width: 48px;
    height: 48px;
  }
  .ant-spin-dot-spin {
    transform: rotate(45deg);
    animation: antRotate 1.2s infinite linear;
  }
  .ant-spin-dot-item {
    position: absolute;
    display: block;
    width: 9px;
    height: 9px;
    background: var(--ant-color-primary);
    border-radius: 100%;
    transform: scale(0.75);
    transform-origin: 50% 50%;
    opacity: 0.3;
    animation: antSpinMove 1s infinite linear alternate;
  }
  .ant-spin-sm .ant-spin-dot-item {
    width: 4px;
    height: 4px;
  }
  .ant-spin-lg .ant-spin-dot-item {
    width: 14px;
    height: 14px;
  }
  .ant-spin-dot-item:nth-child(1) {
    top: 0;
    left: 0;
  }
  .ant-spin-dot-item:nth-child(2) {
    top: 0;
    right: 0;
    animation-delay: 0.4s;
  }
  .ant-spin-dot-item:nth-child(3) {
    right: 0;
    bottom: 0;
    animation-delay: 0.8s;
  }
  .ant-spin-dot-item:nth-child(4) {
    bottom: 0;
    left: 0;
    animation-delay: 1.2s;
  }
  .ant-spin-text {
    padding-top: 5px;
    text-shadow: 0 1px 2px var(--ant-color-bg-container);
  }
  :global {
    @keyframes antSpinMove {
      to {
        opacity: 1;
        transform: scale(1);
      }
    }
    @keyframes antRotate {
      to {
        transform: rotate(405deg);
      }
    }
  }
</style>
