<script lang="ts">
  // Switch：开关
  import { t } from '$lib/i18n'

  let {
    checked = false,
    disabled = false,
    onChange,
    loading = false,
  }: {
    checked?: boolean
    disabled?: boolean
    onChange?: (v: boolean) => void
    loading?: boolean
  } = $props()
</script>

<button
  type="button"
  class="ant-switch"
  class:ant-switch-checked={checked}
  class:ant-switch-disabled={disabled}
  role="switch"
  aria-checked={checked}
  aria-label={t('component.switch')}
  disabled={disabled || loading}
  onclick={() => onChange?.(!checked)}
>
  <div class="ant-switch-handle"></div>
  <span class="ant-switch-inner"></span>
</button>

<style>
  .ant-switch {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 22px;
    min-width: 44px;
    background: var(--ant-color-fill-quaternary);
    border: 0;
    border-radius: 100px;
    cursor: pointer;
    transition: all 0.2s;
    padding: 0;
    vertical-align: middle;
  }
  .ant-switch-handle {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 2px 4px rgba(0, 35, 11, 0.2);
    transition: all 0.2s;
  }
  .ant-switch-checked {
    background: var(--ant-color-primary);
  }
  .ant-switch-checked .ant-switch-handle {
    left: 24px;
  }
  .ant-switch-disabled {
    cursor: not-allowed;
    opacity: 0.4;
  }
</style>
