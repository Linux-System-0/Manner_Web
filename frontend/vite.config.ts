// Manner_Web - 可以在 Linux 系统上运行的企业管理系统
// Copyright (C) 2026 Linux-System-0(Github) / 一架在Linux上起飞的A320(Bilibili) <ls0_1@qq.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

import { fileURLToPath } from 'node:url'
import { createLogger, defineConfig } from 'vite'
import { sveltekit } from '@sveltejs/kit/vite'
import type { Connect, Logger, Plugin } from 'vite'

/**
 * dev server 安全加固，针对以下暴露面（自 React/Vite 版迁移，逻辑与注释保持一致）：
 * 1. 源码/配置匿名可读（/src/*.svelte、vite.config.ts、tsconfig.json、package.json 等）
 *    统一由 server.fs.deny + 中间件兜底返回通用 403；
 * 2. 强制仅绑定回环地址——任何 --host/HOST/server.host 试图绑定到非回环
 *    接口时直接拒绝启动，杜绝公网暴露后升级为高危；
 * 3. 剥离内联 sourcemap、折叠响应体与 HMR 错误浮层中的服务器绝对路径
 *    （保留文件名与行号，补丁本身不携带任何服务器路径）；
 * 4. 将 SvelteKit dev 注入的 /@fs/<服务器绝对路径>/... 客户端入口改写为项目根
 *    相对路径，消除浏览器端（Sources/Network）可见的绝对路径泄露。
 */

/** 允许绑定的回环主机名/IP */
const LOOPBACK_HOSTS = new Set(['127.0.0.1', 'localhost', '::1'])

/** 折叠绝对路径：/home/.../src/lib/App.svelte:5:3 -> .../App.svelte:5:3 */
const ABSOLUTE_PATH_RE = /((?:\/[^\s:]+){3,}\/[^\s:]*)/g
const collapsePaths = (text: string): string =>
  text.replace(ABSOLUTE_PATH_RE, (m) => `...${m.split('/').filter(Boolean).slice(-2).join('/')}`)

/** 本项目根目录绝对路径（仅服务端用于响应体清洗，不写入任何下发内容） */
const ROOT_PATH = fileURLToPath(new URL('.', import.meta.url)).replace(/\/$/, '').replace(/\\/g, '/')
const ESCAPED_ROOT = ROOT_PATH.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
const ROOT_PATH_RE = new RegExp(`(?<!@fs)${ESCAPED_ROOT}`, 'g')

/**
 * 匹配 Vite/SvelteKit dev 的 @fs 客户端入口 URL（如
 * /@fs/home/ubuntu/Project/Manner_Web/frontend/.svelte-kit/generated/client/app.js）。
 * SvelteKit 用 to_fs() 硬编码该入口，携带服务器绝对路径；统一改写为项目根相对路径
 * /<rest>（根内文件本就由 Vite 以根相对 URL 提供），使浏览器端不再出现绝对路径。
 */
const FS_ROOT_RE = new RegExp(`/@fs${ESCAPED_ROOT}/`, 'g')

/**
 * 强制回环绑定：CLI 的 --host、环境变量 HOST 会覆盖配置文件中的 server.host，
 * 故在 configResolved 阶段按最终解析结果校验，非回环即抛错拒绝启动。
 */
function refusePublicBind(): Plugin {
  return {
    name: 'refuse-public-bind',
    configResolved(config) {
      const host = config.server.host
      const resolved =
        host === false || host === undefined || host === null
          ? 'localhost'
          : host === true
            ? '0.0.0.0'
            : String(host)
      if (!LOOPBACK_HOSTS.has(resolved)) {
        throw new Error(
          `[security] dev server 仅允许绑定回环地址，检测到 host="${resolved}"，已拒绝启动。` +
            '请勿使用 --host / HOST / server.host 将开发服务器暴露到公网。',
        )
      }
    },
  }
}

/**
 * 响应面收敛：
 * 1. 越权/穿越类请求（@fs、路径穿越、反斜杠、?raw）与敏感文件名
 *    （vite.config.*、tsconfig*、package*.json、*.log、*.tsbuildinfo、.env*）
 *    直接返回通用 403，避免 Vite 的 403 页面泄露 server.fs.allow 中的项目根绝对路径。
 * 2. 剥离模块响应末尾的内联 sourcemap（其中 sources 数组嵌入 /home/... 绝对路径），
 *    并清洗响应体中出现的项目根绝对路径（Svelte HMR 元数据同样携带绝对路径）。
 * 3. 非代理路径的错误响应体（>=400）折叠其中出现的绝对路径。
 * 注意：sourcemap 由 Vite 核心在插件 transform 之后追加，故须在最外层中间件改写响应体。
 */
