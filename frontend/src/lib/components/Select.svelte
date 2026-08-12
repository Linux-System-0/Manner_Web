<script lang="ts">
  // Select：下拉选择（复刻 antd 5 视觉；含 multiple / allowClear / 下拉面板）
  import { Icon } from '$lib/icons'
  import { t } from '$lib/i18n'
  import { onMount, tick } from 'svelte'

  export interface SelectOption {
    value: string | number
    label: string
    disabled?: boolean
  }

  let {
    value = undefined as string | number | Array<string | number> | undefined,
    options = [] as SelectOption[],
    multiple = false,
    placeholder = '',
    allowClear = false,
    disabled = false,
    onChange,
    style = '',
    class: className = '',
    width = '100%',
  }: {
    value?: string | number | Array<string | number> | undefined
    options?: SelectOption[]
    multiple?: boolean
    placeholder?: string
    allowClear?: boolean
    disabled?: boolean
    onChange?: (v: string | number | Array<string | number> | undefined) => void
    style?: string
    class?: string
    width?: string
  } = $props()

  let open = $state(false)
  let panelEl: HTMLDivElement | undefined = $state()
  let wrapEl: HTMLSpanElement | undefined = $state()

  let selectedArr = $derived(
    multiple
      ? (Array.isArray(value) ? value : value !== undefined ? [value] : []) as Array<string | number>
      : value !== undefined ? [value] as Array<string | number> : [],
  )
  let labelOf = $derived((v: string | number) => options.find((o) => o.value === v)?.label ?? String(v))
  let displayText = $derived(
    multiple ? selectedArr.map(labelOf).join('，') : selectedArr.length ? labelOf(selectedArr[0]) : '',
  )

  function toggle(v: string | number) {
    if (disabled) return
    if (multiple) {
      const arr = selectedArr.includes(v) ? selectedArr.filter((x) => x !== v) : [...selectedArr, v]
      onChange?.(arr)
    } else {
      onChange?.(v)
      open = false
    }
  }

  function clear(e: MouseEvent | KeyboardEvent) {
    e.stopPropagation()
    onChange?.(multiple ? [] : undefined)
  }

  function onDocClick(e: MouseEvent) {
    if (wrapEl && !wrapEl.contains(e.target as Node)) {
      open = false
    }
  }

  onMount(() => {
    document.addEventListener('click', onDocClick)
    return () => document.removeEventListener('click', onDocClick)
  })

  // 面板贴边修正（简单版：不处理边界翻转）
  $effect(() => {
    if (open && panelEl && wrapEl) {
      const rect = wrapEl.getBoundingClientRect()
      panelEl.style.top = rect.bottom + 4 + 'px'
      panelEl.style.left = rect.left + 'px'
      panelEl.style.minWidth = rect.width + 'px'
    }
  })
</script>

