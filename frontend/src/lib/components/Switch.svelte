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
  // Switch：开关
  import { t } from '$lib/i18n'

  let {
    checked = false,
    disabled = false,
    onChange,
    loading = false,
  }: {
    checked?: boolean
    disabled?: boolean
    onChange?: (v: boolean) => void
    loading?: boolean
  } = $props()
</script>

<button
  type="button"
  class="ant-switch"
  class:ant-switch-checked={checked}
  class:ant-switch-disabled={disabled}
  role="switch"
  aria-checked={checked}
  aria-label={t('component.switch')}
  disabled={disabled || loading}
  onclick={() => onChange?.(!checked)}
>
  <div class="ant-switch-handle"></div>
  <span class="ant-switch-inner"></span>
</button>

<style>
  .ant-switch {
    position: relative;
    display: inline-block;
    width: 44px;
    height: 22px;
    min-width: 44px;
    background: var(--ant-color-fill-quaternary);
    border: 0;
    border-radius: 100px;
    cursor: pointer;
    transition: all 0.2s;
    padding: 0;
    vertical-align: middle;
  }
  .ant-switch-handle {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 2px 4px rgba(0, 35, 11, 0.2);
    transition: all 0.2s;
  }
  .ant-switch-checked {
    background: var(--ant-color-primary);
  }
  .ant-switch-checked .ant-switch-handle {
    left: 24px;
  }
  .ant-switch-disabled {
    cursor: not-allowed;
    opacity: 0.4;
  }
</style>
