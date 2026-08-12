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
  // Avatar：头像（图片或文字首字）
  import type { Snippet } from 'svelte'

  let {
    src,
    size = 32,
    shape = 'circle',
    children,
    style = '',
  }: {
    src?: string | null
    size?: number | 'small' | 'large'
    shape?: 'circle' | 'square'
    children?: Snippet
    style?: string
  } = $props()

  let px = $derived(
    typeof size === 'number' ? size : size === 'large' ? 40 : size === 'small' ? 24 : 32,
  )
</script>

<span
  class="ant-avatar ant-avatar-{shape}"
  style="width:{px}px;height:{px}px;font-size:{px / 2}px;line-height:{px}px;background:var(--ant-color-primary);color:#fff;{style}"
>
  {#if src}
    <img src={src} alt="avatar" style="width:100%;height:100%;object-fit:cover" />
  {:else if children}
    {@render children()}
  {/if}
</span>

<style>
  .ant-avatar {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    vertical-align: middle;
    text-align: center;
    background: #ccc;
    color: #fff;
    white-space: nowrap;
    position: relative;
    font-weight: 400;
  }
  .ant-avatar-circle {
    border-radius: 50%;
  }
  .ant-avatar-square {
    border-radius: var(--ant-border-radius-sm);
  }
</style>
