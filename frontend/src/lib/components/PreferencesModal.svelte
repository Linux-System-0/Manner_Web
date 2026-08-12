<script lang="ts">
  // 个人设置弹窗（复刻原 React PreferencesModal.tsx）
  // 主题 / 新建会话位置 / 时区；确认时写入 preferencesStore
  import { t } from '$lib/i18n'
  import Modal from './Modal.svelte'
  import Radio from './Radio.svelte'
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

<Modal open={open} title={t('preferences.title')} width={480} onclose={onClose} onOk={handleOk} okText={t('preferences.confirm')}>
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
