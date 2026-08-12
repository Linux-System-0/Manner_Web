# Manner_Web

> 内部企业管理系统：员工管理 + 角色权限（RBAC + 数据范围 + 部门角色继承） + 组织架构（部门树） + 站内聊天 + 系统设置。

Manner_Web 是面向企业内部的 Web 管理系统，将员工管理、权限控制、组织架构、站内聊天与系统设置整合到同一平台。采用前后端分离架构：后端为 Rust（axum）REST API，前端为 SvelteKit 5 构建的纯 SPA 静态站点。权限采用 **RBAC + 数据范围 + 部门角色继承** 模型：权限经角色授予（员工直接分配 + 部门绑定 + 父子角色继承），并支持数据范围（全部/本部门及子部门/本部门/仅本人/指定部门）。

## 技术栈

| 层级 | 技术 | 说明 |
| --- | --- | --- |
| 后端 | Rust（axum 0.7 + sqlx 0.8） | REST API；bcrypt 密码哈希、JWT（HS256）双令牌认证；员工敏感字段 AES-256-GCM 静态加密；MySQL 异步访问（运行时 SQL） |
| 前端 | SvelteKit 5 + Svelte 5 + Vite 8 + TypeScript 5 | 纯 SPA（adapter-static，`ssr=false`）；UI 组件全部自研（40+ 个），无 React/antd 运行时依赖；内置 i18n（中/英） |
| 数据库 | MySQL 8 | utf8mb4 / InnoDB；16 张表；`backend/sql/init.sql` 启动时幂等执行建表 |
| 部署 | Nginx + 后端二进制 + MySQL | 静态产物托管 + `/api`、`/uploads` 反向代理；运行期无需 Node.js |

## 目录结构

```text
Manner_Web/
├── backend/          # Rust 后端服务
│   ├── src/          #   源码（handlers / middleware / models / services / utils）
│   ├── sql/          #   数据库脚本（init.sql 建表与种子数据）
│   ├── uploads/      #   上传文件存储目录（运行期生成，不入库）
│   ├── logs/         #   运行日志目录（运行期生成，不入库）
│   └── .env.example  #   环境变量模板（复制为 .env 使用）
├── frontend/         # SvelteKit 5 前端
│   ├── src/routes/   #   页面路由（login / chat / employees / departments / roles / logs / profile / settings 等）
│   ├── src/lib/      #   api / stores / i18n / components / types / icons / styles
│   ├── svelte.config.js  # adapter-static（fallback index.html）
│   └── vite.config.ts    # 开发代理 /api、/uploads → 后端
├── sql/              # 数据库脚本目录（权威脚本为 backend/sql/init.sql）
└── docs/             # 项目文档（入口：docs/README.md）
```

## 环境配置

后端读取 `backend/.env` 配置文件，参照 `backend/.env.example` 设置以下变量：

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `DATABASE_URL` | 必填，无默认 | MySQL 连接串，如 `mysql://user:pass@host:3306/manner_web`；示例值拒绝启动 |
| `JWT_SECRET` | 无默认（公开默认值拒绝启动） | JWT 签名密钥，建议 ≥64 字符强随机 |
| `FIELD_ENC_KEY` | 必填，无默认（示例值拒绝启动） | 员工敏感字段静态加密密钥，SHA-256 派生为 32 字节 AES-256-GCM 密钥；建议 ≥32 字符强随机 |
| `BCRYPT_COST` | 12 | bcrypt 哈希成本 |
| `TOKEN_EXPIRE_MINUTES` | 30 | access 令牌有效期（分钟） |
| `REFRESH_TOKEN_EXPIRE_DAYS` | 7 | refresh 令牌有效期（天） |
| `LOG_LEVEL` | debug | 日志级别（`RUST_LOG` 优先） |
| `SERVER_HOST` | 127.0.0.1 | 监听地址（绑定 0.0.0.0 会告警） |
| `SERVER_PORT` | 8080 | 监听端口 |
| `UPLOAD_DIR` | ./uploads | 上传文件目录 |
| `LOG_FILE` | ./logs/manner.log | 业务日志文件（日志 API 读取展示） |
| `CORS_ALLOWED_ORIGINS` | http://localhost:5173,http://127.0.0.1:5173 | 允许的前端来源，逗号分隔 |
| `LOGIN_MAX_FAILURES` | 5 | 登录限流阈值（系统设置可覆盖） |
| `LOGIN_LOCK_WINDOW_SECS` | 900 | 限流窗口秒数（系统设置可覆盖） |
| `COOKIE_SECURE` | false | Cookie `Secure` 属性；生产环境必须置 `true` |
| `TRUSTED_PROXIES` | 空 | 可信反向代理 IP（单 IP 或 CIDR，逗号分隔）。经 Nginx 反代部署时填代理地址（如 `127.0.0.1`），后端才信任 `X-Real-IP` 解析真实客户端 IP；直连部署留空即忽略一切转发头 |
| `PROFILE` | — | 设为 `production` 时启用 JSON 滚动日志 |

