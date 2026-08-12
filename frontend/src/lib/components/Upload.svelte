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
  // Upload：文件上传（隐藏 input，点击触发；beforeUpload 返回 false 时调用方自行上传）
  import { t } from '$lib/i18n'
  import type { Snippet } from 'svelte'

  let {
    accept,
    beforeUpload,
    children,
    disabled = false,
    multiple = false,
    onChange,
  }: {
    accept?: string
    beforeUpload?: (file: File) => boolean | Promise<boolean>
    children?: Snippet
    disabled?: boolean
    multiple?: boolean
    onChange?: (file: File) => void
  } = $props()

  let inputEl: HTMLInputElement | undefined = $state()

  async function handleFile(e: Event) {
    const input = e.target as HTMLInputElement
    const files = input.files
    if (!files || files.length === 0) return
    const file = files[0]
    input.value = ''
    if (disabled) return
    let ok = true
    if (beforeUpload) {
      ok = await beforeUpload(file)
    }
    if (ok) onChange?.(file)
  }
</script>

<span
  class="ant-upload-wrapper"
  style="display:inline-flex"
  role="button"
  tabindex={disabled ? -1 : 0}
  aria-label={t('common.upload')}
  onclick={() => !disabled && inputEl?.click()}
  onkeydown={(e) => {
    if (disabled) return
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      inputEl?.click()
    }
  }}
>
  {#if children}{@render children()}{/if}
  <input
    type="file"
    bind:this={inputEl}
    {accept}
    {multiple}
    style="display:none"
    onchange={handleFile}
  />
</span>
