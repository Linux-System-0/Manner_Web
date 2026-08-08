// 与后端 API 契约对齐的类型定义（以 backend/src/models 源码为准）
// 认证采用 httpOnly Cookie 会话（manner_token / manner_refresh），前端不持有 JWT。

export interface User {
  id: string
  username: string
  name: string
  email: string
  title: string | null
  phone: string | null
  avatar: string | null
  permissions: string[]
  /** F-02: 首次登录强制改密标记（随机初始密码创建的用户为 true） */
  must_change_password?: boolean
}

export interface Employee {
  id: string
  username: string
  name: string
  title: string | null
  email: string
  phone: string
  id_number: string
  address: string
  avatar: string | null
  hire_date: string
  status: number
  protect_block: number
  permissions?: string[]
  created_at: string
}

export interface Permission {
  code: string
  name: string
}

export interface PermissionModule {
  module: string
  module_name: string
  permissions: Permission[]
}

export interface ApiResponse<T = unknown> {
  code: number
  message: string
  data: T
}

export interface PaginatedData<T> {
  items: T[]
  total: number
  page: number
  page_size: number
}

export type PaginatedResponse<T> = ApiResponse<PaginatedData<T>>

export interface LoginRequest {
  username: string
  password: string
}

export interface LoginResponseData {
  token: string
  expires_in: number
  user: User
}

export interface RegisterRequest {
  username: string
  password: string
  name: string
  email: string
}

export interface ChangePasswordRequest {
  old_password: string
  new_password: string
}

export interface CreateEmployeeRequest {
  username: string
  name: string
  title?: string
  email?: string
  phone?: string
  id_number?: string
  address?: string
  hire_date?: string
}

export type UpdateEmployeeRequest = Partial<CreateEmployeeRequest>

export interface EmployeeQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  status?: number
}

export interface PrecheckRequest {
  username: string
}

export interface PrecheckResponse {
  /** true 表示该用户名处于「首次登录待设置密码」状态（must_change_password） */
  must_change: boolean
}

export interface FirstLoginRequest {
  username: string
  /** 当前生效的初始密码（创建员工/重置密码时下发的一次性密码） */
  initial_password: string
  new_password: string
}

// ---- 聊天模块（src/pages/Chat.tsx 提取）----

export interface ChatParticipant {
  id: string
  name: string
  role?: string | null
  nickname?: string | null
  avatar?: string | null
}

export interface Conversation {
  id: string
  type: string
  name: string | null
  created_by: string | null
  created_at: string
  last_message: string | null
  last_time: string | null
  participants: ChatParticipant[]
  my_role: string
  my_nickname: string | null
  my_group_note: string | null
}

export interface ChatMessage {
  id: string
  conversation_id: string
  sender_id: string
  sender_name: string
  sender_avatar?: string | null
  type: string
  content: string | null
  file_url: string | null
  file_name: string | null
  created_at: string
}

// ---- 系统模块 ----

export interface LoginPageInfo {
  login_site_title?: string
  login_theme?: 'light' | 'dark' | 'system'
  site_title?: string
  registration_open?: boolean
}

export interface SystemSettings {
  [key: string]: string | number | boolean | undefined
}

export interface LogEntry {
  line: string
}

export interface LogsData {
  lines: string[]
  total: number
  file: string
}
