// 纯 SPA 模式：关闭 SSR 与预渲染，所有路由由 adapter-static 的 fallback(index.html) 提供，
// 与现有 React 单页应用行为一致（Nginx try_files 回退）。
export const ssr = false
export const prerender = false
