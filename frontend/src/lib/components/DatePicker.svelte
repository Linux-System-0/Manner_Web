<script lang="ts">
  // DatePicker：日期选择（月历面板，value 为 YYYY-MM-DD）
  import { Icon } from '$lib/icons'
  import { onMount } from 'svelte'

  let {
    value = '',
    placeholder = '请选择日期',
    onChange,
    disabled = false,
    style = '',
  }: {
    value?: string
    placeholder?: string
    onChange?: (v: string) => void
    disabled?: boolean
    style?: string
  } = $props()

  let open = $state(false)
  let rootEl: HTMLSpanElement | undefined = $state()
  let panelEl: HTMLDivElement | undefined = $state()

  let now = $state(new Date())
  let viewYear = $state(new Date().getFullYear())
  let viewMonth = $state(new Date().getMonth())
  // 初始化时读取 value 一次（有意为之），后续变化由下方 $effect 同步
  // svelte-ignore state_referenced_locally
  let selected = $state(value)
  // svelte-ignore state_referenced_locally
  let lastValue = value

  $effect(() => {
    // 仅当外部 value 实际变化（如异步回填）时同步内部选中值；
    // 不追踪 selected，避免非受控用法下把用户新选择拉回旧值
    if (value !== lastValue) {
      lastValue = value
      selected = value
    }
  })

  let weekdays = ['日', '一', '二', '三', '四', '五', '六']
  let cells = $derived.by(() => {
    const first = new Date(viewYear, viewMonth, 1)
    const startWeek = first.getDay()
    const daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate()
    const arr: Array<{ day: number; iso: string; other?: boolean }> = []
    for (let i = 0; i < startWeek; i++) {
      const d = new Date(viewYear, viewMonth, 1 - (startWeek - i))
      arr.push({ day: d.getDate(), iso: isoOf(d), other: true })
    }
    for (let day = 1; day <= daysInMonth; day++) {
      const d = new Date(viewYear, viewMonth, day)
      arr.push({ day, iso: isoOf(d) })
    }
    return arr
  })

  function isoOf(d: Date): string {
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
  }

  function pick(iso: string) {
    selected = iso
    onChange?.(iso)
    open = false
  }

  function prevMonth() {
    viewMonth--
    if (viewMonth < 0) {
      viewMonth = 11
      viewYear--
    }
  }

  function nextMonth() {
    viewMonth++
    if (viewMonth > 11) {
      viewMonth = 0
      viewYear++
    }
  }

  function onDocClick(e: MouseEvent) {
    if (rootEl && !rootEl.contains(e.target as Node)) open = false
  }

  onMount(() => {
    document.addEventListener('click', onDocClick)
    return () => document.removeEventListener('click', onDocClick)
  })

  $effect(() => {
    if (open && panelEl && rootEl) {
      const r = rootEl.getBoundingClientRect()
      panelEl.style.top = r.bottom + 4 + 'px'
      panelEl.style.left = r.left + 'px'
    }
  })
</script>

<span
  class="ant-picker"
  bind:this={rootEl}
  style="position:relative;display:inline-flex;align-items:center;width:100%;height:32px;padding:0 11px;border:1px solid var(--ant-color-border);border-radius:var(--ant-border-radius);background:var(--ant-color-bg-container);cursor:pointer;{style}"
  role="button"
  tabindex={disabled ? -1 : 0}
  aria-haspopup="dialog"
  aria-expanded={open}
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
  <span style="color:var(--ant-color-text);flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{selected || placeholder}</span>
  <span style="color:var(--ant-color-text-quaternary);display:inline-flex"><Icon name="calendar" /></span>
</span>

{#if open}
  <div class="ant-picker-dropdown" bind:this={panelEl} style="position:fixed;z-index:1050;background:var(--ant-color-bg-elevated);border-radius:var(--ant-border-radius-lg);box-shadow:var(--ant-box-shadow);padding:12px;width:280px">
    <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:12px">
      <span
        class="ant-picker-header-super-prev-btn"
        role="button"
        tabindex={0}
        aria-label="上个月"
        onclick={prevMonth}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); prevMonth() } }}
        style="cursor:pointer;color:var(--ant-color-text-tertiary);display:inline-flex"
      ><Icon name="left" style="font-size:12px" /></span>
      <span style="font-weight:600;color:var(--ant-color-text)">{viewYear} 年 {viewMonth + 1} 月</span>
      <span
        class="ant-picker-header-next-btn"
        role="button"
        tabindex={0}
        aria-label="下个月"
        onclick={nextMonth}
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); nextMonth() } }}
        style="cursor:pointer;color:var(--ant-color-text-tertiary);display:inline-flex"
      ><Icon name="right" style="font-size:12px" /></span>
    </div>
    <div style="display:grid;grid-template-columns:repeat(7, 1fr);text-align:center;font-size:12px;color:var(--ant-color-text-secondary);margin-bottom:4px">
      {#each weekdays as w}<div style="padding:4px 0">{w}</div>{/each}
    </div>
    <div style="display:grid;grid-template-columns:repeat(7, 1fr);text-align:center;font-size:14px">
      {#each cells as cell (cell.iso)}
        <div
          class="ant-picker-cell"
          class:ant-picker-cell-selected={cell.iso === selected}
          class:ant-picker-cell-today={cell.iso === isoOf(now)}
          class:ant-picker-cell-other={cell.other}
          style="padding:4px;cursor:pointer;border-radius:var(--ant-border-radius-sm);color:{cell.other ? 'var(--ant-color-text-quaternary)' : 'var(--ant-color-text)'}"
          role="button"
          tabindex={0}
          aria-label={cell.iso}
          onclick={() => pick(cell.iso)}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault()
              pick(cell.iso)
            }
          }}
        >
          {cell.day}
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .ant-picker-cell-selected {
    background: var(--ant-color-primary);
    color: #fff !important;
    font-weight: 600;
  }
  .ant-picker-cell-today {
    box-shadow: 0 0 0 1px var(--ant-color-primary) inset;
  }
</style>
