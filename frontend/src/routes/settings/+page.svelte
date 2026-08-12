<script lang="ts">
  // 系统设置页（复刻原 frontend/src/pages/Settings.tsx）
  // 权限守卫：system:settings（403 Result，文案同原 ProtectedRoute）
  import { onMount } from 'svelte'
  import { authStore } from '$lib/stores/auth'
  import { getSystemSettings, updateSystemSettings, uploadImage } from '$lib/api/system'
  import { getApiError } from '$lib/api/client'
  import { t } from '$lib/i18n'
  import { setLocale, supportedLocales, localeDisplayName, type LanguageMode } from '$lib/i18n'
  import { message } from '$lib/components/message'
  import Card from '$lib/components/Card.svelte'
  import Form from '$lib/components/Form.svelte'
  import Input from '$lib/components/Input.svelte'
  import Select from '$lib/components/Select.svelte'
  import Radio from '$lib/components/Radio.svelte'
  import Button from '$lib/components/Button.svelte'
  import Tooltip from '$lib/components/Tooltip.svelte'
  import Divider from '$lib/components/Divider.svelte'
  import Space from '$lib/components/Space.svelte'
  import Result from '$lib/components/Result.svelte'
  import Title from '$lib/components/Title.svelte'
  import Upload from '$lib/components/Upload.svelte'

  // 单位选项：value 为后端存储值（大小单位字母 或 中文字面量），label 随语言包显示
  const UNLIMITED_TOKEN = '无限制'
  const BANNED_TOKEN = '禁止'
  const units = $derived([
    { value: 'B', label: 'B' },
    { value: 'KB', label: 'KB' },
    { value: 'MB', label: 'MB' },
    { value: 'GB', label: 'GB' },
    { value: 'TB', label: 'TB' },
    { value: UNLIMITED_TOKEN, label: t('settings.unlimited') },
    { value: BANNED_TOKEN, label: t('settings.banned') },
  ])

  const themeOptions = $derived([
    { value: 'system', label: t('settings.followSystem') },
    { value: 'light', label: t('settings.light') },
    { value: 'dark', label: t('settings.dark') },
  ])

  // 语言选项：跟随系统 + 自动扫描的语言包（添加语言包后此处自动出现）
  const languageOptions = $derived([
    { value: 'system', label: t('settings.languageSystem') },
    ...supportedLocales().map((code) => ({ value: code, label: localeDisplayName(code) })),
  ])

  function parseSetting(value: string): { size: string; unit: string } {
    if (value === UNLIMITED_TOKEN || value === BANNED_TOKEN) return { size: '', unit: value }
    const match = value.match(/^(\d+)([A-Za-z]+)$/)
    if (match) return { size: match[1], unit: match[2] }
    return { size: '', unit: 'MB' }
  }

  let allowed = $derived($authStore.permissions.includes('system:settings'))

  let size = $state('')
  let unit = $state('MB')
  let loginTheme = $state('system')
  let siteTitle = $state('')
  let loginSiteTitle = $state('')
  let loginSiteIcon = $state('')
  let siteIcon = $state('')
  let maxFailures = $state('5')
  let lockWindow = $state('900')
  let defaultLanguage = $state<LanguageMode>('system')
  let saving = $state(false)

  let orig = $state({
    uploadLimit: '',
    loginTheme: 'system',
    siteTitle: '',
    loginSiteTitle: '',
    loginSiteIcon: '',
    siteIcon: '',
    maxFailures: '5',
    lockWindow: '900',
    defaultLanguage: 'system' as LanguageMode,
  })

  let hideInput = $derived(unit === UNLIMITED_TOKEN || unit === BANNED_TOKEN)
  let uploadVal = $derived(hideInput ? unit : `${size}${unit}`)

  let changed = $derived(
    uploadVal !== orig.uploadLimit ||
      loginTheme !== orig.loginTheme ||
      siteTitle !== orig.siteTitle ||
      loginSiteTitle !== orig.loginSiteTitle ||
      loginSiteIcon !== orig.loginSiteIcon ||
      siteIcon !== orig.siteIcon ||
      maxFailures !== orig.maxFailures ||
      lockWindow !== orig.lockWindow ||
      defaultLanguage !== orig.defaultLanguage,
  )

  onMount(async () => {
    if (!allowed) return
    try {
      const res = await getSystemSettings()
      const data = res.data || {}
      const val = String(data.chat_upload_limit || '')
      if (val) {
        const parsed = parseSetting(val)
        size = parsed.size
        unit = parsed.unit
      }
      const theme = String(data.login_theme || 'system')
      loginTheme = theme
      const title = String(data.site_title || '')
      siteTitle = title
      const loginTitle = String(data.login_site_title || '')
      loginSiteTitle = loginTitle
      loginSiteIcon = String(data.login_site_icon || '')
      siteIcon = String(data.site_icon || '')
      const mf = String(data.login_max_failures || '5')
      maxFailures = mf
      const lw = String(data.login_lock_window_secs || '900')
      lockWindow = lw
      const dl = String(data.default_language || 'system')
      defaultLanguage = (dl === 'system' || supportedLocales().includes(dl)) ? dl : 'system'
      orig = {
        uploadLimit: val,
        loginTheme: theme,
        siteTitle: title,
        loginSiteTitle: loginTitle,
        loginSiteIcon,
        siteIcon,
        maxFailures: mf,
        lockWindow: lw,
        defaultLanguage,
      }
    } catch {
      /* 加载失败保持默认值（原版静默处理） */
    }
  })

  const handleSave = async () => {
    if (!hideInput) {
      if (!size) {
        message.error(t('settings.errSize'))
        return
      }
      const num = Number(size)
      if (!Number.isInteger(num) || num <= 0) {
        message.error(t('settings.errPositiveInt'))
        return
      }
    }
    const mfNum = Number(maxFailures)
    if (!Number.isInteger(mfNum) || mfNum < 1 || mfNum > 100) {
      message.error(t('settings.errMaxFailures'))
      return
    }
    const lwNum = Number(lockWindow)
    if (!Number.isInteger(lwNum) || lwNum < 1 || lwNum > 86400) {
      message.error(t('settings.errLockWindow'))
      return
    }
    saving = true
    try {
      await updateSystemSettings({
        chat_upload_limit: uploadVal,
        login_theme: loginTheme,
        site_title: siteTitle,
        login_site_title: loginSiteTitle,
        login_site_icon: loginSiteIcon,
        site_icon: siteIcon,
        login_max_failures: maxFailures,
        login_lock_window_secs: lockWindow,
        default_language: defaultLanguage,
      })
      const titleChanged =
        siteTitle !== orig.siteTitle ||
        loginSiteTitle !== orig.loginSiteTitle ||
        loginSiteIcon !== orig.loginSiteIcon ||
        siteIcon !== orig.siteIcon
      const langChanged = defaultLanguage !== orig.defaultLanguage
      orig = {
        uploadLimit: uploadVal,
        loginTheme,
        siteTitle,
        loginSiteTitle,
        loginSiteIcon,
        siteIcon,
        maxFailures,
        lockWindow,
        defaultLanguage,
      }
      message.success(t('common.saved'))
      // 语言包变更立即生效，无需整页刷新（站点标题变更仍需刷新以更新 document.title）
      if (langChanged) {
        setLocale(defaultLanguage)
      }
      if (titleChanged) {
        setTimeout(() => location.reload(), 800)
      }
    } catch (err) {
      message.error(getApiError(err, t('common.savedFailed')))
    }
    saving = false
  }

  async function handleLoginIconUpload(file: File) {
    try {
      const url = await uploadImage(file)
      loginSiteIcon = url
      message.success(t('settings.loginIconUploaded'))
    } catch (err: unknown) {
      message.error(getApiError(err, t('settings.uploadFailed')))
    }
  }

  async function handleSiteIconUpload(file: File) {
    try {
      const url = await uploadImage(file)
      siteIcon = url
      message.success(t('settings.siteIconUploaded'))
    } catch (err: unknown) {
      message.error(getApiError(err, t('settings.uploadFailed')))
    }
  }
