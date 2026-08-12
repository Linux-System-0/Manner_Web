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
