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
  // ConfirmModal：modal.confirm 的渲染宿主（由 modal.ts mount 单例挂载）
  // 状态通过 modal.ts 的 confirmState store 驱动（$confirmState 自动订阅）
  import { t } from '$lib/i18n'
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
    okText={$confirmState.okText ?? t('common.ok')}
    cancelText={$confirmState.cancelText ?? t('common.cancel')}
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
