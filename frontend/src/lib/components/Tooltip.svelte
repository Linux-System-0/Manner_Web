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
  // Tooltip：气泡提示（hover 触发）
  // 位置基于触发元素实时计算（getBoundingClientRect），不再依赖 fixed 静态位，
  // 避免提示出现在元素下方过远处；滚动/缩放时自动跟随。
  import type { Snippet } from 'svelte'

  let {
    title = '',
    position = 'top',
    children,
    disabled = false,
    gap = 8,
    block = false,
    wrapperStyle = '',
  }: {
    title?: string
    position?: 'top' | 'bottom' | 'left' | 'right'
    children?: Snippet
    disabled?: boolean
    /** 气泡与触发元素之间的间距（px） */
    gap?: number
    /** 触发元素占满整行（用于 block 按钮） */
    block?: boolean
    /** 触发包装器的自定义样式（覆盖默认 display） */
    wrapperStyle?: string
  } = $props()

  let show = $state(false)
  let triggerEl = $state<HTMLElement | null>(null)
  let tipEl = $state<HTMLDivElement | null>(null)
  let pos = $state({ top: 0, left: 0 })

  function computePos() {
    if (!triggerEl || !tipEl) return
    const tr = triggerEl.getBoundingClientRect()
    const tw = tipEl.offsetWidth
    const th = tipEl.offsetHeight
    let top = 0
    let left = 0
    switch (position) {
      case 'bottom':
        top = tr.bottom + gap
        left = tr.left + tr.width / 2 - tw / 2
        break
      case 'left':
        top = tr.top + tr.height / 2 - th / 2
        left = tr.left - tw - gap
        break
      case 'right':
        top = tr.top + tr.height / 2 - th / 2
        left = tr.right + gap
        break
      default: // top
        top = tr.top - th - gap
        left = tr.left + tr.width / 2 - tw / 2
    }
    // 视口边界约束
    const margin = 4
    top = Math.max(margin, Math.min(top, window.innerHeight - th - margin))
    left = Math.max(margin, Math.min(left, window.innerWidth - tw - margin))
    pos = { top, left }
  }

  // show 或触发/气泡元素变化时重新定位
  $effect(() => {
    if (show && triggerEl && tipEl) {
      computePos()
      const onViewportChange = () => computePos()
      window.addEventListener('scroll', onViewportChange, true)
      window.addEventListener('resize', onViewportChange)
      return () => {
        window.removeEventListener('scroll', onViewportChange, true)
        window.removeEventListener('resize', onViewportChange)
      }
    }
  })
</script>

<!-- Tooltip 触发包装器：任意内容容器，需可聚焦以支持键盘显示提示 -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<span
  bind:this={triggerEl}
  class="ant-tooltip-trigger"
  style={wrapperStyle || `display:${block ? 'flex' : 'inline-flex'}`}
  role="note"
  tabindex={0}
  aria-describedby="ant-tooltip"
  onmouseenter={() => (show = true)}
  onmouseleave={() => (show = false)}
  onfocus={() => (show = true)}
  onblur={() => (show = false)}
>
  {#if children}{@render children()}{/if}
</span>

{#if show && title && !disabled}
  <div
    bind:this={tipEl}
    class="ant-tooltip ant-tooltip-placement-{position}"
    style="top:{pos.top}px;left:{pos.left}px"
    role="tooltip"
  >
    <div class="ant-tooltip-arrow"></div>
    <div class="ant-tooltip-inner">{title}</div>
  </div>
{/if}

<style>
  .ant-tooltip {
    position: fixed;
    z-index: 1070;
    max-width: 250px;
    padding: 2px 0;
    color: #fff;
    font-size: 12px;
    line-height: 1.4;
    pointer-events: none;
    /* 进入动画：淡入 + 沿 placement 方向轻微位移 */
    animation: ant-tooltip-in 0.15s ease-out;
  }
  @keyframes ant-tooltip-in {
    from {
      opacity: 0;
      transform: translateY(4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  .ant-tooltip-placement-top {
    animation-name: ant-tooltip-in-top;
  }
  @keyframes ant-tooltip-in-top {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  .ant-tooltip-inner {
    padding: 3px 8px;
    min-height: 24px;
    background: var(--ant-color-bg-spotlight);
    border-radius: var(--ant-border-radius);
    box-shadow: var(--ant-box-shadow);
    display: flex;
    align-items: center;
    word-break: break-word;
  }
  .ant-tooltip-arrow {
    position: absolute;
    width: 8px;
    height: 8px;
    background: var(--ant-color-bg-spotlight);
    transform: rotate(45deg);
  }
  .ant-tooltip-placement-top .ant-tooltip-arrow {
    bottom: 0;
    left: 50%;
    margin-left: -4px;
  }
  .ant-tooltip-placement-bottom .ant-tooltip-arrow {
    top: 0;
    left: 50%;
    margin-left: -4px;
  }
  .ant-tooltip-placement-left .ant-tooltip-arrow {
    right: 0;
    top: 50%;
    margin-top: -4px;
  }
  .ant-tooltip-placement-right .ant-tooltip-arrow {
    left: 0;
    top: 50%;
    margin-top: -4px;
  }
</style>
