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
| 40005 | 400 | 校验失败（如密码不足 8 位、设置格式非法） |
| 40006 | 404 | 资源不存在 |
| 40007 | 409 | 用户名已存在 |
| 40008 | 400 | 旧密码错误 |
| 40009 | 429 | 请求过于频繁（响应携带 `Retry-After` 头） |
| 50000 | 500 | 服务器内部错误 |

### 1.4 认证方式

- 双令牌机制：`access` 令牌（默认 30 分钟）承载权限快照；`refresh` 令牌（默认 7 天）仅用于续期。
- Cookie 会话（推荐，浏览器默认行为）：
  - `manner_token`：access 令牌。
  - `manner_refresh`：refresh 令牌。
  - 属性：`HttpOnly; SameSite=Strict; Path=/; Max-Age=...`；配置 `COOKIE_SECURE=true`（生产必须）时追加 `Secure`。
  - 登出时后端清除两个 Cookie。
- 兼容 `Authorization: Bearer <token>` 头方式（access 令牌），非浏览器客户端可用。
- `refresh` 令牌**只允许**用于 `POST /api/auth/refresh`；将其冒充 access 令牌访问其他接口一律拒绝（`typ` 校验）。
- 令牌校验链：验签 → `typ` 必须为 `access` → `jti` 不在 `token_blacklist` → 员工存在且 `pwd_version` 与库一致且 `status = 1`。
- 401 自动续期流程（前端 `client.ts` 行为）：收到 401 时并发去重，用 `manner_refresh` 调 `/api/auth/refresh` 换新会话后重放原请求；刷新失败则登出并跳转登录页。

### 1.5 Content-Type 与请求体约定

- 业务接口请求/响应均为 `application/json`。
- 上传接口（`POST /api/upload`、`POST /api/upload/file`）为 `multipart/form-data`，文件字段名固定为 `file`。
- GET / DELETE 接口无请求体。
- 请求体 JSON 解析失败（422）、文本型 400、不支持的 Content-Type（415）统一收敛为 `code=40000`。

### 1.6 CORS

- 白名单来源：`CORS_ALLOWED_ORIGINS`（默认 `http://localhost:5173,http://127.0.0.1:5173`，逗号分隔；生产应配置为正式域名）。
- 允许方法：GET / POST / PUT / DELETE / OPTIONS；允许头：Content-Type / Accept / Authorization；`allow_credentials(true)`（配合 Cookie 会话）。
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

---

## 二、权限码表（11 个）

权限经 `employee_permissions` 表**直接授予员工**（员工级直接授权，角色机制已移除），不再有角色/部门分组。access 令牌签发时从数据库读取员工全部权限码写入令牌，权限变更后最长 30 分钟（令牌有效期）内自动生效。

| 模块 | code | 名称 | 校验的端点 |
| --- | --- | --- | --- |
| employee | employee:list | 查看员工列表 | GET /api/employees、GET /api/permissions |
| employee | employee:view | 查看员工详情 | GET /api/employees/:id |
| employee | employee:create | 新增员工 | POST /api/employees |
| employee | employee:edit | 编辑员工 | PUT /api/employees/:id、PUT /api/employees/:id/permissions |
| employee | employee:delete | 删除员工 | DELETE /api/employees/:id |
| employee | employee:password | 重置员工密码 | PUT /api/employees/:id/password |
| chat | chat:protect_block | 防拉黑保护 | PUT /api/employees/:id/protect-block |
| chat | chat:upload | 上传文件 | POST /api/upload/file |
| chat | chat:group_create | 群聊创建 | 种子保留码，当前无端点校验 |
| system | system:settings | 系统设置 | GET /api/system/health、GET/PUT /api/system/settings、GET /api/system/logs |
| system | system:config | 系统配置 | 种子保留码，当前无端点校验 |

说明：

- `chat:group_create` 与 `system:config` 为种子预留码，当前**没有端点校验**它们，保留以备后续功能。
- 首个管理员：系统初始时（`registration_open=1` 且员工表为空）经 `POST /api/auth/register` 注册，注册成功即被授予**全部权限码**，并关闭注册通道（`registration_open` 置 `0`）。