</script>

<div style="height:100%;overflow:auto">
  {#if !allowed}
    <Result status="403" title="403" subTitle={t('common.noAccess')}>
      {#snippet extra()}
        <Button type="primary" tooltip={t('common.backPrev')} onClick={() => window.history.back()}>{t('common.backPrev')}</Button>
      {/snippet}
    </Result>
  {:else}
    <Card title={t('settings.title')}>
      {#snippet extra()}
        <Tooltip title={changed ? t('settings.saveTooltip') : t('settings.noChange')}>
          <Button type="primary" loading={saving} onClick={handleSave} disabled={!changed} tooltip={undefined}>
            {t('settings.confirm')}
          </Button>
        </Tooltip>
      {/snippet}
      <Form>
        <div style="margin-bottom:24px">
          <Title level={5}>{t('settings.uploadLimit')}</Title>
          <Space>
            {#if !hideInput}
              <Input
                value={size}
                onInput={(v) => (size = v)}
                placeholder={t('settings.sizePlaceholder')}
                style="width:160px"
              />
            {/if}
            <Select
              value={unit}
              onChange={(v) => (unit = String(v))}
              width="120px"
              options={units}
            />
          </Space>
        </div>
  
        <Divider />
  
        <div style="margin-bottom:24px">
          <Title level={5}>{t('settings.loginSiteTitle')}</Title>
          <Input
            value={loginSiteTitle}
            onInput={(v) => (loginSiteTitle = v)}
            placeholder={t('settings.siteTitlePlaceholder')}
            style="width:300px"
          />
        </div>
  
        <Divider />
  
        <div style="margin-bottom:24px">
          <Title level={5}>{t('settings.loginSiteIcon')}</Title>
          <Space>
            <Upload
              accept="image/*"
              beforeUpload={(file) => {
                handleLoginIconUpload(file)
                return false
              }}
            >
              <Button tooltip={t('settings.uploadLoginIconTooltip')}>{t('settings.uploadIcon')}</Button>
            </Upload>
            {#if loginSiteIcon}
              <img
                src={`/api/system/icon/login?v=${Date.now()}`}
                alt={t('settings.loginIconAlt')}
                style="width:24px;height:24px;object-fit:contain;border:1px solid var(--ant-color-border);border-radius:4px"
              />
              <Button type="text" tooltip={t('settings.clearLoginIconTooltip')} onClick={() => (loginSiteIcon = '')}>{t('settings.clearLoginIcon')}</Button>
            {/if}
          </Space>
          <div style="color:#999;font-size:12px;margin-top:8px">
            {t('settings.iconFormatHint')}
          </div>
        </div>
  
        <Divider />
  
        <div style="margin-bottom:24px">
          <Title level={5}>{t('settings.siteTitle')}</Title>
          <Input
            value={siteTitle}
            onInput={(v) => (siteTitle = v)}
            placeholder={t('settings.siteTitlePlaceholder')}
            style="width:300px"
          />
        </div>
  
        <Divider />
  
        <div style="margin-bottom:24px">
          <Title level={5}>{t('settings.siteIcon')}</Title>
          <Space>
            <Upload
              accept="image/*"
              beforeUpload={(file) => {
                handleSiteIconUpload(file)
                return false
              }}
            >
              <Button tooltip={t('settings.uploadSiteIconTooltip')}>{t('settings.uploadIcon')}</Button>
            </Upload>
            {#if siteIcon}
              <img
                src={`/api/system/icon/site?v=${Date.now()}`}
                alt={t('settings.siteIconAlt')}
                style="width:24px;height:24px;object-fit:contain;border:1px solid var(--ant-color-border);border-radius:4px"
              />
              <Button type="text" tooltip={t('settings.clearSiteIconTooltip')} onClick={() => (siteIcon = '')}>{t('settings.clearSiteIcon')}</Button>
            {/if}
          </Space>
          <div style="color:#999;font-size:12px;margin-top:8px">
            {t('settings.iconFormatHint')}
          </div>
        </div>
  
        <Divider />
  
        <div style="margin-bottom:24px">
          <Title level={5}>{t('settings.loginTheme')}</Title>
          <Radio
            options={themeOptions}
            value={loginTheme}
            onChange={(v) => (loginTheme = String(v))}
          />
        </div>
  
        <Divider />
  
        <div style="margin-bottom:24px">
          <Title level={5}>{t('settings.defaultLanguage')}</Title>
          <Radio
            options={languageOptions}
            value={defaultLanguage}
            onChange={(v) => (defaultLanguage = String(v) as LanguageMode)}
          />
          <div style="color:#999;font-size:12px;margin-top:8px">
            {t('settings.defaultLanguageHint')}
          </div>
        </div>
  
        <Divider />
  
        <div style="margin-bottom:24px">
          <Title level={5}>{t('settings.loginSecurity')}</Title>
          <div style="display:flex;flex-direction:column;gap:12px">
            <div style="display:flex;align-items:center;gap:12px">
              <span style="width:140px;flex-shrink:0">{t('settings.maxFailures')}</span>
              <Input
                value={maxFailures}
                onInput={(v) => (maxFailures = v)}
                placeholder="1~100"
                style="width:120px;flex-shrink:0"
              />
              <span style="color:#999">{t('settings.maxFailuresHint')}</span>
            </div>
            <div style="display:flex;align-items:center;gap:12px">
              <span style="width:140px;flex-shrink:0">{t('settings.lockWindow')}</span>
              <Input
                value={lockWindow}
                onInput={(v) => (lockWindow = v)}
                placeholder="1~86400"
                style="width:120px;flex-shrink:0"
              />
              <span style="color:#999">{t('settings.lockWindowHint')}</span>
            </div>
          </div>
        </div>
      </Form>
    </Card>
  {/if}
</div>
