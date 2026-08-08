<script lang="ts">
  // ConfirmModal：modal.confirm 的渲染宿主（由 modal.ts mount 单例挂载）
  // 状态通过 modal.ts 的 confirmState store 驱动（$confirmState 自动订阅）
  import Modal from './Modal.svelte'
  import Button from './Button.svelte'
  import { Icon } from '$lib/icons'
  import { confirmState, settleConfirm } from './modal'

  let submitting = $state(false)

  async function ok() {
    const s = $confirmState
    if (!s) return
    const r = s.onOk?.()
    if (r && typeof (r as Promise<void>).then === 'function') {
      submitting = true
      try {
        await (r as Promise<void>)
        settleConfirm(true)
      } catch {
        submitting = false
      }
    } else {
      settleConfirm(true)
    }
  }
</script>

{#if $confirmState}
  <Modal
    open={true}
    title={$confirmState.title}
    onclose={() => settleConfirm(false)}
    onOk={ok}
    okText={$confirmState.okText ?? '确定'}
    cancelText={$confirmState.cancelText ?? '取消'}
    okDanger={$confirmState.okDanger}
    confirmLoading={submitting}
  >
    <div class="ant-modal-confirm">
      <div class="ant-modal-confirm-body">
        <span
          class="ant-modal-confirm-title"
          style="display:flex;align-items:flex-start;gap:8px;font-size:16px;font-weight:600;color:var(--ant-color-text)"
        >
          <span style="color:{$confirmState.okDanger ? 'var(--ant-color-error)' : 'var(--ant-color-warning)'};line-height:1">
            <Icon name={$confirmState.okDanger ? 'exclamation-circle' : 'question-circle'} style="font-size:22px" />
          </span>
          {$confirmState.title}
        </span>
        {#if $confirmState.content}
          <div
            class="ant-modal-confirm-content"
            style="margin:8px 0 0 30px;font-size:14px;color:var(--ant-color-text-secondary);line-height:1.5715"
          >
            {$confirmState.content}
          </div>
        {/if}
      </div>
    </div>
  </Modal>
{/if}
