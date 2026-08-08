# Manner_Web 前端-SvelteKit-重构规范

> 本文是 SvelteKit 重构**完成之后**的编码规范（当前代码库已全面迁移到
> SvelteKit 5 + Svelte 5 + Vite 8 + TypeScript 5，纯 SPA，自研组件库），
> 不是"进行中"记录。所有规范均以 `frontend/src` 现有代码为基准，新代码必须遵守。
> 技术栈与目录结构总览见 [前端开发指南.md](./前端开发指南.md)。

## 0. 总则

- 技术基线：SvelteKit 5（`@sveltejs/kit ^2.70`）、Svelte 5（`svelte ^5.56`，runes 语法）、Vite 8（`vite ^8.2`）、TypeScript 5（`typescript ^5.6`，`strict: true`）、`@sveltejs/adapter-static ^3` 纯 SPA（`fallback: 'index.html'`，`+layout.ts` 中 `ssr=false`、`prerender=false`）。
- **无 React / antd 运行时依赖**：全部组件自研于 `src/lib/components/`，`package.json` 全部为 devDependencies，无任何运行时依赖。
- 唯一质量门禁：`npm run check`（`svelte-kit sync && svelte-check --tsconfig ./tsconfig.json`）必须零错误通过；发布前另执行 `npm run build`。

## 1. 组件开发规范

### 1.1 Svelte 5 语法

- 一律使用 Svelte 5 runes：`$props()`、`$state`、`$derived`、`$effect`、`$bindable`；**禁止**旧版 `export let` / `$:` / `<slot>`（改用 `{@render children()}` / Snippet）。
- 组件 `<script>` 一律 `lang="ts"`。
- 页面/组件内状态用 `$state` 声明，派生值用 `$derived`，副作用（watch 类）用 `$effect`。
- 根布局 `+layout.svelte` 中的 `children` 用 Snippet 接收：`let { children }: { children: Snippet } = $props()`，渲染用 `{@render children()}`。

### 1.2 自研组件原则

- 不引入任何第三方 UI 组件库；需要新控件时先在 `src/lib/components/` 自研。
- 组件命名 PascalCase（如 `Button.svelte`、`ConfirmModal.svelte`）。
- 组件只做展示与交互，不直接发请求；数据由页面通过 props 传入，交互结果通过回调 prop 上抛。
- 命令式能力（toast / 确认框）统一使用 `lib/components/message.ts` 的 `message.success/error/warning/info` 与 `lib/components/modal.ts` 的 `modal`，不各自造轮子。

### 1.3 Props / Events 约定

- Props 全部在 `$props()` 中声明并给出默认值，命名沿用 antd 语义（`type`、`size`、`disabled`、`placeholder` 等）。
- 事件不声明事件名（无 `createEventDispatcher`），直接传回调 prop，统一命名为 `onXxx`（如 `onClick`、`onChange`、`onConfirm`），与现有组件（`Menu`、`Dropdown`、`Button` 等）一致。
- 插槽用 Snippet prop 接收：`children` 为默认内容，具名内容用 `header`、`footer` 等 Snippet。
- 双向绑定型 prop 需要可写时用 `$bindable()`。

### 1.4 样式 class 命名沿用 antd 体系

- 组件根元素使用 antd 约定类名：`ant-btn`、`ant-input`、`ant-table`、`ant-modal`、`ant-menu`、`ant-layout-*` 等，使 `src/styles/global.css` 的覆盖规则与自研组件天然一致。
- 颜色、圆角、阴影、字号一律用设计令牌（`var(--ant-color-primary)`、`var(--ant-border-radius)` 等），禁止写死色值。
- 组件内 `<style>` 只放布局性/结构性样式，主题相关样式放到 `global.css` 的令牌层。

## 2. 页面开发规范

### 2.1 路由组织

- 路由文件：`src/routes/<path>/+page.svelte`，动态参数用 `[id]` 目录（如 `employees/[id]/edit`）。当前共 9 个页面路由（`/`、`/login`、`/chat`、`/employees`、`/employees/new`、`/employees/[id]/edit`、`/logs`、`/profile`、`/settings`），清单见《前端开发指南.md》。
- 纯 SPA：不写 `+server.ts` / `+page.ts` 数据加载（`+layout.ts` 固定 `ssr=false`、`prerender=false`），数据在组件 `onMount` 中拉取。
- 页面目录只放该页私有组件；可复用组件放 `src/lib/components/`。
- 受权限控制的页面/菜单项：在 `src/lib/components/Layout.svelte` 的 `menuItems` 中按权限码控制（如 `authStore.hasPermission('employee:list')`、`authStore.hasPermission('system:settings')`）。

