<script lang="ts">
  // Checkbox：复选（含 indeterminate 半选）
  import type { Snippet } from 'svelte'

  let {
    checked = false,
    indeterminate = false,
    label = '',
    disabled = false,
    onChange,
    children,
    style = '',
  }: {
    checked?: boolean
    indeterminate?: boolean
    label?: string
    disabled?: boolean
    onChange?: (v: boolean) => void
    children?: Snippet
    style?: string
  } = $props()
</script>

<label class="ant-checkbox-wrapper" class:ant-checkbox-wrapper-disabled={disabled} style={style}>
  <span class="ant-checkbox" class:ant-checkbox-checked={checked} class:ant-checkbox-indeterminate={indeterminate} class:ant-checkbox-disabled={disabled}>
    <input type="checkbox" class="ant-checkbox-input" {checked} {disabled} onchange={(e) => onChange?.((e.target as HTMLInputElement).checked)} />
    <span class="ant-checkbox-inner"></span>
  </span>
  {#if label}<span style="padding-left:8px">{label}</span>{/if}
  {#if children}<span style="padding-left:8px">{@render children()}</span>{/if}
</label>

<style>
  .ant-checkbox-wrapper {
    display: inline-flex;
    align-items: center;
    font-size: var(--ant-font-size);
    color: var(--ant-color-text);
    cursor: pointer;
    vertical-align: middle;
  }
  .ant-checkbox {
    position: relative;
    display: inline-block;
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }
  .ant-checkbox-input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
    z-index: 1;
    margin: 0;
  }
  .ant-checkbox-inner {
    position: absolute;
    inset: 0;
    border: 1px solid var(--ant-color-border);
    border-radius: var(--ant-border-radius-sm);
    background: var(--ant-color-bg-container);
    transition: all 0.2s;
    box-sizing: border-box;
  }
  .ant-checkbox-inner::after {
    content: '';
    position: absolute;
    left: 4px;
    top: 1px;
    width: 5px;
    height: 9px;
    border: 2px solid #fff;
    border-top: 0;
    border-left: 0;
    transform: rotate(45deg) scale(0);
    transition: transform 0.2s;
  }
  .ant-checkbox-checked .ant-checkbox-inner {
    background: var(--ant-color-primary);
    border-color: var(--ant-color-primary);
  }
  .ant-checkbox-checked .ant-checkbox-inner::after {
    transform: rotate(45deg) scale(1);
  }
  .ant-checkbox-indeterminate .ant-checkbox-inner {
    background: var(--ant-color-primary);
    border-color: var(--ant-color-primary);
  }
  .ant-checkbox-indeterminate .ant-checkbox-inner::after {
    content: '';
    width: 8px;
    height: 2px;
    left: 3px;
    top: 6px;
    transform: none;
    border: none;
    background: #fff;
    opacity: 1;
  }
  .ant-checkbox-disabled .ant-checkbox-inner {
    background: var(--ant-color-fill-tertiary);
    border-color: var(--ant-color-border-secondary);
  }
  .ant-checkbox-wrapper-disabled {
    cursor: not-allowed;
    color: var(--ant-color-text-disabled);
  }
</style>
