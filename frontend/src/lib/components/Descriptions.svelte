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
  // Descriptions：描述列表
  export interface DescriptionsItem {
    label: string
    value: string | number | null | undefined
    span?: number
  }

  let {
    items = [] as DescriptionsItem[],
    column = 3,
    bordered = false,
    title = '',
    size = 'default',
    style = '',
  }: {
    items?: DescriptionsItem[]
    column?: number
    bordered?: boolean
    title?: string
    size?: 'small' | 'middle' | 'default'
    style?: string
  } = $props()

  let pad = $derived(size === 'small' ? '8px 12px' : size === 'middle' ? '12px 16px' : '16px 24px')
</script>

<div class="ant-descriptions" style="{style}">
  {#if title}<div class="ant-descriptions-title" style="font-size:16px;font-weight:600;color:var(--ant-color-text);margin-bottom:16px">{title}</div>{/if}
  <table style="width:100%;border-collapse:collapse">
    <tbody>
      {#each items as item (item.label)}
        <tr>
          <th class="ant-descriptions-item-label" style="padding:{pad};font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top">
            {item.label}
          </th>
          <td class="ant-descriptions-item-content" colspan={item.span ?? 1} style="padding:{pad};color:var(--ant-color-text);border:1px solid var(--ant-color-border-secondary);vertical-align:top">
            {item.value === null || item.value === undefined ? '-' : item.value}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
