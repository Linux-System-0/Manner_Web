<script lang="ts">
  // 主布局：Sider + Header + Content（复刻原 React Layout.tsx）
  // - Sider 宽度按站点标题自适应（clamp 200~360），可折叠
  // - 菜单按权限过滤；用户下拉：个人资料/个人设置/退出登录
  // - 主题：读取 manner-preferences + 系统偏好 → html[data-theme]
  import type { Snippet } from 'svelte'
  import { onMount } from 'svelte'
  import { page } from '$app/stores'
  import { goto } from '$app/navigation'
  import { Icon } from '$lib/icons'
  import Menu from './Menu.svelte'
  import Dropdown from './Dropdown.svelte'
  import Button from './Button.svelte'
  import Avatar from './Avatar.svelte'
  import PreferencesModal from './PreferencesModal.svelte'
  import { authStore } from '$lib/stores/auth'
  import { getLoginPage } from '$lib/api/system'
  import { getEffectiveTheme, subscribe as subscribePrefs } from '$lib/stores/preferences'
  import { logout as logoutApi } from '$lib/api/auth'
  import { setFavicon } from '$lib/utils/favicon'

  let { children }: { children: Snippet } = $props()

  let collapsed = $state(false)
  let prefOpen = $state(false)
  let themeMode = $state<'light' | 'dark'>('light')
  let siteTitle = $state('企业管理系统')
  let siderWidth = $state(200)
  let titleEl: HTMLDivElement | undefined = $state()

  function syncTheme() {
    const raw = localStorage.getItem('manner-preferences')
    if (raw) {
      try {
        themeMode = getEffectiveTheme((JSON.parse(raw).theme as 'light' | 'dark' | 'system') || 'system')
        return
      } catch {
        /* ignore */
      }
    }
    themeMode = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }

  onMount(() => {
    syncTheme()
    const unsub = subscribePrefs(syncTheme)
    const mq = window.matchMedia('(prefers-color-scheme: dark)')
    mq.addEventListener('change', syncTheme)

    // F-24: 站点标题改从匿名登录页配置接口获取，避免无 system:settings 权限的
    // 用户（如 test2）在加载任意页面时请求受保护的系统设置接口。
    getLoginPage()
      .then((res) => {
        const t = res.data?.site_title
        if (t) {
          siteTitle = t
          document.title = t
        }
        setFavicon(res.data?.site_icon ? `/api/system/icon/site?v=${Date.now()}` : null)
      })
      .catch(() => {})

    return () => {
      unsub()
      mq.removeEventListener('change', syncTheme)
    }
  })

  $effect(() => {
    document.documentElement.setAttribute('data-theme', themeMode)
  })

  $effect(() => {
    if (titleEl) {
      const w = titleEl.scrollWidth + 48
      siderWidth = Math.min(Math.max(w, 200), 360)
    }
  })

  async function handleLogout() {
    try {
      await logoutApi()
    } catch {
      /* ignore */
    }
    authStore.logout()
    goto('/login', { replaceState: true })
  }

  let selectedKey = $derived('/' + $page.url.pathname.split('/').filter(Boolean)[0])

  let menuItems = $derived([
    { key: '/', label: '仪表盘', icon: 'dashboard' },
    ...(authStore.hasPermission('employee:list')
      ? [{ key: '/employees', label: '员工管理', icon: 'team' }]
      : []),
    ...(authStore.hasPermission('department:list')
      ? [{ key: '/departments', label: '部门管理', icon: 'idcard' }]
      : []),
    ...(authStore.hasPermission('role:manage')
      ? [{ key: '/roles', label: '角色管理', icon: 'lock' }]
      : []),
    { key: '/chat', label: '聊天', icon: 'message' },
    ...(authStore.hasPermission('system:settings')
      ? [{ key: '/logs', label: '应用日志', icon: 'file-text' }]
      : []),
    ...(authStore.hasPermission('system:settings')
      ? [{ key: '/settings', label: '系统设置', icon: 'setting' }]
      : []),
  ])

  let userMenuItems = [
    { key: 'profile', label: '个人资料', icon: 'user' },
    { key: 'divider1', label: '', divider: true },
    { key: 'preferences', label: '个人设置', icon: 'setting' },
    { key: 'divider2', label: '', divider: true },
    { key: 'logout', label: '退出登录', icon: 'logout', danger: true },
  ]

  function onUserMenu(key: string) {
    if (key === 'profile') goto('/profile')
    else if (key === 'preferences') prefOpen = true
    else if (key === 'logout') handleLogout()
  }
</script>

<div class="ant-layout" style="min-height:100vh;display:flex;flex-direction:row" data-theme={themeMode}>
  <aside
    class="ant-layout-sider"
    class:ant-layout-sider-dark={themeMode === 'dark'}
    style="width:{collapsed ? 80 : siderWidth}px;min-width:{collapsed ? 80 : siderWidth}px;max-width:{collapsed ? 80 : siderWidth}px;background:var(--ant-layout-sider-bg);transition:all 0.2s;overflow:auto"
  >
    <div
      bind:this={titleEl}
      style="height:64px;display:flex;align-items:center;justify-content:center;color:var(--ant-layout-sider-color);font-size:{collapsed ? 16 : 20}px;font-weight:700;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;padding:0 16px"
    >
      {collapsed ? '' : siteTitle}
    </div>
    <Menu
      items={menuItems}
      theme={themeMode === 'dark' ? 'dark' : 'light'}
      selectedKeys={[selectedKey]}
      onClick={(key) => goto(key)}
      collapsed={collapsed}
      style="border-right:none"
    />
  </aside>
  <div class="ant-layout" style="flex:1;display:flex;flex-direction:column;min-width:0">
    <header
      class="ant-layout-header"
      style="padding:0 24px;background:var(--ant-layout-header-bg);display:flex;align-items:center;justify-content:space-between;box-shadow:0 1px 4px rgba(0,0,0,0.08);height:64px;line-height:64px"
    >
      <Button type="text" tooltip={collapsed ? '展开侧边菜单' : '收起侧边菜单'} tooltipPosition="right" onClick={() => (collapsed = !collapsed)}>
        <span style="display:inline-flex"><Icon name={collapsed ? 'menu-unfold' : 'menu-fold'} style="font-size:16px" /></span>
      </Button>
      <Dropdown items={userMenuItems} onClick={onUserMenu} placement="bottomRight">
        <span style="cursor:pointer;display:inline-flex;align-items:center;gap:8px">
          <Avatar src={$authStore.user?.avatar} size={32}>
            {#if !$authStore.user?.avatar}<span style="display:inline-flex"><Icon name="user" style="font-size:16px" /></span>{/if}
          </Avatar>
          <span style="color:var(--ant-layout-header-color)">{$authStore.user?.name || $authStore.user?.username}</span>
        </span>
      </Dropdown>
    </header>
    <main
      class="ant-layout-content"
      style="margin:24px;padding:24px;background:var(--ant-layout-content-bg);border-radius:var(--ant-border-radius-lg);height:calc(100vh - 112px);overflow:hidden;display:flex;flex-direction:column"
    >
      {@render children()}
    </main>
  </div>
</div>

<PreferencesModal open={prefOpen} onClose={() => (prefOpen = false)} />
