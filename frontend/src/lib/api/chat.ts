import { client } from './client'
import type { ApiResponse, ChatMessage, Conversation } from '@/types'

export async function getConversations(): Promise<ApiResponse<Conversation[]>> {
  const res = await client.get<Conversation[]>('/chat/conversations')
  return res
}

export async function createGroupConversation(
  name: string,
  memberIds: string[],
): Promise<ApiResponse<Conversation>> {
  const res = await client.post<Conversation>('/chat/conversations', { name, member_ids: memberIds })
  return res
}

/** 获取或创建与指定员工的单聊会话（同一对用户只保留一个会话） */
export async function getOrCreateDirectConversation(
  peerId: string,
): Promise<ApiResponse<Conversation>> {
  const res = await client.get<Conversation>(`/chat/direct/${peerId}`)
  return res
}

export async function getMessages(
  convId: string,
): Promise<ApiResponse<ChatMessage[]>> {
  const res = await client.get<ChatMessage[]>(`/chat/conversations/${convId}/messages`)
  return res
}

export async function sendMessage(
  convId: string,
  payload: {
    content?: string
    msg_type: string
    file_url?: string
    file_name?: string
  },
): Promise<ApiResponse<ChatMessage>> {
  const res = await client.post<ChatMessage>(`/chat/conversations/${convId}/messages`, payload)
  return res
}

export async function updateConversationName(
  convId: string,
  name: string,
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(`/chat/conversations/${convId}/name`, { name })
  return res
}

export async function updateParticipant(
  convId: string,
  participantId: string,
  data: { nickname?: string; group_note?: string; role?: string },
): Promise<ApiResponse<null>> {
  const res = await client.put<null>(
    `/chat/conversations/${convId}/participants/${participantId}`,
    data,
  )
  return res
}

export async function removeParticipant(
  convId: string,
  participantId: string,
): Promise<ApiResponse<null>> {
  const res = await client.delete<null>(
    `/chat/conversations/${convId}/participants/${participantId}`,
  )
  return res
}

export async function addParticipant(
  convId: string,
  employeeId: string,
): Promise<ApiResponse<null>> {
  const res = await client.post<null>(
    `/chat/conversations/${convId}/participants`,
    { employee_id: employeeId },
  )
  return res
}

export async function disbandConversation(
  convId: string,
): Promise<ApiResponse<null>> {
  // 后端路由：DELETE /api/chat/conversations/:id/disband
  const res = await client.delete<null>(`/chat/conversations/${convId}/disband`)
  return res
}

export async function getBlocked(): Promise<ApiResponse<Array<{ id: string; name: string }>>> {
  const res = await client.get<Array<{ id: string; name: string }>>('/chat/blocked')
  return res
}

export async function blockUser(blockedId: string): Promise<ApiResponse<null>> {
  const res = await client.post<null>('/chat/block', { blocked_id: blockedId })
  return res
}

export async function unblockUser(blockedId: string): Promise<ApiResponse<null>> {
  const res = await client.delete<null>(`/chat/block/${blockedId}`)
  return res
}

export async function getChatEmployees(): Promise<
  ApiResponse<Array<{ id: string; name: string }>>
> {
  const res = await client.get<Array<{ id: string; name: string }>>('/chat/employees')
  return res
}

/** 上传任意文件（权限 chat:upload），返回 /uploads/<uuid> 相对路径字符串 */
export async function uploadChatFile(file: File): Promise<string> {
  const formData = new FormData()
  formData.append('file', file)
  const res = await client.upload<unknown>('/upload/file', formData)
  const uploadJson = res as { data?: unknown; message?: string }
  if (!uploadJson.data) throw new Error(uploadJson.message || '上传失败')
  return uploadJson.data as string
}
