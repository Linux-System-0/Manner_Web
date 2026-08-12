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
  // 登录页：复刻原 React src/pages/Login.tsx
  // - 三步登录：用户名 → precheck →（首次登录）设置密码 /（否则）输入密码
  // - 注册引导：GET /system/login-page，registration_open=true 时进入注册模式
  // - 主题：login_theme（system/light/dark）→ html[data-theme]；system 模式跟随系统
  // - 已登录访问 /login 时自动跳转首页
  import { onMount } from 'svelte'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { getEffectiveTheme, preferencesStore } from '$lib/stores/preferences'
  import { t } from '$lib/i18n'
  import {
    login as loginApi,
    register as registerApi,
    precheck,
    firstLogin,
  } from '$lib/api/auth'
  import { getLoginPage } from '$lib/api/system'
  import { getApiError } from '$lib/api/client'
  import { setFavicon } from '$lib/utils/favicon'
  import type { LoginPageInfo } from '$lib/types'
  import { message } from '$lib/components/message'
  import Card from '$lib/components/Card.svelte'
  import Title from '$lib/components/Title.svelte'
  import Form from '$lib/components/Form.svelte'
  import FormItem from '$lib/components/FormItem.svelte'
  import Input from '$lib/components/Input.svelte'
  import Button from '$lib/components/Button.svelte'

  type Mode = 'login' | 'register'
  type Step = 'username' | 'password' | 'setup'

  // ---- 页面状态（对齐原版 useState） ----
  let loading = $state(false)
  let loginTheme = $state<'light' | 'dark' | 'system'>('system')
  let siteTitle = $state(t('login.siteTitle'))
  let effectiveTheme = $state<'light' | 'dark'>('light')
  let mode = $state<Mode>('login')
  let step = $state<Step>('username')
  let username = $state('')

  // 登录三步表单值
  let usernameValue = $state('')
  let passwordValue = $state('')
  let initialPassword = $state('')
  let newPassword = $state('')
  let confirmNewPassword = $state('')
  // 注册表单值
  let regUsername = $state('')
  let regName = $state('')
  let regEmail = $state('')
  let regPassword = $state('')
  let regConfirm = $state('')

  // 校验错误（FormItem error prop 展示，文案与原版 antd rules 一致）
  let usernameError = $state('')
  let passwordError = $state('')
  let initialPasswordError = $state('')
  let newPasswordError = $state('')
  let confirmNewPasswordError = $state('')
  let regUsernameError = $state('')
  let regNameError = $state('')
  let regEmailError = $state('')
  let regPasswordError = $state('')
  let regConfirmError = $state('')

  // 提交尝试标记：首次提交后才实时复检（对齐 antd 交互）
  let loginAttempted = $state(false)
  let setupAttempted = $state(false)
  let registerAttempted = $state(false)

  // 被「已在其他设备登录」踢下线后回到登录页的提示
  onMount(() => {
    try {
      if (sessionStorage.getItem('manner-logout-reason') === 'kicked') {
        sessionStorage.removeItem('manner-logout-reason')
        message.warning(t('login.kicked'))
      }
    } catch {
      /* ignore */
    }
  })

  // 已登录访问 /login → 回首页（replace 避免历史栈污染）
  $effect(() => {
    if ($authStore.isAuthenticated) {
      goto('/', { replaceState: true })
    }
  })

  // 主题：login_theme → html[data-theme]；system 模式监听系统主题变化
  $effect(() => {
    const effective = getEffectiveTheme(loginTheme)
    effectiveTheme = effective
    document.documentElement.setAttribute('data-theme', effective)
    if (loginTheme === 'system') {
      const mq = window.matchMedia('(prefers-color-scheme: dark)')
      const handler = () => {
        const t = getEffectiveTheme('system')
        effectiveTheme = t
        document.documentElement.setAttribute('data-theme', t)
      }
      mq.addEventListener('change', handler)
      return () => mq.removeEventListener('change', handler)
    }
  })

  // 初始化：拉取登录页配置（站点标题 / 登录主题 / 是否开放注册）
  $effect(() => {
    getLoginPage()
      .then((res) => {
        const data = (res.data || {}) as LoginPageInfo
        loginTheme = data.login_theme || 'system'
        const loginTitle = data.login_site_title || data.site_title || ''
        if (loginTitle) {
          siteTitle = loginTitle
          document.title = loginTitle
        }
        setFavicon(data.login_site_icon ? `/api/system/icon/login?v=${Date.now()}` : null)
        if (typeof data.registration_open === 'boolean') {
          // 无任何账号时自动进入注册引导；已有账号则直接显示登录
          mode = data.registration_open ? 'register' : 'login'
        }
      })
      .catch(() => {})
  })

  // ---- 校验（文案与原版一致） ----
  const EMAIL_RE =
    /^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$/

  function checkUsernameStep(): boolean {
    usernameError = usernameValue ? '' : t('login.errUsername')
    return !usernameError
  }

  function checkPasswordStep(): boolean {
    passwordError = passwordValue ? '' : t('login.errPassword')
    return !passwordError
  }

  function checkSetupStep(): boolean {
    initialPasswordError = initialPassword ? '' : t('login.errInitialPassword')
    newPasswordError = newPassword
      ? newPassword.length >= 8
        ? ''
        : t('login.errNewPasswordLen')
      : t('login.errNewPassword')
    confirmNewPasswordError = confirmNewPassword
      ? confirmNewPassword === newPassword
        ? ''
        : t('login.errPasswordMismatch')
      : t('login.errConfirmNewPassword')
    return !initialPasswordError && !newPasswordError && !confirmNewPasswordError
  }

  function checkRegister(): boolean {
    regUsernameError = regUsername
      ? regUsername.length >= 3 && regUsername.length <= 64
        ? ''
        : t('login.errUsernameLen')
      : t('login.errUsername')
    regNameError = regName ? '' : t('login.errName')
    regEmailError = regEmail && !EMAIL_RE.test(regEmail) ? t('login.errEmail') : ''
    regPasswordError = regPassword
      ? regPassword.length >= 8
        ? ''
        : t('login.errNewPasswordLen')
      : t('login.errPassword')
    regConfirmError = regConfirm
      ? regConfirm === regPassword
        ? ''
        : t('login.errPasswordMismatch')
      : t('login.errPassword')
    return (
      !regUsernameError &&
      !regNameError &&
      !regEmailError &&
      !regPasswordError &&
      !regConfirmError
    )
  }

  // ---- 输入事件（提交尝试后实时复检，对齐 antd 行为） ----
  function onUsernameInput(v: string) {
    usernameValue = v
    if (loginAttempted) usernameError = v ? '' : t('login.errUsername')
  }

  function onPasswordInput(v: string) {
    passwordValue = v
    if (loginAttempted) passwordError = v ? '' : t('login.errPassword')
  }

  function onInitialPasswordInput(v: string) {
    initialPassword = v
    if (setupAttempted) initialPasswordError = v ? '' : t('login.errInitialPassword')
  }

  function onNewPasswordInput(v: string) {
    newPassword = v
    if (setupAttempted) {
      newPasswordError = v ? (v.length >= 8 ? '' : t('login.errNewPasswordLen')) : t('login.errNewPassword')
      if (confirmNewPasswordError) {
        confirmNewPasswordError = confirmNewPassword
          ? confirmNewPassword === v
            ? ''
            : t('login.errPasswordMismatch')
          : t('login.errConfirmNewPassword')
      }
    }
  }

  function onConfirmNewPasswordInput(v: string) {
    confirmNewPassword = v
    if (setupAttempted) {
      confirmNewPasswordError = v
        ? v === newPassword
          ? ''
          : t('login.errPasswordMismatch')
        : t('login.errConfirmNewPassword')
    }
  }

  function onRegUsernameInput(v: string) {
    regUsername = v
    if (registerAttempted) {
      regUsernameError = v
        ? v.length >= 3 && v.length <= 64
          ? ''
          : t('login.errUsernameLen')
        : t('login.errUsername')
    }
  }

  function onRegNameInput(v: string) {
    regName = v
    if (registerAttempted) regNameError = v ? '' : t('login.errName')
  }

  function onRegEmailInput(v: string) {
    regEmail = v
    if (registerAttempted) regEmailError = v && !EMAIL_RE.test(v) ? t('login.errEmail') : ''
  }

  function onRegPasswordInput(v: string) {
    regPassword = v
    if (registerAttempted) {
      regPasswordError = v ? (v.length >= 8 ? '' : t('login.errNewPasswordLen')) : t('login.errPassword')
      if (regConfirmError) {
        regConfirmError = regConfirm
          ? regConfirm === v
            ? ''
            : t('login.errPasswordMismatch')
          : t('login.errPassword')
      }
    }
  }

  function onRegConfirmInput(v: string) {
    regConfirm = v
    if (registerAttempted) {
      regConfirmError = v
        ? v === regPassword
          ? ''
          : t('login.errPasswordMismatch')
        : t('login.errPassword')
    }
  }

  // ---- 提交逻辑（对齐原版 onFinish / onUsernameFinish / onSetupFinish / onRegisterFinish） ----
  async function handleUsernameSubmit(e: SubmitEvent) {
    e.preventDefault()
    if (loading) return
    loginAttempted = true
    if (!checkUsernameStep()) return
    loading = true
    try {
      username = usernameValue
      const res = await precheck(usernameValue)
      // 首次登录（待设置密码）→ 自动引导设置登录密码；否则进入密码步骤
      step = res.data.must_change ? 'setup' : 'password'
      // 对齐 antd：步骤表单卸载即重置
      passwordValue = ''
      passwordError = ''
    } catch (err: unknown) {
      message.error(getApiError(err, t('login.checkFailed')))
    } finally {
      loading = false
    }
  }

  async function handleLoginSubmit(e: SubmitEvent) {
    e.preventDefault()
    if (loading) return
    loginAttempted = true
    if (!checkPasswordStep()) return
    loading = true
    try {
      const res = await loginApi(username, passwordValue)
      authStore.setAuth(res.data.user)
      // 个人设置按用户存于服务端：登录成功后重新拉取该用户的偏好（而非沿用本机 localStorage）
      await preferencesStore.refresh()
      // 兜底：若预检未命中但登录返回待改密标记，仍引导改密
      if (res.data.user.must_change_password) {
        message.warning(t('login.changePasswordHint'))
        goto('/profile', { replaceState: true })
      } else {
        message.success(t('login.loginSuccess'))
        goto('/', { replaceState: true })
      }
    } catch (err: unknown) {
      message.error(getApiError(err, t('login.loginFailed')))
    } finally {
      loading = false
    }
  }

  async function handleSetupSubmit(e: SubmitEvent) {
    e.preventDefault()
    if (loading) return
    setupAttempted = true
    if (!checkSetupStep()) return
    if (newPassword !== confirmNewPassword) {
      message.error(t('login.errPasswordMismatch'))
      return
    }
    loading = true
    try {
      const res = await firstLogin({
        username,
        initial_password: initialPassword,
        new_password: newPassword,
      })
      authStore.setAuth(res.data.user)
      // 个人设置按用户存于服务端：登录成功后重新拉取该用户的偏好
      await preferencesStore.refresh()
      message.success(t('login.setupSuccess'))
      goto('/', { replaceState: true })
    } catch (err: unknown) {
      message.error(getApiError(err, t('login.setupFailed')))
    } finally {
      loading = false
    }
  }

  function backToUsername() {
    step = 'username'
    // 对齐 antd：表单卸载即重置
    passwordValue = ''
    passwordError = ''
    initialPassword = ''
    newPassword = ''
    confirmNewPassword = ''
    initialPasswordError = ''
    newPasswordError = ''
    confirmNewPasswordError = ''
  }

  async function handleRegisterSubmit(e: SubmitEvent) {
    e.preventDefault()
    if (loading) return
    registerAttempted = true
    if (!checkRegister()) return
    loading = true
    try {
      await registerApi({
        username: regUsername,
        password: regPassword,
        name: regName,
        email: regEmail,
      })
      message.success(t('login.registerSuccess'))
      mode = 'login'
      // 对齐 antd：表单卸载即重置
      regUsername = ''
      regName = ''
      regEmail = ''
      regPassword = ''
      regConfirm = ''
      regUsernameError = ''
      regNameError = ''
      regEmailError = ''
      regPasswordError = ''
      regConfirmError = ''
      registerAttempted = false
    } catch (err: unknown) {
      message.error(getApiError(err, t('login.registerFailed')))
    } finally {
      loading = false
    }
  }
