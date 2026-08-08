<script lang="ts">
  // 系统设置页（复刻原 frontend/src/pages/Settings.tsx）
  // 权限守卫：system:settings（403 Result，文案同原 ProtectedRoute）
  import { onMount } from 'svelte'
  import { authStore } from '$lib/stores/auth'
  import { getSystemSettings, updateSystemSettings } from '$lib/api/system'
  import { getApiError } from '$lib/api/client'
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

  const units = ['B', 'KB', 'MB', 'GB', 'TB', '无限制', '禁止']

  const themeOptions = [
    { value: 'system', label: '跟随系统' },
    { value: 'light', label: '浅色' },
    { value: 'dark', label: '深色' },
  ]

  function parseSetting(value: string): { size: string; unit: string } {
    if (value === '无限制' || value === '禁止') return { size: '', unit: value }
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
  let maxFailures = $state('5')
  let lockWindow = $state('900')
  let saving = $state(false)

  let orig = $state({
    uploadLimit: '',
    loginTheme: 'system',
    siteTitle: '',
    loginSiteTitle: '',
    maxFailures: '5',
    lockWindow: '900',
  })

  let hideInput = $derived(unit === '无限制' || unit === '禁止')
  let uploadVal = $derived(hideInput ? unit : `${size}${unit}`)

  let changed = $derived(
    uploadVal !== orig.uploadLimit ||
      loginTheme !== orig.loginTheme ||
      siteTitle !== orig.siteTitle ||
      loginSiteTitle !== orig.loginSiteTitle ||
      maxFailures !== orig.maxFailures ||
      lockWindow !== orig.lockWindow,
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
      const mf = String(data.login_max_failures || '5')
      maxFailures = mf
      const lw = String(data.login_lock_window_secs || '900')
      lockWindow = lw
      orig = {
        uploadLimit: val,
        loginTheme: theme,
        siteTitle: title,
        loginSiteTitle: loginTitle,
        maxFailures: mf,
        lockWindow: lw,
      }
    } catch {
      /* 加载失败保持默认值（原版静默处理） */
    }
  })

  const handleSave = async () => {
    if (!hideInput) {
      if (!size) {
        message.error('请输入文件大小')
        return
      }
      const num = Number(size)
      if (!Number.isInteger(num) || num <= 0) {
        message.error('请输入正整数')
        return
      }
    }
    const mfNum = Number(maxFailures)
    if (!Number.isInteger(mfNum) || mfNum < 1 || mfNum > 100) {
      message.error('登录失败次数上限需为 1~100 的整数')
      return
    }
    const lwNum = Number(lockWindow)
    if (!Number.isInteger(lwNum) || lwNum < 1 || lwNum > 86400) {
      message.error('锁定窗口需为 1~86400 的整数(秒)')
      return
    }
    saving = true
    try {
      await updateSystemSettings({
        chat_upload_limit: uploadVal,
        login_theme: loginTheme,
        site_title: siteTitle,
        login_site_title: loginSiteTitle,
        login_max_failures: maxFailures,
        login_lock_window_secs: lockWindow,
      })
      const titleChanged =
        siteTitle !== orig.siteTitle || loginSiteTitle !== orig.loginSiteTitle
      orig = {
        uploadLimit: uploadVal,
        loginTheme,
        siteTitle,
        loginSiteTitle,
        maxFailures,
        lockWindow,
      }
      message.success('保存成功')
      if (titleChanged) {
        setTimeout(() => location.reload(), 800)
      }
    } catch (err) {
      message.error(getApiError(err, '保存失败'))
    }
    saving = false
  }
</script>

{#if !allowed}
  <Result status="403" title="403" subTitle="抱歉，您没有访问此页面的权限。">
    {#snippet extra()}
      <Button type="primary" onClick={() => window.history.back()}>返回</Button>
    {/snippet}
  </Result>
{:else}
  <Card title="系统设置">
    {#snippet extra()}
      <Tooltip title={changed ? undefined : '没有更改'}>
        <Button type="primary" loading={saving} onClick={handleSave} disabled={!changed}>
          确认
        </Button>
      </Tooltip>
    {/snippet}
    <Form>
      <div style="margin-bottom:24px">
        <Title level={5}>聊天文件上传大小限制</Title>
        <Space>
          {#if !hideInput}
            <Input
              value={size}
              onInput={(v) => (size = v)}
              placeholder="请输入大小"
              style="width:160px"
            />
          {/if}
          <Select
            value={unit}
            onChange={(v) => (unit = String(v))}
            width="120px"
            options={units.map((u) => ({ value: u, label: u }))}
          />
        </Space>
      </div>

      <Divider />

      <div style="margin-bottom:24px">
        <Title level={5}>登录页的网站标题</Title>
        <Input
          value={loginSiteTitle}
          onInput={(v) => (loginSiteTitle = v)}
          placeholder="例如：企业管理系统"
          style="width:300px"
        />
      </div>

      <Divider />

      <div style="margin-bottom:24px">
        <Title level={5}>登录后的网站标题</Title>
        <Input
          value={siteTitle}
          onInput={(v) => (siteTitle = v)}
          placeholder="例如：企业管理系统"
          style="width:300px"
        />
      </div>

      <Divider />

      <div style="margin-bottom:24px">
        <Title level={5}>登录页面主题</Title>
        <Radio
          options={themeOptions}
          value={loginTheme}
          onChange={(v) => (loginTheme = String(v))}
        />
      </div>

      <Divider />

      <div style="margin-bottom:24px">
        <Title level={5}>登录安全限制</Title>
        <div style="display:flex;flex-direction:column;gap:12px">
          <div style="display:flex;align-items:center;gap:12px">
            <span style="width:140px;flex-shrink:0">失败次数上限</span>
            <Input
              value={maxFailures}
              onInput={(v) => (maxFailures = v)}
              placeholder="1~100"
              style="width:120px;flex-shrink:0"
            />
            <span style="color:#999">次（同一 IP 或用户名在窗口内失败达上限即锁定）</span>
          </div>
          <div style="display:flex;align-items:center;gap:12px">
            <span style="width:140px;flex-shrink:0">锁定窗口</span>
            <Input
              value={lockWindow}
              onInput={(v) => (lockWindow = v)}
              placeholder="1~86400"
              style="width:120px;flex-shrink:0"
            />
            <span style="color:#999">秒（保存后立即生效，无需重启）</span>
          </div>
        </div>
      </div>
    </Form>
  </Card>
{/if}