---

## 三、路由总表（39 个 + /uploads 静态）

### 匿名（6 个）

| # | 方法 | 路径 | 权限 | Handler |
| --- | --- | --- | --- | --- |
| 1 | POST | /api/auth/login | 匿名 | auth::login |
| 2 | POST | /api/auth/register | 匿名（仅首个管理员） | auth::register |
| 3 | POST | /api/auth/precheck | 匿名 | auth::precheck |
| 4 | POST | /api/auth/first-login | 匿名 | auth::first_login |
| 5 | POST | /api/auth/refresh | 匿名（需 refresh 令牌） | auth::refresh |
| 6 | GET | /api/system/login-page | 匿名 | system::get_login_page_settings |

### 受保护（33 个）

| # | 方法 | 路径 | 权限 | Handler |
| --- | --- | --- | --- | --- |
| 7 | GET | /api/system/health | system:settings | system::health |
| 8 | GET | /api/system/settings | system:settings | system::get_settings |
| 9 | POST | /api/auth/logout | 登录 | auth::logout |
| 10 | PUT | /api/auth/password | 登录 | auth::change_password |
| 11 | GET | /api/auth/me | 登录 | auth::me |
| 12 | GET | /api/auth/preferences | 登录 | auth::get_preferences |
| 13 | PUT | /api/auth/preferences | 登录 | auth::update_preferences |
| 14 | GET | /api/employees | employee:list | employee::list_employees |
| 15 | POST | /api/employees | employee:create | employee::create_employee |
| 16 | GET | /api/employees/:id | employee:view | employee::get_employee |
| 17 | PUT | /api/employees/:id | employee:edit | employee::update_employee |
| 18 | DELETE | /api/employees/:id | employee:delete | employee::delete_employee |
| 19 | PUT | /api/employees/:id/password | employee:password | employee::reset_password |
| 20 | PUT | /api/employees/:id/permissions | employee:edit | employee::update_employee_permissions |
| 21 | GET | /api/permissions | employee:list | system::list_permissions |
| 22 | POST | /api/upload | 登录（图片，100MB 上限） | system::upload |
| 23 | POST | /api/upload/file | chat:upload（100MB 上限） | system::upload_file |
| 24 | GET | /api/system/logs | system:settings | system::logs |
| 25 | PUT | /api/system/settings | system:settings | system::update_settings |
| 26 | GET | /api/chat/conversations | 登录 | chat::list_conversations |
| 27 | GET | /api/chat/conversations/:id/messages | 登录 + 会话成员 | chat::get_messages |
| 28 | POST | /api/chat/conversations/:id/messages | 登录 + 会话成员 | chat::send_message |
| 29 | PUT | /api/chat/conversations/:id/name | 登录 + 群管理员 | chat::update_group_name |
| 30 | POST | /api/chat/conversations/:id/participants | 登录 + 群管理员 | chat::add_participant |
| 31 | PUT | /api/chat/conversations/:id/participants/:target_id | 登录（群：管理员可操作他人/本人；单聊：仅本人） | chat::update_participant |
| 32 | DELETE | /api/chat/conversations/:id/participants/:target_id | 登录 + 群管理员 | chat::remove_participant |
| 33 | DELETE | /api/chat/conversations/:id/disband | 登录 + 群管理员 | chat::disband_group |
| 34 | POST | /api/chat/block | 登录 | chat::block_user |
| 35 | DELETE | /api/chat/block/:id | 登录 | chat::unblock_user |
| 36 | GET | /api/chat/blocked | 登录 | chat::list_blocked |
| 37 | GET | /api/chat/employees | 登录 | chat::list_employees_for_chat |
| 38 | GET | /api/chat/file/:name | 登录 + 相关会话成员 | chat::get_chat_file |
| 39 | PUT | /api/employees/:id/protect-block | chat:protect_block | chat::update_protect_block |

### 静态资源

