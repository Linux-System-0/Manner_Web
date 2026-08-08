<script lang="ts">
  // 全局根布局：仅负责
  // 1) 会话恢复（/auth/me，httpOnly Cookie 认证）与登录态分发
  // 2) 偏好初始化与主题应用
  // 3) 登录页裸渲染 vs 受保护页 Layout 包裹
  // 视觉骨架（Sider/Header/Content/菜单）见 src/lib/components/Layout.svelte
  import type { Snippet } from 'svelte'
  import '../styles/global.css'
  import { onMount } from 'svelte'
  import { page } from '$app/stores'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { preferencesStore } from '$lib/stores/preferences'
  import { getMe } from '$lib/api/auth'
  import Layout from '$lib/components/Layout.svelte'

  let { children }: { children: Snippet } = $props()

  // Svelte 5 runes：必须用 $state 声明，否则 onMount 中赋值不会触发重渲染，
  // 页面将永远停留在 loading 分支（白屏）。
  let initialized = $state(false)
  let isLoginPage = $derived($page.url.pathname.startsWith('/login'))

  onMount(async () => {
    authStore.restoreLocal()
    await preferencesStore.initialize()

    if (isLoginPage) {
      // 登录页自身管理显示逻辑；已登录访问 /login 时由登录页决定跳转
      initialized = true
      return
    }

    try {
      const res = await getMe()
      authStore.setUser(res.data)
    } catch {
      // 无有效会话 → 回登录页（replace 避免历史栈污染）
      goto('/login', { replaceState: true })
    } finally {
      initialized = true
    }
  })
</script>

{#if !initialized}
  <div class="root-loading">
    <span class="ant-spin ant-spin-lg">
      <span class="ant-spin-dot ant-spin-dot-spin">
        <i class="ant-spin-dot-item"></i>
        <i class="ant-spin-dot-item"></i>
        <i class="ant-spin-dot-item"></i>
        <i class="ant-spin-dot-item"></i>
      </span>
    </span>
  </div>
{:else if isLoginPage}
  {@render children()}
{:else}
  <Layout>
    {@render children()}
  </Layout>
{/if}

<style>
  .root-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    background: var(--ant-color-bg-layout);
  }
</style>