function hideServerPaths(): Plugin {
  const JS_SOURCEMAP_RE = /[\r\n]+\/\/# sourceMappingURL=data:[^\r\n]*/g
  const CSS_SOURCEMAP_RE = /[\r\n]+\/\*# sourceMappingURL=data:[^\r\n]*\*\/[\r\n]*/g
  // 不拦截 /@fs/ 本身（Vite dev 的必需加载机制），而是把其中携带的项目根绝对路径
  // 改写为根相对 URL（见 FS_ROOT_RE），访问边界已由 server.fs.strict + fs.deny 收敛；
  // 保留穿越与 ?raw 拦截。
  const SUSPICIOUS_RE = /(\.\.|\\|%5c|\?raw)/i
  const SENSITIVE_FILE_RE = /^\/(?:\.env(?:\.\w+)?|vite\.config\.|vitest\.config\.|svelte\.config\.|tsconfig|[^/]*\.(?:log|tsbuildinfo)$|package(?:-lock)?\.json$)/i

  const scrub = (chunk: unknown): unknown => {
    if (typeof chunk === 'string') {
      return chunk
        .replace(JS_SOURCEMAP_RE, '')
        .replace(CSS_SOURCEMAP_RE, '')
        .replace(FS_ROOT_RE, '/')
        .replace(ROOT_PATH_RE, '<project-root>')
    }
    if (chunk instanceof Uint8Array) {
      const text = Buffer.from(chunk).toString('utf8')
      const cleaned = text
        .replace(JS_SOURCEMAP_RE, '')
        .replace(CSS_SOURCEMAP_RE, '')
        .replace(FS_ROOT_RE, '/')
        .replace(ROOT_PATH_RE, '<project-root>')
      return cleaned !== text ? Buffer.from(cleaned) : chunk
    }
    return chunk
  }

  const scrubError = (chunk: unknown): unknown => {
    if (typeof chunk === 'string') {
      return collapsePaths(chunk.replace(FS_ROOT_RE, '/').replace(ROOT_PATH_RE, '<project-root>'))
    }
    if (chunk instanceof Uint8Array) {
      const text = Buffer.from(chunk).toString('utf8')
      const cleaned = collapsePaths(text.replace(FS_ROOT_RE, '/').replace(ROOT_PATH_RE, '<project-root>'))
      return cleaned !== text ? Buffer.from(cleaned) : chunk
    }
    return chunk
  }

  return {
    name: 'hide-server-paths',
    configureServer(server) {
      server.middlewares.use((req: Connect.IncomingMessage, res, next) => {
        const rawUrl = req.url || ''
        // 先将 /@fs/<服务器绝对路径>/ 入口改写为项目根相对路径，保证后续 Vite
        // 中间件按改写后的 URL 提供模块，浏览器端不再出现绝对路径。
        if (FS_ROOT_RE.test(rawUrl)) {
          const rewritten = rawUrl.replace(FS_ROOT_RE, '/')
          req.url = rewritten
        }
        let decoded = req.url || ''
        try {
          decoded = decodeURIComponent(decoded)
        } catch {
          /* malformed percent-encoding: use raw url */
        }
        const pathname = decoded.split('?')[0]
        if (SUSPICIOUS_RE.test(decoded) || SENSITIVE_FILE_RE.test(pathname)) {
          res.statusCode = 403
          res.setHeader('Content-Type', 'text/plain; charset=utf-8')
          res.end('Forbidden')
          return
        }

        // 代理路径的原样透传，仅剥离 sourcemap；不折叠错误体以免破坏后端响应
        const isProxyPath = pathname === '/api' || pathname.startsWith('/api/') || pathname === '/uploads' || pathname.startsWith('/uploads/')

        const write = res.write.bind(res)
        const end = res.end.bind(res)
        res.write = (chunk: unknown, ...args: unknown[]) => {
          let out = scrub(chunk)
          if (!isProxyPath && res.statusCode >= 400 && !res.headersSent) {
            out = scrubError(out)
          }
          return write(out, ...(args as []))
        }
        res.end = (chunk?: unknown, ...args: unknown[]) => {
          if (chunk !== undefined && chunk !== null) {
            const originalLength =
              typeof chunk === 'string' ? Buffer.byteLength(chunk) : chunk instanceof Uint8Array ? chunk.length : 0
            chunk = scrub(chunk)
            if (!isProxyPath && res.statusCode >= 400) {
              chunk = scrubError(chunk)
            }
            // 改写响应体后同步修正 content-length，否则浏览器会等待与 body 不符的
            // 剩余字节导致页面长时间挂起（白屏）。
            if (!res.headersSent && originalLength > 0) {
              const newLength =
                typeof chunk === 'string' ? Buffer.byteLength(chunk) : chunk instanceof Uint8Array ? chunk.length : 0
              if (newLength !== originalLength && res.getHeader('content-length') !== undefined) {
                res.setHeader('content-length', String(newLength))
              }
            }
          }
          return end(chunk, ...(args as []))
        }
        next()
      })
    },
  }
}

