import adapter from '@sveltejs/adapter-static'
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte'

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    // 纯静态 SPA：构建产物由 Nginx 托管（try_files 回退到 index.html），
    // /api 与 /uploads 同源反代到后端（见 vite.config.ts 与 docs/nginx-prod.conf.example）。
    adapter: adapter({
      fallback: 'index.html',
    }),
    alias: {
      '@': './src/lib',
      '$lib': './src/lib',
    },
  },
}

export default config
