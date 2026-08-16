# Manner_Web API 文档

> 本文件以 `backend/src/handlers/mod.rs`（路由）与各 handler 实现为准，2026-08 核对。
> 相关文档：[数据库设计.md](./数据库设计.md) · [权限系统设计.md](./权限系统设计.md) · [架构设计.md](./架构设计.md) · 生产反代配置见 [`nginx-prod.conf.example`](./nginx-prod.conf.example)。

---

## 一、API 基础约定

### 1.1 Base URL

- 所有业务接口以 `/api` 为前缀，如 `POST /api/auth/login`。
- 开发环境：Vite dev server 将 `/api` 与 `/uploads` 代理到 `http://localhost:8080`（见 `frontend/vite.config.ts`），前端可直接使用相对路径。
- 生产环境：Nginx 将 `/api` 与 `/uploads` 反向代理到后端 `127.0.0.1:8080`，`client_max_body_size 100m` 与后端 100MB 上传硬限制配套。
- 静态资源路径 `/uploads/*` 为静态文件服务（见第五节），不属于 `/api` 前缀。

### 1.2 统一响应格式

所有接口返回 JSON，结构固定为：

```json
{
  "code": 0,
  "message": "ok",
  "data": { }
}
```

- 成功：`code = 0`，HTTP 状态一律 **200**（不使用 201 Created）。
- 失败：`data = null`，`code` 为业务错误码，HTTP 状态见错误码表。
- `message` 为人类可读信息，可直接展示给用户。

### 1.3 错误码表

| code | HTTP | 含义 |
| --- | --- | --- |
| 0 | 200 | 成功 |
| 40000 | 400 | 参数错误（含 422 校验失败、文本型 400、415 不支持类型，由响应硬化中间件统一收敛到此码） |
| 40001 | 401 | 用户名或密码错误（登录/首登/预检统一，不区分账号不存在或禁用） |
| 40002 | 401 | Token 无效或已过期 |
| 40003 | 401 | Token 已被注销（命中黑名单） |
| 40004 | 403 | 无权限访问 |
| 40005 | 400 | 校验失败（如密码长度不合规 8~72 字节、设置格式非法） |
| 40006 | 404 | 资源不存在 |
| 40007 | 409 | 用户名已存在 |
| 40008 | 400 | 旧密码错误 |
| 40009 | 429 | 请求过于频繁（响应携带 `Retry-After` 头） |
| 40010 | 401 | 会话已失效——该用户已在其他设备登录（单设备登录被顶下线） |
| 50000 | 500 | 服务器内部错误 |

### 1.4 认证方式

- 双令牌机制：`access` 令牌（默认 30 分钟）承载权限与授权快照；`refresh` 令牌（默认 7 天）仅用于续期。
- Cookie 会话（推荐，浏览器默认行为）：
  - `manner_token`：access 令牌。
  - `manner_refresh`：refresh 令牌。
  - `manner_csrf`：CSRF 双提交令牌（**非 HttpOnly**，前端 JS 读取后经 `X-CSRF-Token` 头随写请求回传）。
  - 属性：`SameSite=Strict; Path=/; Max-Age=...`（`manner_token`/`manner_refresh` 另带 `HttpOnly`）；配置 `COOKIE_SECURE=true`（生产必须）时追加 `Secure`。
  - 登出时后端清除三个 Cookie。
- 兼容 `Authorization: Bearer <token>` 头方式（access 令牌），非浏览器客户端可用。
- `refresh` 令牌**只允许**用于 `POST /api/auth/refresh`；将其冒充 access 令牌访问其他接口一律拒绝（`typ` 校验）。
- 令牌校验链：验签 → `typ` 必须为 `access` → `jti` 不在 `token_blacklist` → 员工存在且 `pwd_version` 与库一致且 `status = 1` → `active_session` 单设备校验 → `perm_version` 失效检测。
- **CSRF**：经 Cookie 认证的浏览器会话，对写方法（POST/PUT/DELETE/PATCH）要求 `X-CSRF-Token` 头与 `manner_csrf` Cookie 一致，否则 `403`；`Authorization: Bearer` 认证不受 CSRF 约束。
- 401 自动续期流程（前端 `client.ts` 行为）：收到 401 时并发去重，用 `manner_refresh` 调 `/api/auth/refresh` 换新会话后重放原请求；刷新失败则登出并跳转登录页。

### 1.5 Content-Type 与请求体约定

- 业务接口请求/响应均为 `application/json`。
- 上传接口（`POST /api/upload`、`POST /api/upload/file`）为 `multipart/form-data`，文件字段名固定为 `file`。
- GET / DELETE 接口无请求体。
- 请求体 JSON 解析失败（422）、文本型 400、不支持的 Content-Type（415）统一收敛为 `code=40000`。

### 1.6 CORS

- 白名单来源：`CORS_ALLOWED_ORIGINS`（默认 `http://localhost:5173,http://127.0.0.1:5173`，逗号分隔；生产应配置为正式域名）。
- 允许方法：GET / POST / PUT / DELETE / OPTIONS；允许头：Content-Type / Accept / Authorization / X-CSRF-Token；`allow_credentials(true)`（配合 Cookie 会话）。
- 配置含 `*` 时启动输出安全告警。

### 1.7 通用错误场景

| 场景 | 响应 |
| --- | --- |
| 未携带 token 或 token 无效/过期 | 401，`40002` |
| token 已被登出/轮换注销 | 401，`40003` |
| 已登录但缺少所需权限码 | 403，`40004` |
| 请求未注册的路径 | 401，`40002`（统一回落，不泄露路由差分） |
| 登录/预检/首登被限流 | 429，`40009`，携带 `Retry-After` |
| 账号被禁用（status ≠ 1）后持旧令牌访问 | 401，`40002` |
| 已在其他设备登录被顶下线 | 401，`40010` |

---

## 二、权限码表（29 个）

权限经**角色授权**派生（RBAC + 数据范围 + 部门角色继承，见 [权限系统设计.md](./权限系统设计.md)），员工级直接授权已移除。access 令牌内嵌授权快照（`permissions` + `grants` + `perm_version`），权限变更后 `perm_version` 失配即由中间件重算，**即时生效**。

