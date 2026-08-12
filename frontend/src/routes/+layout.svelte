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
  // 全局根布局：仅负责
  // 1) 会话恢复（/auth/me，httpOnly Cookie 认证）与登录态分发
  // 2) 偏好初始化与主题应用
  // 3) 登录页裸渲染 vs 受保护页 Layout 包裹
  // 视觉骨架（Sider/Header/Content/菜单）见 src/lib/components/Layout.svelte
  import type { Snippet } from 'svelte'
  import '../styles/global.css'
  import { onMount, onDestroy } from 'svelte'
  import { page } from '$app/stores'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { preferencesStore } from '$lib/stores/preferences'
  import { getMe } from '$lib/api/auth'
  import { initI18n } from '$lib/i18n/init'
  import Layout from '$lib/components/Layout.svelte'

  let { children }: { children: Snippet } = $props()

  // Svelte 5 runes：必须用 $state 声明，否则 onMount 中赋值不会触发重渲染，
  // 页面将永远停留在 loading 分支（白屏）。
  let initialized = $state(false)
  let isLoginPage = $derived($page.url.pathname.startsWith('/login'))

  let meTimer: ReturnType<typeof setInterval> | null = null
  let lastUserId: string | null = null

  // 会话身份对账：同浏览器（共享 cookie）登录第二个账号时，本页的 cookie
  // 会被覆盖为「新账号」，但前端 authStore 仍停留在旧账号 → 出现「界面显示 A、
  // 实际以 B 身份操作」的错乱。周期性以服务端 /auth/me 为准同步 authStore。
  async function reconcileSession() {
    try {
      const res = await getMe()
      if (res.code !== 0 || !res.data) return
      if (lastUserId !== null && res.data.id !== lastUserId) {
        // 身份已切换（同浏览器被另一账号覆盖）：更新本地登录态与偏好
        authStore.setUser(res.data)
        await preferencesStore.refresh()
      } else if (lastUserId === null) {
        authStore.setUser(res.data)
      }
      lastUserId = res.data.id
    } catch {
      // 401 时 client 已处理续期/登出跳转，此处无需处理
    }
  }

  onMount(async () => {
    authStore.restoreLocal()
    await preferencesStore.initialize()
    await initI18n()

    if (isLoginPage) {
      // 登录页自身管理显示逻辑；已登录访问 /login 时由登录页决定跳转
      initialized = true
      return
    }

    try {
      const res = await getMe()
      authStore.setUser(res.data)
      lastUserId = res.data.id
    } catch {
      // 无有效会话 → 回登录页（replace 避免历史栈污染）
      goto('/login', { replaceState: true })
    } finally {
      initialized = true
    }

    meTimer = setInterval(() => void reconcileSession(), 30000)
  })

  onDestroy(() => {
    if (meTimer) clearInterval(meTimer)
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
