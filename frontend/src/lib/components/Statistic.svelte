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
  // Statistic：统计数值
  let {
    title = '',
    value = 0,
    suffix = '',
    prefix = '',
    style = '',
  }: {
    title?: string
    value?: number | string
    suffix?: string
    prefix?: string
    style?: string
  } = $props()
</script>

<div class="ant-statistic" style="{style}">
  <div class="ant-statistic-title" style="font-size:14px;color:var(--ant-color-text-secondary);margin-bottom:4px">{title}</div>
  <div class="ant-statistic-content" style="font-size:24px;font-weight:600;color:var(--ant-color-text);line-height:1.2">
    {#if prefix}<span class="ant-statistic-content-prefix" style="margin-right:4px">{prefix}</span>{/if}
    {value}
    {#if suffix}<span class="ant-statistic-content-suffix" style="margin-left:4px;font-size:14px;font-weight:400">{suffix}</span>{/if}
  </div>
</div>
