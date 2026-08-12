<script lang="ts">
  // Modal：弹窗（复刻 antd 5 视觉：遮罩 + 居中 + header/body/footer）
  import type { Snippet } from 'svelte'
  import { t } from '$lib/i18n'
  import Button from './Button.svelte'
  import { Icon } from '$lib/icons'

  let {
    open = false,
    title = '',
    width = 520,
    footer,
    onclose,
    onOk,
    okText = t('common.ok'),
    cancelText = t('common.cancel'),
    confirmLoading = false,
    okDanger = false,
    maskClosable = true,
    children,
    closable = true,
    destroyOnClose = false,
    bodyStyle = '',
  }: {
    open?: boolean
    title?: string
    width?: number | string
    footer?: Snippet
    onclose?: () => void
    onOk?: () => void
    okText?: string
    cancelText?: string
    confirmLoading?: boolean
    okDanger?: boolean
    maskClosable?: boolean
    children?: Snippet
    closable?: boolean
    destroyOnClose?: boolean
    bodyStyle?: string
  } = $props()

  function handleMask(e: MouseEvent) {
    if (maskClosable && e.target === e.currentTarget) onclose?.()
  }

  $effect(() => {
    if (typeof document !== 'undefined') {
      if (open) document.body.style.overflow = 'hidden'
      else document.body.style.overflow = ''
    }
  })
</script>

{#if open || !destroyOnClose}
  <div class="ant-modal-root" class:ant-modal-hidden={!open}>
    <div class="ant-modal-mask"></div>
    <!-- 遮罩点击关闭（maskClosable）：背景区域非交互控件，键盘用户通过 Esc/关闭按钮关闭 -->
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="ant-modal-wrap" onclick={handleMask}>
      <div class="ant-modal" role="dialog" aria-modal="true" style="width:{typeof width === 'number' ? width + 'px' : width}">
        <div class="ant-modal-content">
          {#if closable}
            <button class="ant-modal-close" title={t('common.closeBtn')} aria-label={t('common.closeBtn')} onclick={onclose}>
              <span class="ant-modal-close-x"><Icon name="close" /></span>
            </button>
          {/if}
          {#if title}
            <div class="ant-modal-header">
              <div class="ant-modal-title">{title}</div>
            </div>
          {/if}
          <div class="ant-modal-body" style={bodyStyle}>
            {#if children}{@render children()}{/if}
          </div>
          {#if footer}
            <div class="ant-modal-footer">{@render footer()}</div>
          {:else if onOk}
            <div class="ant-modal-footer">
              <Button tooltip={t('common.closeDialogNoSave')} onClick={onclose}>{cancelText}</Button>
              <Button type="primary" danger={okDanger} loading={confirmLoading} tooltip={t('common.confirmAction')} onClick={onOk}>{okText}</Button>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .ant-modal-root {
    position: fixed;
    inset: 0;
    z-index: 1000;
  }
  .ant-modal-hidden {
    display: none;
  }
  .ant-modal-mask {
    position: absolute;
    inset: 0;
    background: var(--ant-color-bg-mask);
    animation: ant-fade-in 0.2s ease-out;
  }
  @keyframes ant-fade-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
  .ant-modal-wrap {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    overflow: auto;
    padding: 100px 16px 24px;
  }
  .ant-modal {
    position: relative;
    max-width: calc(100vw - 32px);
    margin: 0 auto;
    top: 0;
  }
  .ant-modal-content {
    position: relative;
    background: var(--ant-modal-bg);
    animation: ant-modal-zoom-in 0.2s ease-out;
    background-clip: padding-box;
    border-radius: var(--ant-border-radius-lg);
    box-shadow: var(--ant-box-shadow);
    pointer-events: auto;
    color: var(--ant-color-text);
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 200px);
  }
  .ant-modal-close {
    position: absolute;
    top: 0;
    right: 0;
    z-index: 10;
    width: 54px;
    height: 54px;
    border: none;
    background: transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--ant-color-text-tertiary);
    transition: color 0.2s;
  }
  .ant-modal-close:hover {
    color: var(--ant-color-text);
  }
  .ant-modal-close-x {
    display: flex;
    font-size: 16px;
  }
  .ant-modal-header {
    padding: 16px 24px;
    border-bottom: 1px solid var(--ant-color-border-secondary);
    border-radius: var(--ant-border-radius-lg) var(--ant-border-radius-lg) 0 0;
    background: var(--ant-modal-bg);
  }
  .ant-modal-title {
    font-size: 16px;
    font-weight: 600;
    line-height: 1.5;
    word-wrap: break-word;
    color: var(--ant-color-text);
  }
  .ant-modal-body {
    padding: 24px;
    font-size: 14px;
    line-height: 1.5715;
    word-wrap: break-word;
    overflow-y: auto;
  }
  .ant-modal-footer {
    padding: 10px 16px;
    text-align: right;
    border-top: 1px solid var(--ant-color-border-secondary);
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  @keyframes ant-modal-zoom-in {
    from {
      opacity: 0;
      transform: scale(0.94) translateY(8px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }

  /* 自适应：小屏下弹窗铺满可用宽度并上下居中，内容超高时内部滚动 */
  @media (max-width: 768px) {
    .ant-modal-wrap {
      padding: 24px 16px;
      align-items: center;
    }
    .ant-modal {
      width: 100% !important;
      max-width: calc(100vw - 32px);
    }
    .ant-modal-content {
      max-height: calc(100vh - 48px);
    }
    .ant-modal-header,
    .ant-modal-body {
      padding-left: 16px;
      padding-right: 16px;
    }
  }

</style>