### 2.2 数据加载

- 在 `onMount` 中调用 `lib/api` 模块函数加载数据，用 `$state` 保存：

  ```svelte
  <script lang="ts">
    import { onMount } from 'svelte'
    import { getEmployees } from '@/api/employees'
    import type { Employee } from '@/types'

    let loading = $state(false)
    let items = $state<Employee[]>([])

    onMount(async () => {
      loading = true
      try {
        const res = await getEmployees({ page: 1, page_size: 20 })
        if (res.code === 0) items = res.data.items
      } finally {
        loading = false
      }
    })
  </script>
  ```

- 列表加载失败用 `message.error` 提示，并渲染 `Empty` / 重试入口。
- 加载状态用 `Spin` 展示，空态用 `Empty` 组件。
- 页面卸载需清理的定时器/订阅在 `onDestroy` 中处理。

### 2.3 错误处理

- 业务失败（`res.code !== 0`）直接用 `res.message` 提示；网络失败（`catch` 分支）用 `lib/api/client.ts` 的 `getApiError(err, fallback)` / `extractApiError(err, fallback)` 提取后端 `message`，兜底文案必须给出，均以 `message.error` 提示。
- 401 场景由 `client` 自动处理（刷新重试/登出跳转），页面无需关心，除非有特殊 UI 需求。

## 3. API 调用规范

- **统一走 `src/lib/api/`，禁止在页面/组件里散落 `fetch`**（`client.ts` 已封装前缀、Cookie、超时、401 刷新）。
- 每个业务域一个模块文件（`auth.ts` / `chat.ts` / `employees.ts` / `system.ts`），导出类型化的 `async function`，内部调用 `client.get/post/put/delete/upload`。
- 返回类型使用 `ApiResponse<T>`；列表返回 `PaginatedResponse<T>`（`ApiResponse<PaginatedData<T>>`，`data` 为 `{ items, total, page, page_size }`）。
- 上传文件：`client.upload(url, formData)`；图片走 `/upload`，任意文件走 `/upload/file`（`uploadImage` / `uploadChatFile` 已封装，返回 `/uploads/...` 相对路径字符串）。
- 不通过 URL 传敏感信息；参数放 `body` 或 `params`（`client` 会自动忽略空值）。

## 4. 状态管理规范

- 全局状态用 Svelte store（`svelte/store` 的 `writable`），集中在 `src/lib/stores/`。
- **authStore**（`stores/auth.ts`）：唯一的认证态来源；页面不要自行维护"是否登录"布尔值，读取 `$authStore.isAuthenticated` / `user` / `permissions`；权限判断用 `authStore.hasPermission(code)`。
- **preferencesStore**（`stores/preferences.ts`）：主题/时区等个人偏好；本地（`manner-preferences`）与后端（`/api/auth/preferences`）双写，页面修改偏好调用其 `updateTheme` / `updateTimezoneMode` / `updateTimezoneOffset` / `updateNewConvPosition` 等方法，不直接写 localStorage。
- 页面级临时状态用 `$state`，不进 store；跨页面共享的会话/偏好才进 store。
- 非组件模块需要读取偏好时用 `getGlobalPrefs()` / `subscribe()`。
- 导航用 `goto`（`$app/navigation`），读取路由参数用 `$page`（`$app/stores`），不直接操作 `window.location`（登出跳转等特殊场景除外）。

## 5. 类型规范

- 所有与后端契约对齐的类型集中在 `src/lib/types/index.ts`（`User`、`Employee`、`Permission`、`PermissionModule`、`ApiResponse<T>`、`PaginatedData<T>` / `PaginatedResponse<T>`、请求体类型等），以 `backend/src/models` 源码为准。
- 页面/组件内部派生类型就地定义或放入 `types/index.ts`，不散落重复定义。
- TS 配置为 `strict: true`；新代码不得出现 `any`（确有必要时用 `unknown` + 收窄）。
- 组件 props 必须显式标注类型（`$props<{ ... }>()` 或内联类型），禁止隐式 `any`。

## 6. 代码质量

- **`npm run check` 必须通过**（svelte-check 会做组件类型、模板绑定、store 类型全量校验），这是提交/合并的前置门禁。
- 提交前执行：

  ```bash
  cd frontend
  npm run check
  npm run build
  ```

- 不要提交 `.svelte-kit/`、`build/`、`node_modules/`。
- 组件/页面改动后本地 `npm run dev` 联调（后端需运行在 `localhost:8080`），确认无控制台报错、无未处理的 Promise rejection。
