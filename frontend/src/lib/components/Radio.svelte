<script lang="ts">
  // Radio：单选（含 Radio.Group / Radio.Button 风格）
  export interface RadioOption {
    value: string | number
    label: string
    disabled?: boolean
  }

  let {
    options = [] as RadioOption[],
    value = undefined as string | number | undefined,
    onChange,
    disabled = false,
    buttonStyle = false,
    vertical = false,
    style = '',
    class: className = '',
  }: {
    options?: RadioOption[]
    value?: string | number | undefined
    onChange?: (v: string | number) => void
    disabled?: boolean
    buttonStyle?: boolean
    vertical?: boolean
    style?: string
    class?: string
  } = $props()
</script>

<div class="ant-radio-group {buttonStyle ? 'ant-radio-group-button' : ''} {className}" style="display:flex;flex-direction:{vertical ? 'column' : 'row'};gap:{buttonStyle ? 0 : vertical ? '0 8px' : '0 16px'};{style}">
  {#each options as opt (opt.value)}
    <label class="ant-radio-wrapper {buttonStyle ? 'ant-radio-button-wrapper' : ''}" class:ant-radio-wrapper-checked={value === opt.value} class:ant-radio-button-wrapper-checked={buttonStyle && value === opt.value} style="margin:{buttonStyle ? 0 : vertical ? '0 0 8px' : '0 16px 0 0'}">
      {#if !buttonStyle}
        <span class="ant-radio" class:ant-radio-checked={value === opt.value}>
          <input type="radio" class="ant-radio-input" checked={value === opt.value} disabled={disabled || opt.disabled} onclick={() => !disabled && !opt.disabled && onChange?.(opt.value)} />
          <span class="ant-radio-inner"></span>
        </span>
      {/if}
      <span>{opt.label}</span>
      {#if buttonStyle}
        <input type="radio" class="ant-radio-button-input" checked={value === opt.value} disabled={disabled || opt.disabled} onclick={() => !disabled && !opt.disabled && onChange?.(opt.value)} />
      {/if}
    </label>
  {/each}
</div>

<style>
  .ant-radio-wrapper {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: var(--ant-font-size);
    color: var(--ant-color-text);
    cursor: pointer;
    vertical-align: middle;
  }
  .ant-radio {
    position: relative;
    display: inline-block;
    width: 16px;
    height: 16px;
    line-height: 1;
  }
  .ant-radio-input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
    z-index: 1;
    margin: 0;
  }
  .ant-radio-inner {
    position: absolute;
    inset: 0;
    border: 1px solid var(--ant-color-border);
    border-radius: 50%;
    background: var(--ant-color-bg-container);
    transition: all 0.2s;
    box-sizing: border-box;
  }
  .ant-radio-inner::after {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    width: 8px;
    height: 8px;
    margin: -4px 0 0 -4px;
    border-radius: 50%;
    background: var(--ant-color-primary);
    transform: scale(0);
    transition: transform 0.2s;
  }
  .ant-radio-checked .ant-radio-inner {
    border-color: var(--ant-color-primary);
  }
  .ant-radio-checked .ant-radio-inner::after {
    transform: scale(1);
  }
  .ant-radio-group-button {
    border: 1px solid var(--ant-color-border);
    border-radius: var(--ant-border-radius);
    overflow: hidden;
    display: inline-flex;
  }
  .ant-radio-button-wrapper {
    margin: 0 !important;
    padding: 4px 15px;
    border-right: 1px solid var(--ant-color-border);
    background: var(--ant-color-bg-container);
    transition: all 0.2s;
    color: var(--ant-color-text);
  }
  .ant-radio-button-wrapper:last-child {
    border-right: none;
  }
  .ant-radio-button-wrapper:hover {
    color: var(--ant-color-primary);
  }
  .ant-radio-button-wrapper-checked {
    background: var(--ant-color-primary);
    color: #fff;
  }
  .ant-radio-button-wrapper-checked:hover {
    color: #fff;
  }
  .ant-radio-button-input {
    display: none;
  }
</style>
