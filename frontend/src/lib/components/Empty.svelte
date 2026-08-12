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
  // Empty：空态占位
  import { Icon } from '$lib/icons'
  import { t } from '$lib/i18n'
  import type { Snippet } from 'svelte'

  let {
    description = t('common.noData'),
    image,
    children,
  }: {
    description?: string
    image?: Snippet
    children?: Snippet
  } = $props()
</script>

<div class="ant-empty">
  <div class="ant-empty-image">
    {#if image}{@render image()}{:else}<Icon name="message" style="font-size:48px;color:var(--ant-color-text-quaternary)" />{/if}
  </div>
  <p class="ant-empty-description" style="color:var(--ant-color-text-tertiary);font-size:14px;margin:8px 0 0">{description}</p>
  {#if children}
    <div class="ant-empty-footer" style="margin-top:16px">{@render children()}</div>
  {/if}
</div>

<style>
  .ant-empty {
    text-align: center;
    padding: 16px 0;
  }
  .ant-empty-image {
    display: flex;
    justify-content: center;
    line-height: 0;
  }
</style>