> **注意**：`backend/.env` 已被 `.gitignore` 忽略，严禁提交到 git；生产环境建议改用环境变量注入。

## 安全要点

- **凭据治理**：`backend/.env` 不入库（`.gitignore` 已忽略）；生产环境用环境变量注入（systemd `Environment=` / Docker `-e` / K8s Secret）；`JWT_SECRET` 与 `FIELD_ENC_KEY` 必须为强随机值（示例值拒绝启动），生产环境 `COOKIE_SECURE=true`。
- **敏感字段加密**：员工邮箱/电话/身份证/地址以 AES-256-GCM 静态加密落库（`enc:v1:` 前缀），普通接口返回掩码，仅 `employee:view_sensitive` 权限可解密查看（强制审计日志）。
- **反代部署**：经 Nginx 反代时必须设置 `TRUSTED_PROXIES=127.0.0.1`，后端才按真实来源 IP 做登录限流与审计（否则失败计数全站共享，5 次失败可能锁死全站登录）。
- **密钥轮换**：任何密钥（`JWT_SECRET`、`FIELD_ENC_KEY`、SSH 私钥等）疑似泄露时，立即轮换，并重写 git 历史清除残留（如 `git filter-repo`）。
- 详细安全基线见 [加密标准.md](docs/加密标准.md) 与 [安全设计说明.md](docs/安全设计说明.md)。

## 本地开发

### 前置要求

- Rust 工具链（stable，2021 edition）
- Node.js 与 npm
- MySQL 8

### 前端

```bash
cd frontend
npm install
npm run dev
```

开发服务器默认运行在 `http://127.0.0.1:5173`；开发模式下 Vite 将 `/api`、`/uploads` 代理到后端 `http://localhost:8080`。

### 后端

```bash
cd backend
cp .env.example .env   # 然后编辑 .env，至少配置 DATABASE_URL、JWT_SECRET、FIELD_ENC_KEY
cargo run
```

后端默认监听 `127.0.0.1:8080`；首次启动会自动执行 `backend/sql/init.sql` 建表与种子数据（幂等，可重复执行），并完成存量敏感字段加密迁移。

### 首个管理员

**首个管理员**指全新部署（`employees` 表为空）时通过前端注册页创建的第一个账号：系统注册开关 `registration_open` 默认开启，注册成功后自动绑定内置 `super_admin` 角色（全量权限 + all 数据范围）并将注册开关关闭；此后注册一律返回 403。

## 部署

前端为 SvelteKit 静态构建（adapter-static），产物是纯静态文件。**npm/Node 仅在前端构建期需要，运行期不需要**——部署机无需安装 Node.js，只需 Nginx 与后端二进制。

### 1. 在构建机（有 Node 环境）构建前端

```bash
cd frontend
npm install        # 首次执行一次，安装 node_modules
npm run build      # 产物输出到 frontend/build/（约 70 个静态文件）
npm run preview    # 可选：本地预览验证构建产物
```

> 构建产物与平台无关（HTML/JS/CSS），可跨系统部署；构建机仅在构建时需要 Node，运行与部署环境不需要。

### 2. 拷贝产物到部署机

```bash
# scp 直接拷贝
scp -r frontend/build/ user@部署机:/var/www/manner/dist/

# 或打包传输（目标机执行 tar 解压到 /var/www/manner/dist/）
tar -czf manner-dist.tar.gz -C frontend build
```

### 3. 部署机配置（无需 Node.js）

部署机只需三样：

- **Nginx**：托管静态产物，`/api`、`/uploads` 反向代理到后端
- **后端二进制**：Rust 服务可执行文件（`cargo build --release` 产物），本机监听 `127.0.0.1:8080`
- **数据库**：MySQL

Nginx 配置使用项目自带模板 `docs/nginx-prod.conf.example`：替换 `<your-domain>` 与证书路径后放入 `/etc/nginx/conf.d/manner-web.conf`，然后：

```bash
nginx -t && systemctl reload nginx
```

模板已包含：SPA 回退、静态资源长缓存、安全响应头、`/api` 与 `/uploads` 反代（`client_max_body_size 100m` 与后端对齐）。后端生产环境变量示例见模板尾部注释。

### 更新前端

在构建机上重新 `npm run build`，将新的 `frontend/build/` 覆盖到部署机 `/var/www/manner/dist/` 即可，无需重启 Nginx 与后端。

## 贡献

如果你有改进建议，欢迎提交 Issue。