| 方法 | 路径 | 权限 | 说明 |
| --- | --- | --- | --- |
| GET | /uploads/* | 登录（Bearer 或 Cookie） | 见第五节「静态资源访问」 |

> 路由按 auth / employee / chat / system 模块划分，**不存在** department、role 相关端点。

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

同时 `Set-Cookie`：`manner_token`、`manner_refresh`。

错误场景：`40001`（用户名或密码错误，含账号禁用时统一返回，不区分）、`40009`（节流，429 + Retry-After）。

特殊行为：登录成功后清除该 IP 与该用户名的失败计数；账号 `status ≠ 1`（禁用）拒绝登录。

#### POST /api/auth/register

匿名，仅限**首个管理员**注册（系统尚无任何账号且 `registration_open = '1'`）。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| username | string | 是 | 用户名 |
| password | string | 是 | 密码（至少 8 位） |
| name | string | 是 | 姓名 |
| email | string | 否 | 邮箱 |

成功响应 `data`：与登录 `user` 结构一致（`id/username/name/permissions/avatar/must_change_password`），**不签发会话**，注册后需登录。

特殊行为：

- 事务内 `SELECT ... FOR UPDATE` 锁定注册开关，防并发抢注出多个管理员。
- 成功后自动授予**全部权限码**，并将 `registration_open` 置 `0`。
- 已有账号后调用一律 `403`（`40004`）。

错误场景：`40004`（注册通道已关闭）、`40007`（用户名已存在）、`40005`（密码不足 8 位）。

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
| new_password | string | 是 | 新密码（至少 8 位） |

成功响应 `data`：与登录一致（LoginResponse + 双 Cookie）。

特殊行为：

- 仅 `must_change_password = 1` 的账号可走此流程，否则统一 `40001`。
- 改密成功后 `pwd_version` 递增（此前签发的令牌全部失效）、`must_change_password` 置 0。

错误场景：`40001`（账号不存在/非待激活/禁用/初始密码错误，统一不区分）、`40005`（密码不足 8 位）、`40009`。

#### POST /api/auth/refresh

匿名（需携带 refresh 令牌）。静默续期：用 refresh 令牌换取全新 access + refresh 会话。

- 令牌来源：Cookie `manner_refresh` 或 `Authorization: Bearer`（与 access 令牌双通道读取一致）。
- 校验：签名 → `typ` 必须为 `refresh` → 不在黑名单 → 员工存在、启用且 `pwd_version` 一致。
- 成功后**轮换**：旧 refresh 的 `jti` 立即入黑名单，返回新的 LoginResponse 并覆盖两个 Cookie。

错误场景：`40002`（令牌无效/过期）、`40003`（已被注销或轮换）。

#### POST /api/auth/logout

登录。登出并使会话失效。

- 无请求体。当前 access 令牌的 `jti` 与 refresh 令牌的 `jti`（若有效）同时入 `token_blacklist`。
- 响应清除两个 Cookie（`Max-Age=0`）。

错误场景：`40002`（令牌无效/过期）、`40003`。

#### PUT /api/auth/password

登录。修改自己的密码。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| old_password | string | 是 | 旧密码 |
| new_password | string | 是 | 新密码（至少 8 位） |

特殊行为：成功后 `pwd_version` 递增——**所有已登录会话全部失效**（含本会话，前端将收到 401 引导重新登录），并清除首登改密标记。

错误场景：`40008`（旧密码错误）、`40005`（密码不足 8 位）、`40002`。

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
| items | 员工行数组（id/username/name/title/email/phone/id_number/address/avatar/hire_date/status/protect_block/created_at） |
| total / page / page_size | 总数与分页信息 |

按 `created_at DESC` 排序。错误场景：`40004`。

#### GET /api/employees/:id

权限：employee:view。员工详情。

成功响应 `data`：列表行全部字段 + `permissions`（权限码数组）+ `updated_at`。

错误场景：`40006`（不存在）、`40004`。

#### POST /api/employees

权限：employee:create。创建员工。

请求体（除 username/name 外均可选）：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| username | string | 是 | 用户名（唯一） |
| name | string | 是 | 姓名 |
| title / email / phone / id_number / address | string | 否 | 基本信息 |
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
| avatar | string/null | 头像 URL |
| hire_date | date/null | 可清空 |
| status | int | 1 启用 / 非 1 禁用 |

特殊行为：

- **不能修改自己的资料**（仅 avatar 例外），否则 `40000`「员工管理不能更改自己的资料」。
- `protect_block = 1` 的**保护账号**禁止更新，`40000`「该账号受保护，禁止该操作」（本人改自己的 avatar 不受此限）。
- 无任何字段 → 直接成功。

错误场景：`40000`、`40006`（不存在）、`40004`。

#### DELETE /api/employees/:id

权限：employee:delete。删除员工。

特殊行为：不能删除自己（`40000`「不能删除自己」）；保护账号禁止删除（`40000`「该账号受保护，禁止该操作」）；删除后其 `employee_permissions` 记录随外键 CASCADE 清除。

错误场景：`40000`、`40006`、`40004`。

#### PUT /api/employees/:id/password

权限：employee:password。重置员工密码。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| new_password | string | 是 | 管理员为其设置的新密码 |

特殊行为：不能重置自己（`40000`「不能重置自己的密码，请在个人资料中修改」）；保护账号禁止；成功后 `pwd_version` 递增（该员工所有会话被踢出）且 `must_change_password = 1`（下次登录强制改密）。响应 `message` 为「密码已重置」。

错误场景：`40000`、`40006`、`40004`。

#### PUT /api/employees/:id/permissions

权限：employee:edit。覆盖式更新员工权限。

请求体：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| permission_codes | string[] | 是 | 授权后的完整权限码集合（覆盖式） |

特殊行为：

- 不能修改自己的权限（`40000`「不能修改自己的权限」）。
- 保护账号禁止。
- **新权限必须是操作者自身权限的子集**，否则 `40004`（防受限管理员提权）。
- 事务内先 DELETE 全量删除再逐条重插；响应「权限已更新」。

错误场景：`40000`、`40004`、`40006`。

### 4.3 聊天模块（chat）

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
| content | string | 文本内容 |
| msg_type | string | 默认 text；file 为文件消息 |
| file_url | string | 文件消息必须携带 |
| file_name | string | 原始文件名（≤256 字符） |

特殊行为与错误场景：

- 非成员 → `40004`；会话不存在 → `40006`。
- 单聊：对方拉黑我 → `40000`「对方已拉黑你」。
- 文件消息必须携带 `file_url` → `40000`「文件消息必须携带文件链接」。
- `file_url` 必须以 `/uploads/` 开头且扩展名合法 → `40000`「文件链接必须指向本站上传的文件」（防任意链接传播）。
- `file_name` 超 256 字符 → `40000`「文件名过长」。

成功响应 `data`：`MessageResponse`（同消息列表结构）。

#### PUT /api/chat/conversations/:id/name

登录 + 群管理员。修改群名。

请求体：`{ "name": "新群名" }`。非管理员 → `40004`。响应「已更新」。

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

成功响应 `data`：全量员工（除自己外）`ParticipantInfo[]`，按 `name` 排序（无部门数据范围过滤）。

#### GET /api/chat/file/:name

登录 + 相关会话成员。鉴权下载聊天文件（替代 /uploads 静态直链）。

路径参数：`name` 为文件名（如 `uuid.png`）。

特殊行为（三重校验）：

- 文件名白名单：ASCII 字母数字 + `.` + `-`，长度 ≤64，禁止 `..`、前导点、空名；扩展名必须合法，否则 `40006`。
- 文件必须被某条消息引用（`messages.file_url` 匹配 `/uploads/chat/<name>` 或存量 `/uploads/<name>` 格式），否则 `40006`。
- 当前用户必须是引用该文件的会话的**成员**，否则 `40004`。
- 路径任一级为软链接 → `40006`。

响应：图片扩展名（png/jpg/jpeg/gif/webp/bmp/ico）内联，其余扩展名强制 `Content-Disposition: attachment`；Content-Type 映射（png/jpeg/gif/webp/bmp/ico/txt/md/log/mp4/webm/ogg/mov，缺省 octet-stream）。

#### PUT /api/employees/:id/protect-block

权限：chat:protect_block。设置/取消防拉黑保护。

请求体：`{ "protect_block": 1 }`（int，缺省 0）。

特殊行为：保护账号禁止被删除/禁用/改密/改权/拉黑（见各端点）；本人改自己的头像不受限。员工不存在 → `40006`。响应「已更新」。

### 4.4 系统模块（system）

#### GET /api/system/login-page

匿名。登录页公开配置。

成功响应 `data`（字段存在才返回）：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| login_site_title | string | 登录页标题 |
| login_theme | string | 登录页主题 |
| site_title | string | 网站标题 |
| registration_open | bool | 是否开放注册（`registration_open='1'` 且员工数为 0） |

#### GET /api/system/health

权限：system:settings。健康检查。

成功响应 `data`：`{ "server": "running", "database": "connected" | "disconnected"（3 秒超时探测）, "version": "后端版本号" }`。

#### GET /api/system/settings

权限：system:settings。读取全部系统设置。

成功响应 `data`：`system_settings` 表全部键值（字符串值 Map），如 `chat_upload_limit`、`login_theme`、`site_title`、`login_site_title`、`login_max_failures`、`login_lock_window_secs`。

#### PUT /api/system/settings

权限：system:settings。更新系统设置。

请求体（全部可选）：

| 字段 | 类型 | 校验 |
| --- | --- | --- |
| chat_upload_limit | string | 「无限制」/「禁止」/ 数字+单位（B/KB/MB/GB/TB），非法 → `40005`「上传限制格式不正确（如 10MB / 无限制 / 禁止）」 |
| login_theme / site_title / login_site_title | string | 直接保存 |
| login_max_failures | string | 1~100，非法 → `40005` |
| login_lock_window_secs | string | 1~86400，非法 → `40005` |

特殊行为：变更即写库并**同步内存登录节流器**（无需重启生效）；有变更时写审计日志。响应「保存成功」。

#### GET /api/system/logs

权限：system:settings。读取业务日志（尾 N 行）。

查询参数：`lines`（默认 200）。

成功响应 `data`：`{ "lines": [...], "total": 行数, "file": "日志文件名（仅文件名，不泄露绝对路径）" }`；日志文件不存在 → 空结果。

#### POST /api/upload

登录。上传图片（头像等）。`multipart/form-data`，字段名 `file`，请求体上限 100MB。

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

#### GET /api/permissions

权限：employee:list。权限字典（按模块分组，供授权界面使用）。

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

- 维度：真实 IP（TCP 对端，不信任 `X-Forwarded-For`）+ 用户名双维度。
- 默认阈值：5 次失败 / 900 秒窗口（环境变量兜底，`system_settings` 可动态调整）。
- 超限：429（code `40009`）+ `Retry-After` 头；作用于 `login` / `precheck` / `first-login`。
- 登录成功清除双维度计数；窗口过期自动复位。

### 6.2 登出与 refresh 轮换（F-22）

- 登出：access 与 refresh 的 `jti` 同时入黑名单，并清除 Cookie。
- 每次 refresh 续期：旧 refresh `jti` 立即入黑名单（轮换），旧令牌不可再次使用。

### 6.3 改密全端踢出（F-08）

`pwd_version` 在改密（本人修改、管理员重置、首登激活）时递增；令牌校验要求令牌内 `pwd_version` 与库一致，因此改密后**所有已签发令牌立即失效**。

### 6.4 未知路由统一 401

未注册路径统一回落 401（code `40002`），与匿名访问未认证响应一致，消除路由枚举差分。

### 6.5 前端行为参考

- `credentials: 'include'` 携带 HttpOnly Cookie。
- API 路径自动补 `/api` 前缀。
- 请求 30 秒超时。
- 收到 401 时并发去重：仅一个请求发起 `/api/auth/refresh` 续期，其余等待后重放原请求；刷新失败则登出并跳转登录页（`skipAuthRedirect` 与登录相关端点豁免）。

### 6.6 数据库与权限文档

- 表结构以 [数据库设计.md](./数据库设计.md) 为准（9 张表，无部门/角色表）。
- 权限模型细节见 [权限系统设计.md](./权限系统设计.md)（员工级直接授权，11 个权限码）。
- 前端部署与代理细节见根目录 [README](../README.md) 与 [`nginx-prod.conf.example`](./nginx-prod.conf.example)。