</script>

<div
  style="display:flex;justify-content:center;align-items:center;min-height:100vh;background:{effectiveTheme === 'dark' ? '#141414' : '#f0f2f5'}"
>
  <Card style="width:400px">
    <div style="text-align:center;margin-bottom:32px">
      <Title level={3} style="margin:0">{siteTitle}</Title>
    </div>

    {#if mode === 'login'}
      {#if step === 'username'}
        <Form onSubmit={handleUsernameSubmit}>
          <FormItem error={usernameError}>
            <Input
              value={usernameValue}
              placeholder={t('login.enterUsername')}
              prefix="user"
              size="large"
              onInput={onUsernameInput}
            />
          </FormItem>
          <FormItem>
            <Button type="primary" htmlType="submit" block loading={loading} size="large" tooltip={t('login.checkUsername')}>
              {t('login.next')}
            </Button>
          </FormItem>
        </Form>
      {:else if step === 'password'}
        <Form onSubmit={handleLoginSubmit}>
          <FormItem error={passwordError}>
            <Input
              type="password"
              value={passwordValue}
              placeholder={t('login.enterPassword')}
              prefix="lock"
              size="large"
              onInput={onPasswordInput}
            />
          </FormItem>
          <FormItem>
            <Button type="primary" htmlType="submit" block loading={loading} size="large" tooltip={t('login.submitLogin')}>
              {t('login.submit')}
            </Button>
          </FormItem>
          <div style="text-align:center">
            <Button type="link" size="small" tooltip={t('login.backToUsernameTooltip')} onClick={backToUsername}>{t('login.backToUsername')}</Button>
          </div>
        </Form>
      {:else}
        <Form onSubmit={handleSetupSubmit}>
          <FormItem error={initialPasswordError}>
            <Input
              type="password"
              value={initialPassword}
              placeholder={t('login.initialPassword')}
              prefix="lock"
              size="large"
              onInput={onInitialPasswordInput}
            />
          </FormItem>
          <FormItem error={newPasswordError}>
            <Input
              type="password"
              value={newPassword}
              placeholder={t('login.newPassword')}
              prefix="lock"
              size="large"
              onInput={onNewPasswordInput}
            />
          </FormItem>
          <FormItem error={confirmNewPasswordError}>
            <Input
              type="password"
              value={confirmNewPassword}
              placeholder={t('login.confirmNewPassword')}
              prefix="lock"
              size="large"
              onInput={onConfirmNewPasswordInput}
            />
          </FormItem>
          <FormItem>
            <Button type="primary" htmlType="submit" block loading={loading} size="large" tooltip={t('login.setupLogin')}>
              {t('login.setupPassword')}
            </Button>
          </FormItem>
          <div style="text-align:center">
            <Button type="link" size="small" tooltip={t('login.backToUsernameTooltip')} onClick={backToUsername}>{t('login.backToUsername')}</Button>
          </div>
        </Form>
      {/if}
    {:else}
      <Form onSubmit={handleRegisterSubmit}>
        <FormItem error={regUsernameError}>
          <Input
            value={regUsername}
            placeholder={t('login.enterUsername')}
            prefix="user"
            size="large"
            onInput={onRegUsernameInput}
          />
        </FormItem>
        <FormItem error={regNameError}>
          <Input
            value={regName}
            placeholder={t('login.enterName')}
            prefix="idcard"
            size="large"
            onInput={onRegNameInput}
          />
        </FormItem>
        <FormItem error={regEmailError}>
          <Input
            value={regEmail}
            placeholder={t('login.emailOptional')}
            prefix="mail"
            size="large"
            onInput={onRegEmailInput}
          />
        </FormItem>
        <FormItem error={regPasswordError}>
          <Input
            type="password"
            value={regPassword}
            placeholder={t('login.password')}
            prefix="lock"
            size="large"
            onInput={onRegPasswordInput}
          />
        </FormItem>
        <FormItem error={regConfirmError}>
          <Input
            type="password"
            value={regConfirm}
            placeholder={t('login.confirmPassword')}
            prefix="lock"
            size="large"
            onInput={onRegConfirmInput}
          />
        </FormItem>
        <FormItem>
          <Button type="primary" htmlType="submit" block loading={loading} size="large" tooltip={t('login.registerTooltip')}>
            {t('login.register')}
          </Button>
        </FormItem>
      </Form>
    {/if}
  </Card>
</div>

<style>
  /* 校验态：输入框红框（antd form-item-has-error 视觉，组件库未内置故页面级补充） */
  :global(.ant-form-item-has-error .ant-input-affix-wrapper) {
    border-color: var(--ant-color-error);
  }
  :global(.ant-form-item-has-error .ant-input-affix-wrapper:hover) {
    border-color: var(--ant-color-error);
  }
</style>
