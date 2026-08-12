<script lang="ts">
  // Input：普通 / Password / TextArea（复刻 antd 5 视觉）
  import { Icon } from '$lib/icons'
  import { t } from '$lib/i18n'

  let {
    type = 'text',
    value = '',
    placeholder = '',
    disabled = false,
    size = 'middle',
    prefix,
    onChange,
    onInput,
    onEnter,
    class: className = '',
    style = '',
    rows = 4,
    maxlength,
  }: {
    type?: 'text' | 'password' | 'textarea'
    value?: string
    placeholder?: string
    disabled?: boolean
    size?: 'large' | 'middle' | 'small'
    prefix?: string
    onChange?: (v: string) => void
    onInput?: (v: string) => void
    onEnter?: () => void
    class?: string
    style?: string
    rows?: number
    maxlength?: number
  } = $props()

  let showPassword = $state(false)
  let sizeCls = $derived(size === 'large' ? 'ant-input-lg' : size === 'small' ? 'ant-input-sm' : '')

  function handleInput(e: Event) {
    const v = (e.target as HTMLInputElement | HTMLTextAreaElement).value
    onInput?.(v)
    onChange?.(v)
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === 'Enter') onEnter?.()
  }
</script>

{#if type === 'textarea'}
  <span class="ant-input-affix-wrapper ant-input-textarea-wrapper {sizeCls} {className}" style={style}>
    <textarea
      class="ant-input"
      {value}
      {placeholder}
      {disabled}
      {rows}
      {maxlength}
      oninput={handleInput}
    ></textarea>
  </span>
{:else}
  <span class="ant-input-affix-wrapper {sizeCls} {prefix ? 'ant-input-affix-wrapper-has-prefix' : ''} {className}" style={style}>
    {#if prefix}<span class="ant-input-prefix"><Icon name={prefix} /></span>{/if}
    <input
      class="ant-input"
      type={type === 'password' && showPassword ? 'text' : type}
      {value}
      {placeholder}
      {disabled}
      {maxlength}
      oninput={handleInput}
      onkeydown={handleKey}
    />
    {#if type === 'password'}
      <span
        class="ant-input-suffix"
        role="button"
        tabindex={0}
        aria-label={t('component.togglePassword')}
        onclick={() => (showPassword = !showPassword)}
        onkeydown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault()
            showPassword = !showPassword
          }
        }}
      >
        <Icon name={showPassword ? 'eye' : 'eye-invisible'} />
      </span>
    {/if}
  </span>
{/if}

<style>
  .ant-input-affix-wrapper {
    position: relative;
    display: inline-flex;
    align-items: center;
    width: 100%;
    padding: 4px 11px;
    font-size: var(--ant-font-size);
    line-height: 1.5715;
    color: var(--ant-color-text);
    background: var(--ant-color-bg-container);
    border: 1px solid var(--ant-color-border);
    border-radius: var(--ant-border-radius);
    transition: all 0.2s;
    gap: 4px;
  }
  .ant-input-affix-wrapper:hover {
    border-color: var(--ant-color-primary-hover);
  }
  .ant-input-affix-wrapper:focus-within {
    border-color: var(--ant-color-primary);
    box-shadow: 0 0 0 2px rgba(22, 119, 255, 0.1);
  }
  :global([data-theme='dark']) .ant-input-affix-wrapper:focus-within {
    box-shadow: 0 0 0 2px rgba(22, 119, 255, 0.2);
  }
  .ant-input {
    width: 100%;
    border: none;
    outline: none;
    background: transparent;
    font-size: inherit;
    line-height: 1.5715;
    color: var(--ant-color-text);
    padding: 0;
  }
  .ant-input::placeholder {
    color: var(--ant-color-text-quaternary);
  }
  .ant-input-affix-wrapper:has(.ant-input[disabled]) {
    background: var(--ant-color-fill-tertiary);
  }
  .ant-input[disabled] {
    color: var(--ant-color-text-disabled);
    cursor: not-allowed;
  }
  .ant-input-prefix,
  .ant-input-suffix {
    display: inline-flex;
    align-items: center;
    color: var(--ant-color-text-quaternary);
  }
  .ant-input-suffix {
    cursor: pointer;
    color: var(--ant-color-text-tertiary);
  }
  .ant-input-suffix:hover {
    color: var(--ant-color-text);
  }
  .ant-input-textarea-wrapper {
    padding: 0;
    border: none;
    display: block;
  }
  .ant-input-textarea-wrapper:focus-within {
    box-shadow: none;
  }
  .ant-input-textarea-wrapper textarea.ant-input {
    padding: 4px 11px;
    border: 1px solid var(--ant-color-border);
    border-radius: var(--ant-border-radius);
    resize: vertical;
    min-height: 32px;
  }
  .ant-input-textarea-wrapper textarea.ant-input:focus {
    border-color: var(--ant-color-primary);
    box-shadow: 0 0 0 2px rgba(22, 119, 255, 0.1);
  }
  .ant-input-lg {
    padding: 6.5px 11px;
    font-size: var(--ant-font-size-lg);
  }
  .ant-input-sm {
    padding: 0 7px;
    font-size: var(--ant-font-size-sm);
  }
</style>