<span class="ant-select {className}" style="width:{width};{style}" bind:this={wrapEl}>
  <div
    class="ant-select-selector"
    class:ant-select-open={open}
    role="combobox"
    aria-expanded={open}
    aria-controls="ant-select-listbox"
    aria-haspopup="listbox"
    tabindex={disabled ? -1 : 0}
    onclick={() => !disabled && (open = !open)}
    onkeydown={(e) => {
      if (disabled) return
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault()
        open = !open
      }
      if (e.key === 'Escape') open = false
    }}
  >
    <span class="ant-select-selection-item" class:ant-select-selection-placeholder={!displayText}>
      {displayText || placeholder}
    </span>
    {#if allowClear && selectedArr.length > 0 && !disabled}
      <span
        class="ant-select-clear"
        role="button"
        tabindex={-1}
        aria-label={t('common.clear')}
        onclick={clear}
        onkeydown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault()
            clear(e)
          }
        }}
      ><Icon name="close" style="font-size:12px" /></span>
    {:else}
      <span class="ant-select-arrow"><Icon name="down" style="font-size:12px" /></span>
    {/if}
  </div>

  {#if open}
    <div class="ant-select-dropdown" id="ant-select-listbox" role="listbox" bind:this={panelEl}>
      <div class="ant-select-item-empty" class:ant-select-dropdown-empty={options.length === 0}>
        {#if options.length === 0}
          <div class="ant-empty">
            <div class="ant-empty-image"><Icon name="message" style="font-size:40px;color:var(--ant-color-text-quaternary)" /></div>
            <p class="ant-empty-description" style="color:var(--ant-color-text-tertiary);font-size:14px">{t('common.noData')}</p>
          </div>
        {:else}
          {#each options as opt (opt.value)}
            <div
              class="ant-select-item ant-select-item-option"
              class:ant-select-item-option-selected={selectedArr.includes(opt.value)}
              class:ant-select-item-option-disabled={opt.disabled}
              role="option"
              aria-selected={selectedArr.includes(opt.value)}
              tabindex={opt.disabled ? -1 : 0}
              onclick={() => !opt.disabled && toggle(opt.value)}
              onkeydown={(e) => {
                if (opt.disabled) return
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault()
                  toggle(opt.value)
                }
              }}
            >
              <div class="ant-select-item-option-content">{opt.label}</div>
              {#if multiple && selectedArr.includes(opt.value)}
                <span class="ant-select-selected-icon"><Icon name="check" style="font-size:12px" /></span>
              {/if}
            </div>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</span>

<style>
  .ant-select {
    position: relative;
    display: inline-block;
    box-sizing: border-box;
    font-size: var(--ant-font-size);
    vertical-align: middle;
  }
  .ant-select-selector {
    position: relative;
    display: flex;
    align-items: center;
    width: 100%;
    height: 32px;
    padding: 0 11px;
    color: var(--ant-color-text);
    background: var(--ant-color-bg-container);
    border: 1px solid var(--ant-color-border);
    border-radius: var(--ant-border-radius);
    cursor: pointer;
    transition: all 0.2s;
    gap: 4px;
  }
  .ant-select-selector:hover {
    border-color: var(--ant-color-primary-hover);
  }
  .ant-select-open {
    border-color: var(--ant-color-primary);
    box-shadow: 0 0 0 2px rgba(22, 119, 255, 0.1);
  }
  :global([data-theme='dark']) .ant-select-open {
    box-shadow: 0 0 0 2px rgba(22, 119, 255, 0.2);
  }
  .ant-select-selection-item {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .ant-select-selection-placeholder {
    color: var(--ant-color-text-quaternary);
  }
  .ant-select-arrow {
    display: inline-flex;
    align-items: center;
    color: var(--ant-color-text-quaternary);
    font-size: 12px;
    transition: transform 0.2s;
  }
  .ant-select-open .ant-select-arrow {
    transform: rotate(180deg);
  }
  .ant-select-clear {
    display: inline-flex;
    align-items: center;
    color: var(--ant-color-text-quaternary);
    cursor: pointer;
  }
  .ant-select-dropdown {
    position: fixed;
    z-index: 1050;
    padding: 4px;
    background: var(--ant-color-bg-elevated);
    border-radius: var(--ant-border-radius-lg);
    box-shadow: var(--ant-box-shadow);
    max-height: 256px;
    overflow-y: auto;
  }
  .ant-select-item-option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 5px 12px;
    font-size: var(--ant-font-size);
    color: var(--ant-color-text);
    cursor: pointer;
    border-radius: var(--ant-border-radius-sm);
    transition: background 0.2s;
    gap: 8px;
  }
  .ant-select-item-option:hover {
    background: var(--ant-color-fill-secondary);
  }
  .ant-select-item-option-selected {
    font-weight: 600;
    color: var(--ant-color-primary);
    background: rgba(22, 119, 255, 0.08);
  }
  .ant-select-item-option-disabled {
    color: var(--ant-color-text-disabled);
    cursor: not-allowed;
  }
  .ant-select-item-option-content {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .ant-select-selected-icon {
    display: inline-flex;
    color: var(--ant-color-primary);
  }
  .ant-select-dropdown-empty {
    padding: 16px 0;
    text-align: center;
  }
</style>
