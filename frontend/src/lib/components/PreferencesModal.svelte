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
  // 个人设置弹窗（复刻原 React PreferencesModal.tsx）
  // 主题 / 语言（个人覆盖系统默认）/ 新建会话位置 / 时区；确认时写入 preferencesStore
  import { t } from '$lib/i18n'
  import { setLocale } from '$lib/i18n'
  import { supportedLocales, localeDisplayName, resolveLocale } from '$lib/i18n'
  import { i18nState } from '$lib/i18n'
  import Modal from './Modal.svelte'
  import Radio from './Radio.svelte'
  import Select from './Select.svelte'
  import Input from './Input.svelte'
  import Button from './Button.svelte'
  import Space from './Space.svelte'
  import Title from './Title.svelte'
  import Text from './Text.svelte'
  import { message } from './message'
  import { preferencesStore, getEffectiveTheme } from '$lib/stores/preferences'

  let {
    open = false,
    onClose,
  }: { open?: boolean; onClose?: () => void } = $props()

  let themeVal = $state<'light' | 'dark' | 'system'>($preferencesStore.theme)
  let newConvPos = $state<'first' | 'last'>($preferencesStore.newConvPosition)
  let tzMode = $state<'system' | 'manual'>($preferencesStore.timezoneMode)
  let offsetStr = $state(String($preferencesStore.timezoneOffset))
  let languageVal = $state($preferencesStore.language || 'system')

  // 「跟随系统」标签动态显示当前解析结果（如：跟随系统（简体中文）），
  // 便于确认浏览器语言被识别成了什么。
  let followSystemLabel = $derived.by(() => {
    const resolved = resolveLocale('system')
    const name = localeDisplayName(resolved)
    return resolved === 'en-US' && !supportedLocales().includes('en-US')
      ? t('i18n.followSystem')
      : `${t('i18n.followSystem')}（${name}）`
  })

  let languageOptions = $derived([
    { value: 'system', label: followSystemLabel },
    ...supportedLocales()
      .filter((l) => l !== 'system')
      .map((l) => ({ value: l, label: localeDisplayName(l) })),
  ])

  function getSystemTzLabel(): string {
    const offsetMin = new Date().getTimezoneOffset()
    const hours = -Math.round(offsetMin / 60)
    const sign = hours >= 0 ? '+' : ''
    return t('preferences.followSystemTz', { sign, hours })
  }

  function getSystemThemeLabel(): string {
    return getEffectiveTheme('system') === 'dark'
      ? t('preferences.followSystemDark')
      : t('preferences.followSystemLight')
  }

  function getTzLabel(val: string): string {
    const trimmed = val.trim()
    if (trimmed !== '' && /^[+-]?\d+$/.test(trimmed) && Number(trimmed) > 0 && !trimmed.startsWith('+')) {
      return 'UTC+'
    }
    return 'UTC'
  }

  function handleOk() {
    preferencesStore.updateTheme(themeVal)
    preferencesStore.updateTimezoneMode(tzMode)
    if (tzMode === 'manual') {
      const trimmed = offsetStr.trim()
      if (!/^[+-]?\d+$/.test(trimmed)) {
        message.error(t('preferences.tzOffsetFormatError'))
        return
      }
      preferencesStore.updateTimezoneOffset(Number(trimmed))
    }
    preferencesStore.updateNewConvPosition(newConvPos)
    // 个人语言：写入偏好并立即生效（仅本账号，覆盖系统默认语言）。
    preferencesStore.updateLanguage(languageVal)
    setLocale(languageVal as 'system' | 'en-US' | 'zh-CN')
    onClose?.()
  }

  function onOffsetInput(v: string) {
    if (v !== '' && !/^[+-]?\d*$/.test(v)) {
      message.warning(t('preferences.tzOffsetOnly'))
      return
    }
    offsetStr = v
  }
</script>

<Modal open={open} title={t('preferences.title')} width={520} onclose={onClose} onOk={handleOk} okText={t('preferences.confirm')}>
  <div style="margin-bottom:24px">
    <Title level={5}>{t('preferences.theme')}</Title>
    <Radio
      options={[
        { value: 'light', label: t('preferences.light') },
        { value: 'dark', label: t('preferences.dark') },
        { value: 'system', label: getSystemThemeLabel() },
      ]}
      value={themeVal}
      onChange={(v) => (themeVal = String(v) as 'light' | 'dark' | 'system')}
    />
  </div>

  <div style="margin-bottom:24px">
    <Title level={5}>{t('preferences.language')}</Title>
    <Select
      value={languageVal}
      options={languageOptions}
      width="100%"
      onChange={(v) => (languageVal = String(v || 'system'))}
    />
    <Text type="secondary" style="font-size:12px;margin-top:4px;display:block">
      {t('preferences.languageHint')}
    </Text>
  </div>

  <div style="margin-bottom:24px">
    <Title level={5}>{t('preferences.newConvPosition')}</Title>
    <Radio
      options={[
        { value: 'first', label: t('preferences.first') },
        { value: 'last', label: t('preferences.last') },
      ]}
      value={newConvPos}
      onChange={(v) => (newConvPos = String(v) as 'first' | 'last')}
    />
  </div>

  <div>
    <Title level={5}>{t('preferences.timezone')}</Title>
    <Space direction="vertical" style="width:100%" align="start">
      <Radio
        options={[
          { value: 'system', label: getSystemTzLabel() },
          { value: 'manual', label: t('preferences.manual') },
        ]}
        value={tzMode}
        onChange={(v) => (tzMode = String(v) as 'system' | 'manual')}
      />
      {#if tzMode === 'manual'}
        <Space>
          <Text>{getTzLabel(offsetStr)}</Text>
          <Input value={offsetStr} onInput={onOffsetInput} style="width:120px" placeholder={t('preferences.tzOffsetPlaceholder')} />
        </Space>
      {/if}
    </Space>
  </div>
</Modal>