/**
 * 客户端侧 HMR 错误浮层（vite-error-overlay）会把 err.loc.file / err.id / stack
 * 中携带的服务器绝对路径展示给浏览器。在 @vite/client 末尾注入一段通用补丁，
 * 折叠其中的绝对路径（保留文件名与行号）。补丁只含正则，不含任何服务器路径。
 */
const OVERLAY_PATH_PATCH = `
;(() => {
  const Ctor = customElements.get('vite-error-overlay');
  if (Ctor && Ctor.prototype) {
    const re = ${ABSOLUTE_PATH_RE};
    const orig = Ctor.prototype.text;
    Ctor.prototype.text = function () {
      if (arguments.length > 1 && typeof arguments[1] === 'string') {
        arguments[1] = arguments[1].replace(re, (m, p) =>
          p ? '...' + p.split('/').filter(Boolean).slice(-2).join('/') : m,
        );
      }
      return orig.apply(this, arguments);
    };
  }
})();
`

function scrubOverlayPaths(): Plugin {
  return {
    name: 'scrub-overlay-paths',
    enforce: 'post',
    transform(code, id) {
      if (id.replace(/\\/g, '/').includes('/node_modules/vite/dist/client/client.mjs')) {
        return code + OVERLAY_PATH_PATCH
      }
      return null
    },
  }
}

/**
 * 安全响应头（dev server）：HTML/静态响应补充 CSP、nosniff、X-Frame-Options、
 * Referrer-Policy，收敛 F-06 指出的前端响应头缺失。CSP 需兼容 Vite HMR
 * （内联脚本、eval 与 WebSocket 连接），生产环境由 Nginx 下发收敛后的 CSP。
 */
function securityHeaders(): Plugin {
  return {
    name: 'security-headers',
    configureServer(server) {
      server.middlewares.use((_req, res, next) => {
        res.setHeader('X-Content-Type-Options', 'nosniff')
        res.setHeader('X-Frame-Options', 'DENY')
        res.setHeader('Referrer-Policy', 'no-referrer')
        res.setHeader(
          'Content-Security-Policy',
          "default-src 'self'; " +
            "script-src 'self' 'unsafe-inline' 'unsafe-eval'; " +
            "style-src 'self' 'unsafe-inline'; " +
            "img-src 'self' data: blob:; font-src 'self' data:; " +
            "connect-src 'self' ws: wss: http://localhost:8080 http://127.0.0.1:8080; " +
            "frame-ancestors 'none'; base-uri 'self'; form-action 'self'",
        )
        next()
      })
    },
  }
}

export default defineConfig({
  plugins: [sveltekit(), refusePublicBind(), hideServerPaths(), scrubOverlayPaths(), securityHeaders()],
  customLogger: (() => {
    const logger: Logger = createLogger()
    const origWarn = logger.warn.bind(logger)
    logger.warn = (msg, options) => {
      const text = typeof msg === 'string' ? msg : Array.isArray(msg) ? (msg as Array<string | Error>).map(String).join(' ') : String(msg)
      if (text.includes('outside of Vite serving allow list')) {
        return
      }
      return origWarn(msg, options)
    }
    return logger
  })(),
  build: {
    sourcemap: false,
  },
  server: {
    host: '127.0.0.1',
    strictPort: true,
    fs: {
      strict: true,
      deny: [
        '.env',
        '.env.*',
        '*.{crt,pem}',
        '**/.git/**',
        'vite.config.*',
        'vitest.config.*',
        'svelte.config.*',
        'tsconfig*.json',
        '*.tsbuildinfo',
        '*.log',
        'package.json',
        'package-lock.json',
        'pnpm-lock.yaml',
        'yarn.lock',
        'Cargo.toml',
        'Cargo.lock',
      ],
    },
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
      '/uploads': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
})
