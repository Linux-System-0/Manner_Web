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
  // Result：结果页（403/404/500/success）
  import type { Snippet } from 'svelte'
  import { Icon } from '$lib/icons'

  let {
    status = 'info',
    title = '',
    subTitle = '',
    extra,
    children,
  }: {
    status?: '403' | '404' | '500' | 'success' | 'info' | 'warning' | 'error'
    title?: string
    subTitle?: string
    extra?: Snippet
    children?: Snippet
  } = $props()

  let icon = $derived(
    status === '403' ? { name: 'stop', color: 'var(--ant-color-warning)' }
      : status === '404' ? { name: 'search', color: 'var(--ant-color-warning)' }
      : status === '500' ? { name: 'close-circle', color: 'var(--ant-color-error)' }
      : status === 'success' ? { name: 'check-circle', color: 'var(--ant-color-success)' }
      : status === 'warning' ? { name: 'exclamation-circle', color: 'var(--ant-color-warning)' }
      : status === 'error' ? { name: 'close-circle', color: 'var(--ant-color-error)' }
      : { name: 'info-circle', color: 'var(--ant-color-primary)' },
  )
</script>

<div class="ant-result" style="padding:48px 32px;text-align:center">
  <div class="ant-result-icon" style="margin-bottom:24px;line-height:1">
    <span style="color:{icon.color};display:inline-flex">
      <Icon name={icon.name} style="font-size:72px" />
    </span>
  </div>
  <div class="ant-result-title" style="font-size:24px;color:var(--ant-color-text);font-weight:600;line-height:1.8">{title}</div>
  {#if subTitle}
    <div class="ant-result-subtitle" style="font-size:14px;color:var(--ant-color-text-secondary);margin-top:8px">{subTitle}</div>
  {/if}
  {#if extra}
    <div class="ant-result-extra" style="margin-top:24px">{@render extra()}</div>
  {/if}
  {#if children}<div class="ant-result-content" style="margin-top:24px">{@render children()}</div>{/if}
</div>