| 模块 | code | 名称 | 校验的端点 |
| --- | --- | --- | --- |
| employee | employee:list | 查看员工列表 | GET /api/employees（数据范围过滤） |
| employee | employee:view | 查看员工详情 | GET /api/employees/:id（数据范围过滤） |
| employee | employee:create | 新增员工 | POST /api/employees |
| employee | employee:edit | 编辑员工 | PUT /api/employees/:id、PUT /api/employees/:id/departments |
| employee | employee:delete | 删除员工 | DELETE /api/employees/:id |
| employee | employee:password | 重置员工密码 | PUT /api/employees/:id/password |
| employee | employee:view_sensitive | 查看敏感信息 | POST /api/employees/:id/sensitive、POST /api/employees/:id/sensitive/:field（数据范围过滤） |
| department | department:list | 查看部门列表 | GET /api/departments |
| department | department:view | 查看部门详情 | GET /api/departments/:id/members |
| department | department:create | 新增部门 | POST /api/departments |
| department | department:edit | 编辑部门 | PUT /api/departments/:id |
| department | department:delete | 删除部门 | DELETE /api/departments/:id |
| role | role:manage | 角色管理 | GET/POST /api/roles、PUT/DELETE /api/roles/:id、PUT /api/employees/:id/roles、GET/PUT /api/departments/:id/roles、GET /api/permissions |
| chat | chat:protect_block | 防拉黑保护 | 目标有效权限含该码则不可被拉黑（POST /api/chat/block） |
| chat | chat:upload | 上传文件 | POST /api/upload/file |
| chat | chat:group_create | 群聊创建 | POST /api/chat/conversations |
| system | system:settings | 系统设置 | GET /api/system/health、GET/PUT /api/system/settings、GET /api/system/logs |
| system | system:config | 系统配置 | 种子保留码，当前无端点校验 |
| finance | finance:reimburse_view | 查看报销单 | GET /api/finance/reimbursements、GET /api/finance/reimbursements/:id（数据范围过滤） |
| finance | finance:reimburse_create | 提交报销 | POST /api/finance/reimbursements、PUT/DELETE /api/finance/reimbursements/:id（本人）、POST /api/finance/reimbursements/:id/withdraw |
| finance | finance:reimburse_approve | 审批报销 | POST /api/finance/reimbursements/:id/approve（数据范围过滤） |
| finance | finance:reimburse_manage | 财务复核/付款 | POST /api/finance/reimbursements/:id/review、POST /api/finance/reimbursements/:id/pay、财务侧编辑/删除/撤回 |
| finance | finance:invoice_manage | 发票管理 | GET/POST /api/finance/invoices、PUT/DELETE /api/finance/invoices/:id |
| finance | finance:payment_manage | 收付款管理 | GET/POST /api/finance/payments、PUT/DELETE /api/finance/payments/:id |
| finance | finance:budget_manage | 预算管理 | GET/POST /api/finance/budgets、PUT/DELETE /api/finance/budgets/:id |
| finance | finance:report_view | 财务报表 | GET /api/finance/reports/*（汇总/排行/趋势/导出） |
| task | task:create | 创建任务 | POST /api/tasks |
| task | task:view_all | 查看全员任务 | GET /api/tasks、GET /api/tasks/stats（无该码仅见本人任务） |
| task | task:manage | 管理任务 | PUT/DELETE /api/tasks/:id（无该码仅可维护本人任务） |

说明：

- `system:config` 为种子预留码，当前**没有端点校验**，保留以备后续功能。
- 报销单数据范围：`finance:reimburse_view` / `finance:reimburse_approve` 为数据型权限，范围作用于**提交时的部门快照**（提交人无部门时拒绝提交）；本人提交的单始终对本人可见。
- 首个管理员：系统初始时（`registration_open=1` 且员工表为空）经 `POST /api/auth/register` 注册，注册成功即绑定内置 `super_admin` 角色（全量权限、all 范围），并关闭注册通道（`registration_open` 置 `0`）。

---

## 三、路由总表（82 个 + /uploads 静态）

### 匿名（7 个）

| # | 方法 | 路径 | 权限 | Handler |
| --- | --- | --- | --- | --- |
| 1 | POST | /api/auth/login | 匿名 | auth::login |
| 2 | POST | /api/auth/register | 匿名（仅首个管理员） | auth::register |
| 3 | POST | /api/auth/precheck | 匿名 | auth::precheck |
| 4 | POST | /api/auth/first-login | 匿名 | auth::first_login |
| 5 | POST | /api/auth/refresh | 匿名（需 refresh 令牌） | auth::refresh |
| 6 | GET | /api/system/login-page | 匿名 | system::get_login_page_settings |
| 7 | GET | /api/system/icon/:key | 匿名 | system::get_site_icon |

### 受保护（75 个）

| # | 方法 | 路径 | 权限 | Handler |
| --- | --- | --- | --- | --- |
| 8 | GET | /api/system/health | system:settings | system::health |
| 9 | GET | /api/system/settings | system:settings | system::get_settings |
| 10 | POST | /api/auth/logout | 登录 | auth::logout |
| 11 | PUT | /api/auth/password | 登录 | auth::change_password |
| 12 | GET | /api/auth/me | 登录 | auth::me |
| 13 | GET | /api/auth/preferences | 登录 | auth::get_preferences |
| 14 | PUT | /api/auth/preferences | 登录 | auth::update_preferences |
| 15 | GET | /api/employees | employee:list（数据范围） | employee::list_employees |
| 16 | POST | /api/employees | employee:create | employee::create_employee |
| 17 | GET | /api/employees/:id | employee:view（数据范围） | employee::get_employee |
| 18 | PUT | /api/employees/:id | employee:edit | employee::update_employee |
| 19 | DELETE | /api/employees/:id | employee:delete | employee::delete_employee |
| 20 | POST | /api/employees/:id/sensitive | employee:view_sensitive（数据范围） | employee::view_sensitive_info |
| 21 | POST | /api/employees/:id/sensitive/:field | employee:view_sensitive（数据范围） | employee::view_sensitive_field |
| 22 | PUT | /api/employees/:id/password | employee:password | employee::reset_password |
| 23 | PUT | /api/employees/:id/departments | employee:edit | department::update_employee_departments |
| 24 | PUT | /api/employees/:id/roles | role:manage | role::update_employee_roles |
| 25 | GET | /api/roles | role:manage | role::list_roles |
| 26 | POST | /api/roles | role:manage | role::create_role |
| 27 | PUT | /api/roles/:id | role:manage | role::update_role |
| 28 | DELETE | /api/roles/:id | role:manage | role::delete_role |
| 29 | GET | /api/departments/:id/roles | role:manage | role::list_department_roles |
| 30 | PUT | /api/departments/:id/roles | role:manage | role::update_department_roles |
| 31 | GET | /api/departments | department:list | department::list_departments |
| 32 | POST | /api/departments | department:create | department::create_department |
| 33 | PUT | /api/departments/:id | department:edit | department::update_department |
| 34 | DELETE | /api/departments/:id | department:delete | department::delete_department |
| 35 | GET | /api/departments/:id/members | department:view | department::list_department_members |
| 36 | GET | /api/permissions | role:manage | system::list_permissions |
| 37 | POST | /api/upload | 登录（图片，100MB 上限） | system::upload |
| 38 | POST | /api/upload/file | chat:upload（100MB 上限） | system::upload_file |
| 39 | GET | /api/system/logs | system:settings | system::logs |
| 40 | PUT | /api/system/settings | system:settings | system::update_settings |
| 41 | GET | /api/chat/conversations | 登录 | chat::list_conversations |
| 42 | POST | /api/chat/conversations | chat:group_create | chat::create_group_conversation |
| 43 | GET | /api/chat/direct/:peer_id | 登录 | chat::get_or_create_direct_conversation |
| 44 | GET | /api/chat/conversations/:id/messages | 登录 + 会话成员 | chat::get_messages |
| 45 | POST | /api/chat/conversations/:id/messages | 登录 + 会话成员 | chat::send_message |
| 46 | PUT | /api/chat/conversations/:id/name | 登录 + 群管理员 | chat::update_group_name |
| 47 | POST | /api/chat/conversations/:id/participants | 登录 + 群管理员 | chat::add_participant |
| 48 | PUT | /api/chat/conversations/:id/participants/:target_id | 登录（群：管理员可操作他人/本人；单聊：仅本人） | chat::update_participant |
| 49 | DELETE | /api/chat/conversations/:id/participants/:target_id | 登录 + 群管理员 | chat::remove_participant |
| 50 | DELETE | /api/chat/conversations/:id/disband | 登录 + 群管理员 | chat::disband_group |
| 51 | POST | /api/chat/block | 登录 | chat::block_user |
| 52 | DELETE | /api/chat/block/:id | 登录 | chat::unblock_user |
| 53 | GET | /api/chat/blocked | 登录 | chat::list_blocked |
| 54 | GET | /api/chat/employees | 登录（employee:view 持有者按数据范围过滤） | chat::list_employees_for_chat |
| 55 | GET | /api/chat/file/:name | 登录 + 相关会话成员 | chat::get_chat_file |
| 56 | GET | /api/finance/reimbursements | finance:reimburse_view/approve/manage（数据范围） | finance::list_reimbursements |
| 57 | POST | /api/finance/reimbursements | finance:reimburse_create | finance::create_reimbursement |
| 58 | GET | /api/finance/reimbursements/:id | finance:reimburse_view/approve/manage（数据范围） | finance::get_reimbursement |
| 59 | PUT | /api/finance/reimbursements/:id | 本人（reimburse_create）或财务（reimburse_manage） | finance::update_reimbursement |
| 60 | DELETE | /api/finance/reimbursements/:id | 本人（驳回/撤回）或财务（reimburse_manage） | finance::delete_reimbursement |
| 61 | POST | /api/finance/reimbursements/:id/approve | finance:reimburse_approve（数据范围） | finance::approve_reimbursement |
| 62 | POST | /api/finance/reimbursements/:id/review | finance:reimburse_manage | finance::review_reimbursement |
| 63 | POST | /api/finance/reimbursements/:id/pay | finance:reimburse_manage | finance::pay_reimbursement |
| 64 | POST | /api/finance/reimbursements/:id/withdraw | 本人（reimburse_create）或财务 | finance::withdraw_reimbursement |
| 65 | GET | /api/finance/invoices | finance:invoice_manage | finance::list_invoices |
| 66 | POST | /api/finance/invoices | finance:invoice_manage | finance::create_invoice |
| 67 | PUT | /api/finance/invoices/:id | finance:invoice_manage | finance::update_invoice |
| 68 | DELETE | /api/finance/invoices/:id | finance:invoice_manage | finance::delete_invoice |
| 69 | GET | /api/finance/payments | finance:payment_manage | finance::list_payments |
| 70 | POST | /api/finance/payments | finance:payment_manage | finance::create_payment |
| 71 | PUT | /api/finance/payments/:id | finance:payment_manage | finance::update_payment |
| 72 | DELETE | /api/finance/payments/:id | finance:payment_manage | finance::delete_payment |
| 73 | GET | /api/finance/budgets | finance:budget_manage | finance::list_budgets |
| 74 | POST | /api/finance/budgets | finance:budget_manage | finance::create_budget |
| 75 | PUT | /api/finance/budgets/:id | finance:budget_manage | finance::update_budget |
| 76 | DELETE | /api/finance/budgets/:id | finance:budget_manage | finance::delete_budget |
| 77 | GET | /api/finance/reports/summary | finance:report_view | finance::report_summary |
| 78 | GET | /api/finance/reports/departments | finance:report_view | finance::report_departments |
| 79 | GET | /api/finance/reports/trend | finance:report_view | finance::report_trend |
| 80 | GET | /api/finance/reports/export/reimbursements | finance:report_view | finance::export_reimbursements |
| 81 | GET | /api/finance/reports/export/payments | finance:report_view | finance::export_payments |
| 82 | GET | /api/tasks | task:view_all（无则仅本人任务） | task::list_tasks |
| 83 | POST | /api/tasks | task:create | task::create_task |
| 84 | GET | /api/tasks/stats | task:view_all（无则本人统计） | task::task_stats |
| 85 | PUT | /api/tasks/:id | 本人（创建者/负责人）或 task:manage | task::update_task |
| 86 | DELETE | /api/tasks/:id | 本人（创建者/负责人）或 task:manage | task::delete_task |

### 静态资源

| 方法 | 路径 | 权限 | 说明 |
| --- | --- | --- | --- |
| GET | /uploads/* | 登录（Bearer 或 Cookie） | 见第五节「静态资源访问」 |

---

## 四、端点详情

### 4.1 认证模块（auth）

#### POST /api/auth/login

匿名。登录并签发会话。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| username | string | 是 | 用户名 |
| password | string | 是 | 密码（bcrypt 校验） |

成功响应 `data`：

| 字段 | 说明 |
| --- | --- |
| token | access 令牌 |
| expires_in | 有效秒数（默认 1800） |
| user.id / username / name | 员工基本信息 |
| user.permissions | 权限码数组（取自令牌） |
| user.avatar | 头像 URL（可能为 null） |
| user.must_change_password | 是否处于首登强制改密状态 |

同时 `Set-Cookie`：`manner_token`、`manner_refresh`、`manner_csrf`。

错误场景：`40001`（用户名或密码错误，含账号禁用时统一返回，不区分）、`40009`（节流，429 + Retry-After）。

特殊行为：登录成功后清除该 IP 与该用户名的失败计数；账号 `status ≠ 1`（禁用）拒绝登录；登录会生成新会话 id 覆盖 `employees.active_session`（**单设备登录**：旧设备令牌立即失效，下次请求返回 40010）。

#### POST /api/auth/register

匿名，仅限**首个管理员**注册（系统尚无任何账号且 `registration_open = '1'`）。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| username | string | 是 | 用户名 |
| password | string | 是 | 密码（8~72 字节） |
| name | string | 是 | 姓名 |
| email | string | 否 | 邮箱 |

成功响应 `data`：与登录 `user` 结构一致（`id/username/name/permissions/avatar/must_change_password`），**不签发会话**，注册后需登录。

特殊行为：

- 事务内 `SELECT ... FOR UPDATE` 锁定注册开关，防并发抢注出多个管理员。
- 成功后自动授予**全部权限码**，并将 `registration_open` 置 `0`。
- 已有账号后调用一律 `403`（`40004`）。

错误场景：`40004`（注册通道已关闭）、`40007`（用户名已存在）、`40005`（密码长度不合规，8~72 字节）。

#### POST /api/auth/precheck

匿名。预检用户名是否处于「首次登录待设置密码」状态。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| username | string | 是 | 用户名 |

成功响应 `data`：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| must_change | bool | true 表示该账号待首登改密；用户名不存在或无需改密均为 false |

特殊行为：执行与 bcrypt 同开销的假校验后再查库，避免时序侧信道枚举；与登录共用节流。

错误场景：`40009`。

#### POST /api/auth/first-login

匿名。首次登录：校验初始密码 → 设置新密码 → 自动签发会话。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| username | string | 是 | 用户名 |
| initial_password | string | 是 | 当前生效的初始密码（创建员工/重置密码时下发） |
| new_password | string | 是 | 新密码（8~72 字节） |

成功响应 `data`：与登录一致（LoginResponse + 三个 Cookie）。

特殊行为：

- 仅 `must_change_password = 1` 的账号可走此流程，否则统一 `40001`。
- 改密成功后 `pwd_version` 递增（此前签发的令牌全部失效）、`must_change_password` 置 0。

错误场景：`40001`（账号不存在/非待激活/禁用/初始密码错误，统一不区分）、`40005`（密码长度不合规，8~72 字节）、`40009`。

#### POST /api/auth/refresh

匿名（需携带 refresh 令牌）。静默续期：用 refresh 令牌换取全新 access + refresh 会话。

- 令牌来源：Cookie `manner_refresh` 或 `Authorization: Bearer`（与 access 令牌双通道读取一致）。
- 校验：签名 → `typ` 必须为 `refresh` → 不在黑名单 → 员工存在、启用且 `pwd_version` 一致 → `active_session` 单设备校验。
- 成功后**轮换**：旧 refresh 的 `jti` 立即入黑名单，返回新的 LoginResponse 并覆盖三个 Cookie；refresh 复用当前会话 id，不覆盖 `active_session`。

错误场景：`40002`（令牌无效/过期）、`40003`（已被注销或轮换）、`40010`（已在其他设备登录，续期被拒）。

#### POST /api/auth/logout

登录。登出并使会话失效。

- 无请求体。当前 access 令牌的 `jti` 与 refresh 令牌的 `jti`（若有效）同时入 `token_blacklist`。
- 响应清除三个 Cookie（`Max-Age=0`）。

错误场景：`40002`（令牌无效/过期）、`40003`。

#### PUT /api/auth/password

登录。修改自己的密码。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| old_password | string | 是 | 旧密码 |
| new_password | string | 是 | 新密码（8~72 字节） |

特殊行为：成功后 `pwd_version` 递增——**所有已登录会话全部失效**（含本会话，前端将收到 401 引导重新登录），并清除首登改密标记。

错误场景：`40008`（旧密码错误）、`40005`（密码长度不合规，8~72 字节）、`40002`。

#### GET /api/auth/me

登录。当前登录员工信息。

成功响应 `data`：`id / username / name / email / title / phone / avatar / permissions`（permissions 取自令牌）。

错误场景：`40006`（员工不存在，如已被删除）、`40002`。

#### GET /api/auth/preferences

登录。读取个人偏好。

成功响应 `data`：偏好 JSON 对象（未设置过则为 `{}`）。

#### PUT /api/auth/preferences

登录。更新个人偏好（白名单 schema 校验）。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| preferences | object | 是 | 偏好对象，见下 |

`preferences` 白名单字段：

| 字段 | 合法值 |
| --- | --- |
| theme | light / dark / system |
| timezoneMode | system / manual |
| timezoneOffset | number |
| newConvPosition | first / last |

特殊行为：未知字段与非法取值一律丢弃后入库（防任意 JSON 入库）；`preferences` 非对象 → `40000`（「preferences 必须是 JSON 对象」）。

### 4.2 员工模块（employee）

#### GET /api/employees

权限：employee:list。分页列表。

查询参数：

| 参数 | 类型 | 默认 | 说明 |
| --- | --- | --- | --- |
| page | int | 1 | 页码，clamp 1~10000 |
| page_size | int | 20 | 每页数量，clamp 1~100 |
| keyword | string | — | LIKE 匹配 name / email / phone |

成功响应 `data`：

| 字段 | 说明 |
| --- | --- |
| items | 员工行数组（id/username/name/title/email/phone/id_number/address/avatar/hire_date/status/created_at）；按 `employee:list` 数据范围过滤；email/phone/id_number/address 为掩码 |
| total / page / page_size | 总数与分页信息 |

按 `created_at DESC` 排序。错误场景：`40004`。

#### GET /api/employees/:id

权限：employee:view。员工详情（目标不在数据范围内按 `40006` 处理，防探测）。

成功响应 `data`：列表行全部字段 + `permissions`（有效权限码数组）+ `grants`（有效授权，含数据范围）+ `department_ids` + `role_ids`（分配的角色）+ `updated_at`。

错误场景：`40006`（不存在/不可见）、`40004`。

#### POST /api/employees

权限：employee:create。创建员工。

请求体（除 username/name 外均可选）：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| username | string | 是 | 用户名（唯一） |
| name | string | 是 | 姓名 |
| title / email / phone / id_number / address | string | 否 | 基本信息（email/phone/id_number/address 加密落库） |
| hire_date | date | 否 | 入职日期 |

成功响应 `data`：

| 字段 | 说明 |
| --- | --- |
| id / username / name | 员工基本信息 |
| initial_password | **随机生成的初始密码（16 位，仅此一次返回）**，须经安全渠道转交员工 |
| must_change_password | true（首登必须改密） |

特殊行为：创建成功后自动创建「操作者 ↔ 新员工」**单聊会话**（`INSERT IGNORE`，幂等）。

错误场景：`40007`（用户名已存在）、`40004`。

#### PUT /api/employees/:id

权限：employee:edit。更新员工。

请求体（全部可选，`null` 表示清空该字段，缺省表示不修改）：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| name | string | 姓名 |
| title / email / phone / id_number / address | string/null | 可清空 |
| avatar | string/null | 头像 URL（仅接受本站上传图片 `/uploads/<uuid>.<图片扩展名>`，其余格式 → `40000`「头像必须是本站上传的图片」） |
| hire_date | date/null | 可清空 |
| status | int | 1 启用 / 非 1 禁用 |

特殊行为：

- **不能修改自己的资料**（仅 avatar 例外），否则 `40000`「员工管理不能更改自己的资料」。
- 无任何字段 → 直接成功。

错误场景：`40000`、`40006`（不存在）、`40004`。

#### DELETE /api/employees/:id

权限：employee:delete。删除员工。

特殊行为：不能删除自己（`40000`「不能删除自己」）；删除后其 `employee_roles`、`employee_departments` 等关联随外键 CASCADE 清除。

错误场景：`40000`、`40006`、`40004`。

#### PUT /api/employees/:id/password

权限：employee:password。重置员工密码。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| new_password | string | 是 | 管理员为其设置的新密码（8~72 字节） |

特殊行为：不能重置自己（`40000`「不能重置自己的密码，请在个人资料中修改」）；成功后 `pwd_version` 递增（该员工所有会话被踢出）且 `must_change_password = 1`（下次登录强制改密）。响应 `message` 为「密码已重置」。

错误场景：`40000`、`40006`、`40004`。

#### POST /api/employees/:id/sensitive 与 POST /api/employees/:id/sensitive/:field

权限：employee:view_sensitive（数据范围过滤）。查看员工敏感信息明文。

- `POST /api/employees/:id/sensitive`：返回全部敏感字段明文（`email/phone/id_number/address`）。
- `POST /api/employees/:id/sensitive/:field`：仅返回指定字段明文，`:field` ∈ `email|phone|id_number|address`。

特殊行为：

- 目标不在数据范围内 → `40006`（防探测）。
- 查看行为**强制写审计日志**（记录操作者、目标、IP）。
- 解密失败（非 `enc:v1:` 前缀的存量数据）按明文原样返回。

错误场景：`40006`、`40004`。

#### PUT /api/employees/:id/roles

权限：role:manage。整体替换员工分配的角色（RBAC）。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| role_ids | string[] | 是 | 分配后的完整角色 id 集合（覆盖式；空数组表示清空） |

特殊行为：

- 不能修改自己的角色（`40000`「不能修改自己的角色」）。
- **防提权**：目标角色有效授权（含父角色继承）必须是操作者自身授权的子集，否则 `40004`。
- **super_admin 保护**：移除后系统须至少保留一名超级管理员，否则 `40000`。
- 成功后目标员工 `perm_version` 递增，其权限变更即时生效（无需重登）。

错误场景：`40000`、`40004`、`40006`。

### 4.3 部门模块（department）

#### GET /api/departments

权限：department:list。部门树列表。

成功响应 `data`：

| 字段 | 说明 |
| --- | --- |
| items | 部门数组，每项含 `id/name/parent_id/sort_order/leader_names/leader_ids/role_names/member_count` |
| total | 部门总数 |

排序：`sort_order ASC, created_at ASC`。`parent_id` 为 null 表示根部门。

#### POST /api/departments

权限：department:create。创建部门。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| name | string | 是 | 部门名称（trim 后非空，≤64 字符） |
| parent_id | string | 否 | 上级部门 id（须存在，防环） |
| leader_ids | string[] | 否 | 部门负责人员工 id 列表 |
| sort_order | int | 否 | 排序（默认 0） |

成功响应 `data`：`{ "id": "..." }`。

错误场景：`40005`（名称为空/超长）、`40000`（父部门不存在/包含无效负责人）、`40004`。

#### PUT /api/departments/:id

权限：department:edit。更新部门（全部可选，`parent_id: null` 表示清空父级）。

请求体：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| name | string | 部门名称 |
| parent_id | string/null | 上级部门（null 清空，防环） |
| sort_order | int | 排序 |
| leader_ids | string[] | 部门负责人整体替换 |

特殊行为：部门结构变更会影响 `subtree` 数据范围，更新后**全体员工 `perm_version` 递增**。

错误场景：`40006`、`40000`（父部门不存在/成环/包含无效负责人）、`40005`、`40004`。

#### DELETE /api/departments/:id

权限：department:delete。删除部门。

特殊行为：存在子部门时拒绝（`40000`「该部门存在子部门，请先删除子部门」）；负责人与成员关联随外键级联清理；删除后全体员工 `perm_version` 递增。

错误场景：`40000`、`40006`、`40004`。

#### GET /api/departments/:id/members

权限：department:view。部门成员列表。

成功响应 `data`：

| 字段 | 说明 |
| --- | --- |
| items | 成员数组（`id/username/name/title/avatar/status/is_leader`） |
| total | 成员总数 |

排序：负责人优先（`is_leader DESC, created_at ASC`）。错误场景：`40006`（部门不存在）、`40004`。

#### PUT /api/employees/:id/departments

权限：employee:edit。整体替换员工归属部门（多对多）。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| department_ids | string[] | 是 | 归属部门 id 集合（覆盖式；空数组表示清空） |

特殊行为：

- 不能修改自己的归属部门（`40000`）。
- 部门 id 须全部存在，否则 `40000`「指定的部门不存在」。
- 成功后该员工 `perm_version` 递增（部门角色继承与数据范围即时生效）。

错误场景：`40000`、`40006`、`40004`。

#### 部门角色绑定（role:manage）

| 端点 | 说明 |
| --- | --- |
| GET /api/departments/:id/roles | 部门绑定的角色列表（`{ items: [{id, name}], total }`） |
| PUT /api/departments/:id/roles | 整体替换部门绑定的角色：`{ role_ids[] }`（super_admin 不允许经部门绑定 → `40000`） |

成功响应：「ok」。错误场景：`40006`、`40004`、`40000`。

### 4.4 角色模块（role）

#### GET /api/roles

权限：role:manage。角色列表。

成功响应 `data`：

| 字段 | 说明 |
| --- | --- |
| items | 角色数组，每项含 `id/name/parent_id/parent_name/is_system/scope_type/scope_department_ids/permission_codes/member_count/description/created_at` |
| total | 角色总数 |

排序：`is_system DESC, created_at ASC`。`member_count` 为直接分配 + 经部门继承的持有员工数。

#### POST /api/roles

权限：role:manage。创建角色。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| name | string | 是 | 角色名称（trim 后非空，≤64 字符，唯一） |
| parent_id | string | 否 | 父角色 id（继承链，防环） |
| scope_type | string | 是 | all / subtree / department / self / custom |
| scope_department_ids | string[] | custom 时必填 | custom 范围指定的部门集合 |
| permission_codes | string[] | 是 | 权限码集合 |
| description | string | 否 | 描述（≤255 字符） |

特殊行为（防提权）：

- 父角色须存在；子角色数据范围不得大于父角色（`40000`）。
- 权限码与部门须全部有效（`40000`）。
- **防提权**：结果集有效授权（自身 ∪ 父角色）必须 ⊆ 操作者授权，否则 `403`（`40004`）。

成功响应 `data`：`{ "id": "..." }`。

错误场景：`40005`、`40000`、`40007`（角色名已存在）、`40004`。

#### PUT /api/roles/:id

权限：role:manage。更新角色。

请求体（全部可选，同创建字段）：`name / parent_id(null 清空) / scope_type / scope_department_ids / permission_codes / description`。

特殊行为：

- `is_system` 角色（super_admin）**仅允许改描述**，改其他字段 → `40000`。
- 父角色不能是自身；沿 `parent_id` 防环（`would_create_cycle`）。
- 子范围不得大于父范围；结果集（自身 ∪ 父角色）须在操作者授权内，否则 `40004`。
- 变更后受影响的持有员工 `perm_version` 递增（即时生效）。

错误场景：`40006`、`40005`、`40000`、`40007`、`40004`。

#### DELETE /api/roles/:id

权限：role:manage。删除角色。

特殊行为：`is_system` 角色拒绝删除（`40000`）；存在子角色时拒绝（`40000`「该角色存在子角色，请先处理子角色继承」）；须在操作者授权内（`40004`）；删除后受影响员工 `perm_version` 递增。

错误场景：`40000`、`40006`、`40004`。

#### GET /api/permissions

权限：role:manage。权限字典（按模块分组，供角色管理界面勾选权限使用）。

成功响应 `data`：

```json
{
  "modules": [
    { "module": "employee", "module_name": "员工管理", "permissions": [ { "code": "employee:list", "name": "查看员工列表" } ] },
    { "module": "system", "module_name": "系统设置", "permissions": [ ] },
    { "module": "chat", "module_name": "聊天", "permissions": [ ] }
  ]
}
```

### 4.5 聊天模块（chat）

#### GET /api/chat/conversations

登录。当前员工参与的会话列表。

成功响应 `data`：`ConversationResponse[]`，每项：

| 字段 | 说明 |
| --- | --- |
| id / type / name / created_by / created_at | 会话基本信息（type: single/group） |
| last_message / last_time | 最后一条消息内容与时间 |
| participants | 参与人数组（id/name/role/nickname/avatar） |
| my_role | 我的角色（member/admin） |
| my_nickname / my_group_note | 我的昵称 / 我为对方设置的备注 |

排序：`last_time DESC, created_at DESC`。

#### POST /api/chat/conversations

登录 + 持有 `chat:group_create` 权限。创建群聊，创建者自动成为群管理员（`role=admin`）。

请求体：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| name | string | 群名（trim 后非空，≤128 字符） |
| member_ids | string[] | 初始成员 ID 列表（自动去重并剔除创建者自己，≤100 人） |

特殊行为与错误场景：

- 无 `chat:group_create` 权限 → `40004`。
- 群名为空 → `40000`「群名不能为空」；超过 128 字符 → `40000`「群名不能超过 128 个字符」。
- 成员数超过 100 → `40000`「群聊成员数量不能超过 100 人」。
- 存在无效成员 ID → `40000`「包含无效成员」。

成功响应 `data`：`ConversationResponse`（`my_role` 为 `admin`，`last_message`/`last_time` 为 null）。

#### GET /api/chat/conversations/:id/messages

登录 + 会话成员。消息列表。

成功响应 `data`：`MessageResponse[]`（按 `created_at ASC`，最多 200 条）：

| 字段 | 说明 |
| --- | --- |
| id / conversation_id / sender_id | 消息与发送者标识 |
| sender_name / sender_avatar | 发送者展示名（昵称优先于姓名）与头像 |
| type | text / file |
| content / file_url / file_name | 文本内容与文件信息 |
| created_at | 发送时间 |

特殊行为：非成员访问 → `40004`；单聊中对方消息的 `sender_name` 显示「我为其设置的 group_note」（若已设置）。

#### POST /api/chat/conversations/:id/messages

登录 + 会话成员。发送消息。

请求体（content/msg_type/file_url/file_name 均可选）：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| content | string | 文本内容（≤20000 字符） |
| msg_type | string | 默认 text；`text` / `file` 二选一，其他取值 → `40000`「不支持的消息类型」 |
| file_url | string | 文件消息必须携带 |
| file_name | string | 原始文件名（≤256 字符） |

特殊行为与错误场景：

- 非成员 → `40004`；会话不存在 → `40006`。
- 单聊：对方拉黑我 → `40000`「对方已拉黑你」。
- 文件消息必须携带 `file_url` → `40000`「文件消息必须携带文件链接」。
- `file_url` 必须以 `/uploads/` 开头且扩展名合法 → `40000`「文件链接必须指向本站上传的文件」（防任意链接传播）。
- **文件归属校验**：`file_url` 已被其他会话引用时，发送者必须是相关会话的成员 → 否则 `403`；未被引用的新上传文件必须真实存在于上传目录 → 否则 `40000`「文件不存在，请重新上传」（防跨会话引用他人文件/引用伪造路径）。
- `file_name` 超 256 字符 → `40000`「文件名过长」；`content` 超 20000 字符 → `40000`「消息内容过长」。

成功响应 `data`：`MessageResponse`（同消息列表结构）。

#### PUT /api/chat/conversations/:id/name

登录 + 群管理员。修改群名。

请求体：`{ "name": "新群名" }`（trim 后非空、≤128 字符，与创建群聊一致）。非管理员 → `40004`；空名 → `40000`；超 128 字符 → `40000`。响应「已更新」。

#### POST /api/chat/conversations/:id/participants

登录 + 群管理员。添加群成员。

请求体：`{ "employee_id": "..." }`。

错误场景：`40004`（非群管理员）、`40006`（目标员工不存在）。重复添加为幂等（`INSERT IGNORE`）。响应「已添加」。

#### PUT /api/chat/conversations/:id/participants/:target_id

登录。更新参与者信息。

请求体（全部可选）：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| role | string | admin / member；仅群管理员可改他人，不能改自己 |
| nickname | string | 昵称，仅能改自己 |
| group_note | string | 单聊备注，仅能改自己 |

特殊行为与错误场景：

- 群聊：非管理员操作他人 → `40004`；把最后一名管理员降为 member → `40000`「群聊至少需要一名管理员」。
- 单聊：会话不存在 → `40006`；非成员 → `40004`；携带 role 字段 → `40004`；改他人 nickname → `40004`。

响应「已更新」。

#### DELETE /api/chat/conversations/:id/participants/:target_id

登录 + 群管理员。移除群成员。

错误场景：`40004`（非群管理员）、`40000`「不能移除自己」、「群聊至少需要一名管理员」（最后一名管理员不可移除）。响应「已移除」。

#### DELETE /api/chat/conversations/:id/disband

登录 + 群管理员。解散群聊（删除会话）。

错误场景：`40004`（非群管理员）。响应「群聊已解散」。

#### POST /api/chat/block

登录。拉黑用户。

请求体：`{ "blocked_id": "..." }`。

错误场景：`40000`「不能拉黑自己」；目标为保护账号 → `40000`「该用户受保护，无法拉黑」。重复拉黑幂等。响应「已拉黑」。

#### DELETE /api/chat/block/:id

登录。取消拉黑。响应「已取消拉黑」。

#### GET /api/chat/blocked

登录。我的拉黑列表。

成功响应 `data`：`ParticipantInfo[]`（id/name/role/nickname/avatar；name 优先取单聊中我为其设置的备注昵称）。

#### GET /api/chat/employees

登录。聊天可用员工名单（发起会话/拉人用）。

成功响应 `data`：员工（除自己外）`ParticipantInfo[]`，按 `name` 排序。持有 `employee:view` 的用户按该权限的数据范围过滤（仅见范围内员工），未持有者全量可见。

#### GET /api/chat/file/:name

登录 + 相关会话成员。鉴权下载聊天文件（替代 /uploads 静态直链）。

路径参数：`name` 为文件名（如 `uuid.png`）。

特殊行为（三重校验）：

- 文件名白名单：ASCII 字母数字 + `.` + `-`，长度 ≤64，禁止 `..`、前导点、空名；扩展名必须合法，否则 `40006`。
- 文件必须被某条消息引用（`messages.file_url` 匹配 `/uploads/chat/<name>` 或存量 `/uploads/<name>` 格式），否则 `40006`。
- 当前用户必须是引用该文件的会话的**成员**，否则 `40004`。
- 路径任一级为软链接 → `40006`。

响应：图片扩展名（png/jpg/jpeg/gif/webp/bmp/ico）内联，其余扩展名强制 `Content-Disposition: attachment`；Content-Type 映射（png/jpeg/gif/webp/bmp/ico/txt/md/log/mp4/webm/ogg/mov，缺省 octet-stream）。

### 4.6 系统模块（system）

#### GET /api/system/login-page

匿名。登录页公开配置。

成功响应 `data`（字段存在才返回）：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| login_site_title | string | 登录页标题 |
| login_theme | string | 登录页主题 |
| site_title | string | 网站标题 |
| login_site_icon | string | 登录页图标 URL（`/uploads/...`） |
| site_icon | string | 登录后站点图标 URL（`/uploads/...`） |
| default_language | string | 默认语言（`system` / `en-US` / `zh-CN`） |
| registration_open | bool | 是否开放注册（`registration_open='1'` 且员工数为 0） |

#### GET /api/system/icon/:key

匿名。公开图标服务，按 key 返回网站图标文件字节。

- `:key` ∈ `login`（登录页图标 `login_site_icon`）/ `site`（站点图标 `site_icon`），其他值 → 404。
- 仅接受 `system_settings` 中配置的 `/uploads/<file>` 路径；限 `UPLOAD_DIR` 根目录文件、禁子目录/路径穿越。
- 扩展名白名单（图片）+ **魔数校验** + 软链接拦截，任一不符 → 404（防任意文件读取）。
- 响应 `Content-Type`（png/jpg/gif/webp/bmp/ico）与 `Cache-Control: public, max-age=300`。

未配置图标 → 404。

#### GET /api/system/health

权限：system:settings。健康检查。

成功响应 `data`：`{ "server": "running", "database": "connected" | "disconnected"（3 秒超时探测）, "version": "后端版本号" }`。

#### GET /api/system/settings

权限：system:settings。读取全部系统设置。

成功响应 `data`：`system_settings` 表全部键值（字符串值 Map）：`chat_upload_limit`、`login_theme`、`site_title`、`login_site_title`、`login_site_icon`、`site_icon`、`login_max_failures`、`login_lock_window_secs`、`default_language`、`registration_open`。

#### PUT /api/system/settings

权限：system:settings。更新系统设置。

请求体（全部可选）：

| 字段 | 类型 | 校验 |
| --- | --- | --- |
| chat_upload_limit | string | 「无限制」/「禁止」/ 数字+单位（B/KB/MB/GB/TB），非法 → `40005`「上传限制格式不正确（如 10MB / 无限制 / 禁止）」 |
| login_theme / site_title / login_site_title | string | 直接保存 |
| login_site_icon / site_icon | string | 仅接受本站上传图片 URL（同头像校验） |
| login_max_failures | string | 1~100，非法 → `40005` |
| login_lock_window_secs | string | 1~86400，非法 → `40005` |
| default_language | string | system / en-US / zh-CN，非法 → `40005` |

特殊行为：变更即写库并**同步内存登录节流器**（无需重启生效）；有变更时写审计日志。响应「保存成功」。

#### GET /api/system/logs

权限：system:settings。读取业务日志（尾 N 行）。

查询参数：`lines`（默认 200）。

成功响应 `data`：`{ "lines": [...], "total": 行数, "file": "日志文件名（仅文件名，不泄露绝对路径）" }`；日志文件不存在 → 空结果。

#### POST /api/upload

登录。上传图片（头像、站点图标等）。`multipart/form-data`，字段名 `file`，请求体上限 100MB。

特殊行为与错误场景：

- 扩展名白名单：png / jpg / jpeg / gif / webp / bmp / ico，否则 `40000`「不支持的文件类型: {ext}」。
- **魔数校验**（PNG/JPEG/GIF/WEBP/BMP/ICO 文件头），不符 → `40000`「文件内容与声明类型不符」。
- 受 `chat_upload_limit` 限制：「禁止」→ `40000`「文件上传已被管理员禁止」；超限 → `40000`「文件大小超过限制 ({limit})」。
- 保存为 `{uuid}.{ext}` 至上传目录根；成功 `data` 为 URL 字符串 `/uploads/{uuid}.{ext}`。
- multipart 无文件 → `50000`「No file uploaded」。

#### POST /api/upload/file

权限：chat:upload。上传任意文件（聊天文件）。`multipart/form-data`，字段名 `file`，请求体上限 100MB。

特殊行为与错误场景：

- 扩展名须为 ASCII 字母数字、非空、≤16 字符，否则 `40000`「不支持的文件类型: {ext}」。
- 图片类仍做魔数校验（同上）。
- 保存至 `UPLOAD_DIR/chat/` 子目录；成功 `data` 为 `/uploads/chat/{uuid}.{ext}`。
- 受 `chat_upload_limit` 与 100MB 硬限制约束（同上）。

### 4.7 财务模块（finance）

财务模块权限码（module = `finance`）见第二节。报销单状态机：

```
pending_leader（待部门审批）→ pending_finance（待财务复核）→ approved（已通过）→ paid（已付款）
                                   ↘ rejected（任一环节驳回）        ↘ withdrawn（提交人撤回）
```

- 提交人在 `pending_leader / rejected / withdrawn` 状态可编辑；驳回/撤回后编辑即**重新提交**回 `pending_leader`。
- 数据范围：`finance:reimburse_view` / `finance:reimburse_approve` 按报销单**提交时部门快照**过滤（本人提交的单恒可见）。
- 金额列 `DECIMAL(12,2)`，接口统一以浮点数传递；非法金额返回 `40005`。

#### POST /api/finance/reimbursements

权限：`finance:reimburse_create`。创建并提交报销单。

请求体：

```json
{
  "title": "出差差旅费",
  "category": "travel",
  "amount": 1200.50,
  "reason": "上海出差",
  "invoice_ids": ["发票id1"]
}
```

- `category` 常见取值：travel / office / meal / transport / other（自由字符串亦可）。
- `invoice_ids`：可选；发票须存在且未被其他报销单关联，否则 `40000`。
- 提交人须已加入部门（部门快照），否则 `40000`「请先加入部门后再提交报销申请」。
- 成功 `data: { id }`。

#### GET /api/finance/reimbursements

权限：`finance:reimburse_view` / `approve` / `manage`（数据范围过滤；`manage` 全量）。

查询参数：`page`、`page_size`（默认 10，上限 100）、`status`、`keyword`（事由/提交人）、`department_id`。

响应 `data.items[]` 字段：`id, employee_id, employee_name, department_id, department_name, title, category, amount, currency, status, approver_id/name, approve_comment, finance_reviewer_id/name, finance_comment, paid_at, created_at`。

#### GET /api/finance/reimbursements/:id

权限同列表（数据范围过滤）。返回完整详情：基础字段 + `reason, approved_at, finance_reviewed_at, invoices[]（发票列表）, logs[]（审批流水）`。

#### PUT /api/finance/reimbursements/:id

- 提交人：`reimburse_create` 且为本人、状态为 `pending_leader / rejected / withdrawn`；驳回/撤回后编辑自动重新提交。
- 财务：`reimburse_manage`，任意非 `paid` 状态。
- 请求体字段均可选：`title, category, amount, reason, invoice_ids`（整体替换关联发票）。
- 已付款单不可编辑（`40000`）。

#### DELETE /api/finance/reimbursements/:id

- 提交人：删除自己的 `rejected / withdrawn` 单。
- 财务：`reimburse_manage` 删除任意非 `paid` 单。
- 删除后关联发票自动回到 `unused` 状态。

#### POST /api/finance/reimbursements/:id/approve

权限：`finance:reimburse_approve`（数据范围覆盖提交部门）。仅 `pending_leader` 状态。

```json
{ "action": "approve" | "reject", "comment": "可选；驳回必填" }
```

- 通过 → `pending_finance`；驳回 → `rejected`。不得审批/复核自己提交的单。

#### POST /api/finance/reimbursements/:id/review

权限：`finance:reimburse_manage`。仅 `pending_finance` 状态。请求体同 approve。

- 通过 → `approved`；驳回 → `rejected`。

#### POST /api/finance/reimbursements/:id/pay

权限：`finance:reimburse_manage`。仅 `approved` 状态。标记 `paid` 并**自动生成一条支出记录**（`direction=expense`、`category=报销`、`reimbursement_id` 关联、往来方=提交人）。

#### POST /api/finance/reimbursements/:id/withdraw

提交人（`reimburse_create`）或财务可操作，仅 `pending_leader / pending_finance` 状态 → `withdrawn`。

#### GET /api/finance/invoices

权限：`finance:invoice_manage`。参数：`page, page_size, keyword`（号码/开票方）、`status`（`unused` / `claimed`）。

#### POST /api/finance/invoices

权限：`finance:invoice_manage`。录入发票，**发票号码唯一查重**（重复返回 `40000`「发票号码已存在」）。

```json
{
  "invoice_code": "110024113301",
  "invoice_type": "普通发票",
  "amount": 1200.50,
  "tax_amount": 138.90,
  "issued_at": "2025-06-01",
  "issuer_name": "开票公司",
  "buyer_name": "可选",
  "image_url": "可选 /uploads 路径"
}
```

#### PUT /api/finance/invoices/:id

已关联（`claimed`）发票不可修改（`40000`）。号码变更同样查重。

#### DELETE /api/finance/invoices/:id

仅 `unused` 发票可删除。

#### GET /api/finance/payments

权限：`finance:payment_manage`。参数：`page, page_size, direction, keyword, department_id, from, to`（`occurred_at` 日期范围）。

#### POST /api/finance/payments

权限：`finance:payment_manage`。新增收付款记录。

```json
{
  "direction": "income" | "expense",
  "category": "销售收入",
  "amount": 5000,
  "counterparty": "某客户",
  "occurred_at": "2025-06-15",
  "department_id": "可选",
  "remark": "可选"
}
```

#### PUT /api/finance/payments/:id · DELETE /api/finance/payments/:id

权限：`finance:payment_manage`。报销付款自动生成的记录（`reimbursement_id` 非空）建议只读，不做强制限制。

#### GET /api/finance/budgets

权限：`finance:budget_manage`。参数：`page, page_size, period_type（year|month）, period_value, department_id`。

响应 `data.items[]` 附加聚合字段：`spent`（已用 = 已通过/已付款报销 + 非报销关联的支出付款，同期间同部门）、`remaining`（额度 - 已用）；`spent > amount` 即超支（前端红色预警）。

#### POST /api/finance/budgets

权限：`finance:budget_manage`。同一部门+期间唯一，重复返回 `40000`。

```json
{ "department_id": "...", "period_type": "month", "period_value": "2025-06", "amount": 10000 }
```

`period_value`：年 `YYYY`（如 `2025`），月 `YYYY-MM`（如 `2025-06`）。

#### PUT /api/finance/budgets/:id · DELETE /api/finance/budgets/:id

权限：`finance:budget_manage`。更新/删除预算。

#### GET /api/finance/reports/summary

权限：`finance:report_view`。参数：`from, to`（`occurred_at`/`created_at` 日期范围）。

响应：`income, expense, net, income_count, expense_count, reimbursement_pending（待付报销金额）, reimbursement_pending_count`。

#### GET /api/finance/reports/departments

权限：`finance:report_view`。参数：`from, to`。部门费用排行（支出付款 + 已通过/已付款报销），按支出降序；`total_expense` 为合计。

#### GET /api/finance/reports/trend

权限：`finance:report_view`。参数：`from, to, granularity（month 默认 | year）`。按月/年聚合收付款趋势。

#### GET /api/finance/reports/export/reimbursements · /export/payments

权限：`finance:report_view`。参数：`from, to`。返回 `text/csv`（UTF-8 BOM + CRLF，Excel 直接打开），`Content-Disposition: attachment`。

### 4.8 任务模块（task）

任务与财务**相互独立**（仅共享 employees 基础表）：员工创建/完成个人任务，持有 `task:view_all` 的管理员可查看全员任务情况（统计与列表均含全员维度）。权限码见第二节。

- 可见范围：无 `task:view_all` 时仅见「本人创建或本人负责」的任务；有则可通过 `scope=all|mine` 与 `assignee_id` 过滤。
- 可维护范围：任务的创建者或负责人可编辑/删除/标记状态；`task:manage` 可操作任意任务。

#### GET /api/tasks

权限：`task:view_all`（无则仅本人任务）。参数：`page, page_size, status（todo|done）, assignee_id, scope（all|mine）`。

响应 `data.items[]`：`id, title, description, assignee_id/name, creator_id/name, status, due_date, completed_at, created_at`；`data.can_view_all` 表明是否全员视图。

#### POST /api/tasks

权限：`task:create`。创建任务。

```json
{
  "title": "完成季度报表",
  "description": "可选",
  "assignee_id": "可选，负责人（默认本人）",
  "due_date": "可选，YYYY-MM-DD"
}
```

#### GET /api/tasks/stats

权限：`task:view_all`（无则本人统计）。响应：`total, todo, done, overdue（逾期未完成）, can_view_all`。供仪表盘任务卡片与任务页统计使用。

#### PUT /api/tasks/:id

创建者/负责人（或 `task:manage`）可更新。字段均可选：`title, description, assignee_id, due_date, status（todo|done）`。传 `status` 即标记完成/未完成（完成记录 `completed_at`，取消完成则清除）。

#### DELETE /api/tasks/:id

创建者/负责人（或 `task:manage`）可删除。

---

## 五、静态资源访问（GET /uploads/*）

`/uploads/*` 由 `ServeDir` 提供服务，外层叠加**登录认证**与扩展名守卫：

| 场景 | 结果 |
| --- | --- |
| 未登录（无有效 Bearer 或 Cookie） | 401 |
| 路径以 `/uploads/chat/` 开头 | 一律 404（聊天文件只能经 `GET /api/chat/file/:name` 鉴权下载，杜绝 URL 直链） |
| 无扩展名 | 403 |
| 路径任一级组件为软链接 | 404（防符号链接逃逸读取任意文件） |
| 文件未被 `employees.avatar` 或 `messages.file_url` 引用 | 404（引用校验，防硬链接/未引用文件被静态读取） |
| 文件不存在 | 404 |

响应头：图片扩展名（png/jpg/jpeg/gif/webp/bmp/ico）内联渲染；非图片一律强制 `Content-Disposition: attachment`（配合 `X-Content-Type-Options: nosniff` 防存储型 XSS）。

---

## 六、补充说明

### 6.1 登录节流（F-02）

- 维度：**「(真实 IP, 用户名) 组合键」+「单 IP 失败总数」**双维度（无全局锁——随机凭据无法锁死全站登录，跨 IP 无法定向锁死单一账号，防 DoS 放大）。
- 真实 IP：直连取 TCP 对端；经可信反代（`TRUSTED_PROXIES` 白名单命中，支持单 IP/CIDR）时信任 `X-Real-IP`/`X-Forwarded-For` 解析；白名单为空或对端不在白名单时忽略一切转发头。
- 默认阈值：5 次失败 / 900 秒窗口（环境变量兜底，`system_settings` 可动态调整）。
- 超限：429（code `40009`）+ `Retry-After` 头；作用于 `login` / `precheck` / `first-login`。
- 登录成功清除该 (IP, 用户名) 与该 IP 的计数；窗口过期自动复位。

### 6.2 登出与 refresh 轮换（F-22）

- 登出：access 与 refresh 的 `jti` 同时入黑名单，并清除三个 Cookie。
- 每次 refresh 续期：旧 refresh `jti` 立即入黑名单（轮换），旧令牌不可再次使用。

### 6.3 改密全端踢出（F-08）

`pwd_version` 在改密（本人修改、管理员重置、首登激活）时递增；令牌校验要求令牌内 `pwd_version` 与库一致，因此改密后**所有已签发令牌立即失效**。

### 6.4 单设备登录（F-16）

- 登录/首登生成新会话 id 写入 `employees.active_session`，旧设备令牌的 `sid` 与之不一致 → 401（code `40010`）被顶下线。
- refresh 续期复用当前会话 id，不覆盖 `active_session`。

### 6.5 未知路由统一 401

未注册路径统一回落 401（code `40002`），与匿名访问未认证响应一致，消除路由枚举差分。

### 6.6 CSRF 防护（F-7）

- 双提交令牌：登录/刷新签发 `manner_csrf` Cookie（SameSite=Strict、**非 HttpOnly**）；浏览器会话的写方法（POST/PUT/DELETE/PATCH）要求请求头 `X-CSRF-Token` 与 Cookie 值一致，否则 `403`（code `40004`）。
- 仅对「Cookie 认证 + 写方法」生效；`Authorization: Bearer` 认证（API 客户端，令牌本就在请求头）不受 CSRF 约束。
- 与 `SameSite=Strict` 叠加，形成同站语义 + 双提交令牌双重防护。

### 6.7 敏感字段加密（F-17）

- 员工 `email/phone/id_number/address` 以 AES-256-GCM 加密落库（`enc:v1:` 前缀，密钥由 `FIELD_ENC_KEY` SHA-256 派生）。
- 普通接口返回 `mask_field` 掩码；仅 `POST /api/employees/:id/sensitive(/:field)`（`employee:view_sensitive` + 数据范围）返回明文并写审计日志。

### 6.8 前端行为参考

- `credentials: 'include'` 携带 HttpOnly Cookie。
- API 路径自动补 `/api` 前缀。
- 请求 30 秒超时。
- 收到 401 时并发去重：仅一个请求发起 `/api/auth/refresh` 续期，其余等待后重放原请求；刷新失败则登出并跳转登录页（`skipAuthRedirect` 与登录相关端点豁免）。

### 6.9 数据库与权限文档

- 表结构以 [数据库设计.md](./数据库设计.md) 为准（16 张表，RBAC + 部门角色 + 数据范围）。
- 权限模型细节见 [权限系统设计.md](./权限系统设计.md)（RBAC + 数据范围 + 部门角色继承，18 个权限码）与 [权限系统重构方案.md](./权限系统重构方案.md)。
- 前端部署与代理细节见根目录 [README](../README.md) 与 [`nginx-prod.conf.example`](./nginx-prod.conf.example)。
