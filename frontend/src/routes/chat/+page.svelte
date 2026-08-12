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
  // 聊天页（复刻原 frontend/src/pages/Chat.tsx，Svelte 5 runes 版）
  // - 会话列表 5s 增量轮询、消息 3s 增量轮询（setInterval + onDestroy/$effect cleanup）
  // - 时间显示统一 formatTime(iso, getGlobalPrefs())；新会话插入位置取偏好 newConvPosition
  // - 文件上传：uploadChatFile(file) → sendMessage(msg_type:'file')
  import { onMount, onDestroy } from 'svelte'
  import { page } from '$app/stores'
  import { authStore } from '$lib/stores/auth'
  import { formatTime, getGlobalPrefs, subscribe as subscribePrefs, preferencesStore } from '$lib/stores/preferences'
  import { t } from '$lib/i18n'
  import { getMe } from '$lib/api/auth'
  import {
    getConversations,
    createGroupConversation,
    getMessages,
    sendMessage,
    updateConversationName,
    updateParticipant,
    removeParticipant,
    addParticipant,
    getBlocked,
    blockUser,
    unblockUser,
    getChatEmployees,
    uploadChatFile,
  } from '$lib/api/chat'
  import { client, getApiError } from '$lib/api/client'
  import { message } from '$lib/components/message'
  import Card from '$lib/components/Card.svelte'
  import Button from '$lib/components/Button.svelte'
  import Input from '$lib/components/Input.svelte'
  import Avatar from '$lib/components/Avatar.svelte'
  import Badge from '$lib/components/Badge.svelte'
  import Modal from '$lib/components/Modal.svelte'
  import Select from '$lib/components/Select.svelte'
  import Tag from '$lib/components/Tag.svelte'
  import Popconfirm from '$lib/components/Popconfirm.svelte'
  import Empty from '$lib/components/Empty.svelte'
  import Spin from '$lib/components/Spin.svelte'
  import List from '$lib/components/List.svelte'
  import Tooltip from '$lib/components/Tooltip.svelte'
  import Text from '$lib/components/Text.svelte'
  import { Icon } from '$lib/icons'
  import type { Conversation, ChatMessage } from '$lib/types'

  // ---- 会话列表 ----
  let conversations = $state<Conversation[]>([])
  let loading = $state(false)
  let selectedConv = $state<string | null>(null)
  let blockedUsers = $state<Array<{ id: string; name: string }>>([])
  let reorderMode = $state(false)
  let dragIndex = $state<number | null>(null)
  let manualOrder = $state<string[]>([])
  let chatCollapsed = $state(false)

  // ---- 消息区 ----
  let messages = $state<ChatMessage[]>([])
  let msgLoading = $state(false)
  let inputText = $state('')
  // 普通变量而非 $state：避免被消息轮询 $effect 同步读取后形成「effect 重跑」环
  let firstMessagesLoad = true
  let dragCounter = 0
  let dragOver = $state(false)
  let messagesContainerEl = $state<HTMLDivElement | undefined>()
  let messagesEndEl = $state<HTMLDivElement | undefined>()
  let fileInputEl = $state<HTMLInputElement | undefined>()
  let textareaEl = $state<HTMLTextAreaElement | undefined>()

  let allEmployees = $state<Array<{ id: string; name: string }>>([])

  // ---- 创建群聊 ----
  let createGroupModal = $state(false)
  let createGroupName = $state('')
  let createGroupMembers = $state<string[]>([])

  // ---- 黑名单 ----
  let blockModal = $state(false)

  // ---- 聊天设置 ----
  let settingsConv = $state<Conversation | null>(null)
  let settingsOpen = $state(false)
  let myNickname = $state('')
  let groupNote = $state('')
  let newGroupName = $state('')
  let addUserModal = $state(false)
  let addUserId = $state<string | null>(null)

  // ---- 偏好（时间显示 / 新会话位置；轮询场景用 getGlobalPrefs 而非响应式订阅） ----
  let prefsVersion = $state(0)
  let prefs = $derived.by(() => {
    void prefsVersion
    return getGlobalPrefs()
  })

  // URL 参数 conv 初始化选中会话
  const convParam = $page.url.searchParams.get('conv')
  if (convParam) selectedConv = convParam

  let convTimer: ReturnType<typeof setInterval> | null = null
  let prefsUnsub: (() => void) | null = null

  // ---- 数据获取 ----

  let identitySyncing = false

  /** 服务端返回的身份(my_id)与本地 authStore 不一致时，以 /auth/me 为准同步本地登录态，
   *  解决同浏览器登录第二个账号后「界面显示 A、实际以 B 身份操作」的错乱。 */
  async function reconcileIdentity(myId?: string) {
    if (!myId) return
    if (myId === $authStore.user?.id) return
    if (identitySyncing) return
    identitySyncing = true
    try {
      const res = await getMe()
      if (res.code === 0 && res.data) {
        authStore.setUser(res.data)
        await preferencesStore.refresh()
      }
    } catch {
      /* client 已处理 401 */
    } finally {
      identitySyncing = false
    }
  }

  async function fetchConversations() {
    loading = true
    try {
      const res = await getConversations()
      if (res.code === 0 && res.data) {
        conversations = res.data
        reconcileIdentity(res.data[0]?.my_id)
      }
    } catch {
      /* ignore */
    }
    loading = false
  }

  /** 5s 增量更新：已存在会话就地更新，新会话追加（保持原 React 版语义） */
  async function fetchConversationsIncremental() {
    try {
      const res = await getConversations()
      if (res.code !== 0 || !res.data) return
      const newConvs: Conversation[] = res.data
      reconcileIdentity(newConvs[0]?.my_id)
      const existingIds = new Set(conversations.map((c) => c.id))
      const append = newConvs.filter((c) => !existingIds.has(c.id))
      const updated = conversations.map((c) => newConvs.find((n) => n.id === c.id) || c)
      if (append.length > 0) conversations = [...updated, ...append]
      else if (newConvs.length !== updated.length) conversations = updated
      else {
        // 保持引用稳定（避免无谓重渲染），仅在内容变化时更新
        const changed = updated.some(
          (c, i) => c.last_message !== conversations[i]?.last_message || c.last_time !== conversations[i]?.last_time,
        )
        if (changed) conversations = updated
      }
    } catch {
      /* ignore */
    }
  }

  async function fetchMessages(convId: string) {
    if (firstMessagesLoad) {
      msgLoading = true
      try {
        const res = await getMessages(convId)
        if (res.code === 0 && res.data) messages = res.data
      } catch {
        /* ignore */
      }
      msgLoading = false
      firstMessagesLoad = false
      return
    }
    try {
      const res = await getMessages(convId)
      if (res.code !== 0 || !res.data) return
      const newMsgs: ChatMessage[] = res.data
      const existingIds = new Set(messages.map((m) => m.id))
      const append = newMsgs.filter((m) => !existingIds.has(m.id))
      if (append.length > 0) messages = [...messages, ...append]
    } catch {
      /* ignore */
    }
  }

  async function fetchBlocked() {
    try {
      const res = await getBlocked()
      if (res.code === 0 && res.data) blockedUsers = res.data
    } catch {
      /* ignore */
    }
  }

  // ---- 轮询 ----

  onMount(() => {
    loadManualOrder()
    void fetchConversations()
    void fetchBlocked()
    convTimer = setInterval(() => void fetchConversationsIncremental(), 5000)
    prefsUnsub = subscribePrefs(() => {
      prefsVersion += 1
    })
  })

  onDestroy(() => {
    if (convTimer) clearInterval(convTimer)
    prefsUnsub?.()
  })

  // 消息 3s 轮询：切换会话时重建定时器
  $effect(() => {
    const convId = selectedConv
    if (!convId) return
    firstMessagesLoad = true
    void fetchMessages(convId)
    const timer = setInterval(() => void fetchMessages(convId), 3000)
    return () => clearInterval(timer)
  })

  // 消息更新时若接近底部则平滑滚动到底部
  $effect(() => {
    const container = messagesContainerEl
    const end = messagesEndEl
    if (!container || !end) return
    void messages.length
    const isAtBottom = container.scrollHeight - container.scrollTop - container.clientHeight < 80
    if (isAtBottom) end.scrollIntoView({ behavior: 'smooth' })
  })

  // 会话顺序持久化（原 manner-chat-order-<userId> 键名）
  function loadManualOrder() {
    try {
      const id = $authStore.user?.id
      if (!id) return
      const raw = localStorage.getItem(`manner-chat-order-${id}`)
      if (raw) manualOrder = JSON.parse(raw) as string[]
    } catch {
      /* ignore */
    }
  }

  $effect(() => {
    const id = $authStore.user?.id
    const order = manualOrder
    if (!id) return
    try {
      localStorage.setItem(`manner-chat-order-${id}`, JSON.stringify(order))
    } catch {
      /* ignore */
    }
  })

  // ---- 发送 ----

  async function handleSend() {
    if (!selectedConv || !inputText.trim()) return
    const text = inputText.trim()
    inputText = ''
    try {
      const res = await sendMessage(selectedConv, { content: text, msg_type: 'text' })
      if (res.code !== 0 || !res.data) {
        message.error(res.message || t('chat.sendFailed'))
        return
      }
      messages = [...messages, res.data]
      await fetchConversations()
    } catch (err) {
      message.error(getApiError(err, t('chat.sendFailed')))
    }
  }

  async function sendFile(file: File) {
    if (!selectedConv) return
    try {
      const fileUrl = await uploadChatFile(file)
      const res = await sendMessage(selectedConv, {
        content: file.name,
        msg_type: 'file',
        file_url: fileUrl,
        file_name: file.name,
      })
      if (res.code !== 0 || !res.data) {
        message.error(res.message || t('chat.sendFileFailed'))
        return
      }
      messages = [...messages, res.data]
      await fetchConversations()
    } catch (err) {
      message.error((err as Error).message || getApiError(err, t('chat.sendFileFailed')))
    }
  }

  function handleFileSend(e: Event) {
    const input = e.target as HTMLInputElement
    const file = input.files?.[0]
    input.value = ''
    if (file) void sendFile(file)
  }

  function onTextareaInput(e: Event) {
    const ta = e.target as HTMLTextAreaElement
    inputText = ta.value
    autoGrow(ta)
  }

  function autoGrow(ta: HTMLTextAreaElement) {
    ta.style.height = 'auto'
    ta.style.height = Math.min(Math.max(ta.scrollHeight, 32), 96) + 'px'
  }

  function onTextareaKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      void handleSend()
    }
  }

  $effect(() => {
    if (inputText === '' && textareaEl) textareaEl.style.height = '32px'
  })

  // ---- 拉黑 / 取消拉黑 ----

  function isBlocked(userId: string): boolean {
    return blockedUsers.some((b) => b.id === userId)
  }

  async function handleBlock(blockedId: string) {
    if (!blockedId) return
    try {
      const res = await blockUser(blockedId)
      if (res.code !== 0) {
        message.error(res.message || t('common.operationFailed'))
        return
      }
      message.success(t('chat.blocked'))
      await fetchBlocked()
      settingsOpen = false
      const conv = conversations.find((c) => {
        if (c.type !== 'single') return false
        const other = c.participants.find((p) => p.id !== selfIdOf(c))
        return other?.id === blockedId && c.id === selectedConv
      })
      if (conv) selectedConv = null
    } catch (err) {
      message.error(getApiError(err, t('common.operationFailed')))
    }
  }

  async function handleUnblock(blockedId: string) {
    if (!blockedId) return
    try {
      const res = await unblockUser(blockedId)
      if (res.code !== 0) {
        message.error(res.message || t('common.operationFailed'))
        return
      }
      message.success(t('chat.unblocked'))
      await fetchBlocked()
      const conv = conversations.find((c) => {
        if (c.type !== 'single') return false
        const other = c.participants.find((p) => p.id !== selfIdOf(c))
        return other?.id === blockedId && c.id === selectedConv
      })
      if (conv) selectedConv = null
    } catch (err) {
      message.error(getApiError(err, t('common.operationFailed')))
    }
  }

  // ---- 聊天设置 ----

  async function openSettings(conv: Conversation) {
    settingsConv = conv
    myNickname = conv.type === 'single' ? conv.my_group_note || '' : conv.my_nickname || ''
    groupNote = conv.my_group_note || ''
    newGroupName = conv.name || ''
    settingsOpen = true
  }

  const otherUserId = $derived(
    settingsConv?.type === 'single'
      ? settingsConv.participants.find((p) => p.id !== selfIdOf(settingsConv))?.id || ''
      : '',
  )

  async function save1v1Remark() {
    if (!settingsConv) return
    try {
      // 备注仅自己可见：写入当前用户自己的 group_note
      const res = await updateParticipant(settingsConv.id, $authStore.user?.id || '', { group_note: myNickname })
      if (res.code !== 0) {
        message.error(res.message || t('common.operationFailed'))
        return
      }
      message.success(t('chat.remarkUpdated'))
      await fetchConversations()
    } catch (err) {
      message.error(getApiError(err, t('common.operationFailed')))
    }
  }

  async function saveGroupName() {
    if (!settingsConv || !newGroupName.trim()) return
    try {
      const res = await updateConversationName(settingsConv.id, newGroupName)
      if (res.code !== 0) {
        message.error(res.message || t('common.operationFailed'))
        return
      }
      message.success(t('chat.groupNameUpdated'))
      await fetchConversations()
    } catch (err) {
      message.error(getApiError(err, t('common.operationFailed')))
    }
  }

  async function saveMyNickname() {
    if (!settingsConv) return
    try {
      const res = await updateParticipant(settingsConv.id, $authStore.user?.id || '', { nickname: myNickname || '' })
      if (res.code !== 0) {
        message.error(res.message || t('common.operationFailed'))
        return
      }
      message.success(t('chat.nicknameUpdated'))
      await fetchConversations()
    } catch (err) {
      message.error(getApiError(err, t('common.operationFailed')))
    }
  }

  async function saveGroupNote() {
    if (!settingsConv) return
    try {
      const res = await updateParticipant(settingsConv.id, $authStore.user?.id || '', { group_note: groupNote || '' })
      if (res.code !== 0) {
        message.error(res.message || t('common.operationFailed'))
        return
      }
      message.success(t('chat.groupNoteUpdated'))
      await fetchConversations()
    } catch (err) {
      message.error(getApiError(err, t('common.operationFailed')))
    }
  }

  async function disbandGroup() {
    if (!settingsConv) return
    try {
      // 注意：$lib/api/chat.ts 的 disbandConversation 封装缺少 /disband 后缀（端点错误），
      // 此处直接用 client 调用后端正确路由 DELETE /chat/conversations/:id/disband。
      const res = await client.delete<null>(`/chat/conversations/${settingsConv.id}/disband`)
      if (res.code !== 0) {
        message.error(res.message || t('common.operationFailed'))
        return
      }
      message.success(t('chat.disbanded'))
      settingsOpen = false
      if (selectedConv === settingsConv.id) selectedConv = null
      await fetchConversations()
    } catch (err) {
      message.error(getApiError(err, t('common.operationFailed')))
    }
  }

  async function refreshSettings() {
    if (!settingsConv) return
    try {
      const res = await getConversations()
      if (res.code !== 0 || !res.data) return
      conversations = res.data
      const updated = res.data.find((c) => c.id === settingsConv?.id)
      if (updated) {
        settingsConv = updated
        myNickname = updated.type === 'single' ? updated.my_group_note || '' : updated.my_nickname || ''
        groupNote = updated.my_group_note || ''
        newGroupName = updated.name || ''
      }
    } catch {
      /* ignore */
    }
  }

  async function changeRole(targetId: string, role: string) {
    if (!settingsConv) return
    try {
      const res = await updateParticipant(settingsConv.id, targetId, { role })
      if (res.code !== 0) {
        message.error(res.message || t('common.operationFailed'))
        return
      }
      message.success(t('chat.roleUpdated'))
      await refreshSettings()
    } catch (err) {
      message.error(getApiError(err, t('common.operationFailed')))
    }
  }

  async function removeMember(targetId: string) {
    if (!settingsConv) return
    try {
      const res = await removeParticipant(settingsConv.id, targetId)
      if (res.code !== 0) {
        message.error(res.message || t('common.operationFailed'))
        return
      }
      message.success(t('chat.memberRemoved'))
      await refreshSettings()
    } catch (err) {
      message.error(getApiError(err, t('common.operationFailed')))
    }
  }

  async function openAddUser() {
    addUserModal = true
    try {
      const res = await getChatEmployees()
      if (res.code === 0 && res.data) allEmployees = res.data
    } catch {
      /* ignore */
    }
  }

  async function addMember() {
    if (!addUserId || !settingsConv) return
    try {
      const res = await addParticipant(settingsConv.id, addUserId)
      if (res.code !== 0) {
        message.error(res.message || t('common.operationFailed'))
        return
      }
      message.success(t('chat.addSuccess'))
      addUserModal = false
      addUserId = null
      await refreshSettings()
    } catch (err) {
      message.error(getApiError(err, t('common.operationFailed')))
    }
  }

  async function openCreateGroup() {
    createGroupName = ''
    createGroupMembers = []
    createGroupModal = true
    try {
      const res = await getChatEmployees()
      if (res.code === 0 && res.data) allEmployees = res.data
    } catch {
      /* ignore */
    }
  }

  async function createGroup() {
    if (!createGroupName.trim()) {
      message.error(t('chat.errGroupName'))
      return
    }
    if (createGroupMembers.length === 0) {
      message.error(t('chat.errMember'))
      return
    }
    try {
      const res = await createGroupConversation(createGroupName.trim(), createGroupMembers)
      if (res.code !== 0 || !res.data) {
        message.error(res.message || t('common.createdFailed'))
        return
      }
      message.success(t('chat.created'))
      createGroupModal = false
      await fetchConversations()
      selectedConv = res.data.id
    } catch (err) {
      message.error(getApiError(err, t('common.createdFailed')))
    }
  }

  // ---- 展示辅助 ----

  /** 服务端认证的当前用户 id：优先取接口返回的 my_id（与后端认证身份一致），
   *  兜底用本地 authStore（未携带 my_id 的旧响应 / 创建会话等场景）。 */
  function selfIdOf(item: { my_id?: string } | null | undefined): string | undefined {
    return item?.my_id || $authStore.user?.id
  }

  function getConvName(conv: Conversation): string {
    if (conv.type === 'group') return conv.my_group_note || conv.name || t('chat.group')
    const other = conv.participants.find((p) => p.id !== selfIdOf(conv))
    const otherInfo = conv.participants.find((p) => p.id === other?.id)
    return conv.my_group_note || otherInfo?.nickname || other?.name || t('chat.unknown')
  }

  function getOtherAvatar(conv: Conversation): string | undefined {
    if (conv.type === 'group') return undefined
    const other = conv.participants.find((p) => p.id !== selfIdOf(conv))
    return other?.avatar || undefined
  }

  const filteredConversations = $derived(
    conversations.filter((conv) => {
      if (conv.type === 'single') {
        const other = conv.participants.find((p) => p.id !== selfIdOf(conv))
        return !!other && !isBlocked(other.id)
      }
      return true
    }),
  )

  const orderedConversations = $derived.by(() => {
    void prefsVersion
    const newFirst = getGlobalPrefs().newConvPosition === 'first'
    const list = [...filteredConversations]
    if (manualOrder.length === 0) {
      const newOnes = list.filter((c) => !c.last_time)
      const others = list.filter((c) => c.last_time)
      return newFirst ? [...newOnes, ...others] : [...others, ...newOnes]
    }
    const pos = new Map(manualOrder.map((id, i) => [id, i]))
    return list.sort((a, b) => {
      const ai = pos.get(a.id)
      const bi = pos.get(b.id)
      if (ai === undefined && bi === undefined) return 0
      if (ai === undefined) return newFirst ? -1 : 1
      if (bi === undefined) return newFirst ? 1 : -1
      return ai - bi
    })
  })

  const selectedConvData = $derived(filteredConversations.find((c) => c.id === selectedConv))

  function handleDragStart(index: number) {
    dragIndex = index
  }

  function handleDrop(targetIndex: number) {
    if (dragIndex === null || dragIndex === targetIndex) {
      dragIndex = null
      return
    }
    const next = [...orderedConversations]
    const [moved] = next.splice(dragIndex, 1)
    next.splice(targetIndex, 0, moved)
    manualOrder = next.map((c) => c.id)
    dragIndex = null
  }

  // ---- 拖拽发送文件 ----

  function handleDragEnter(e: DragEvent) {
    if (!selectedConv) return
    e.preventDefault()
    dragCounter += 1
    dragOver = true
  }

  function handleDragOver(e: DragEvent) {
    if (!selectedConv) return
    e.preventDefault()
  }

  function handleDragLeave() {
    dragCounter -= 1
    if (dragCounter <= 0) {
      dragCounter = 0
      dragOver = false
    }
  }

  function handleDropFiles(e: DragEvent) {
    e.preventDefault()
    dragCounter = 0
    dragOver = false
    if (!selectedConv) return
    const files = Array.from(e.dataTransfer?.files || [])
    files.forEach((f) => void sendFile(f))
  }
</script>

<div style="display:flex;height:100%;gap:16px;overflow:hidden">
  <!-- 左侧会话列表（可折叠宽度） -->
  {#if chatCollapsed}
    <div
      style="width:44px;flex-shrink:0;display:flex;flex-direction:column;align-items:center;padding-top:12px;transition:width 0.2s"
    >
      <Tooltip title={t('chat.expandList')}>
        <Button type="text" onClick={() => (chatCollapsed = false)}>
          {#snippet icon()}<Icon name="right" style="font-size:16px" />{/snippet}
        </Button>
      </Tooltip>
    </div>
  {:else}
    <div style="width:300px;flex-shrink:0;transition:width 0.2s">
      <Card
        style="width:100%;display:flex;flex-direction:column;height:100%"
        bodyStyle="flex:1;overflow:auto;padding:12px"
      >
        {#snippet title()}
          <div style="display:flex;justify-content:space-between;align-items:center;width:100%">
            <span>{t('chat.conversations')}</span>
            <div style="display:flex;gap:4px;align-items:center">
              <Tooltip title={t('chat.blockList')}>
                <Button
                  size="small"
                  onClick={() => {
                    void fetchBlocked()
                    blockModal = true
                  }}
                >
                  {#snippet icon()}<Icon name="stop" />{/snippet}
                </Button>
              </Tooltip>
              {#if $authStore.permissions.includes('chat:group_create')}
                <Tooltip title={t('chat.createGroup')}>
                  <Button size="small" onClick={openCreateGroup}>
                    {#snippet icon()}<Icon name="plus" />{/snippet}
                  </Button>
                </Tooltip>
              {/if}
              {#if reorderMode}
                <Tooltip title={t('chat.finishSort')}>
                  <Button size="small" type="primary" onClick={() => (reorderMode = false)}>
                    {#snippet icon()}<Icon name="check" />{/snippet}
                  </Button>
                </Tooltip>
              {:else}
                <Tooltip title={t('chat.adjustOrder')}>
                  <Button size="small" onClick={() => (reorderMode = true)}>
                    {#snippet icon()}<Icon name="holder" />{/snippet}
                  </Button>
                </Tooltip>
              {/if}
              <Tooltip title={t('chat.collapseList')}>
                <Button size="small" onClick={() => (chatCollapsed = true)}>
                  {#snippet icon()}<Icon name="menu-fold" />{/snippet}
                </Button>
              </Tooltip>
            </div>
          </div>
        {/snippet}

        {#if reorderMode}
          <div style="font-size:12px;color:#999;padding:0 4px 8px">{t('chat.reorderHint')}</div>
        {/if}

        {#if loading}
          <div style="text-align:center;padding:40px 0"><Spin /></div>
        {:else}
          <List hasData={orderedConversations.length > 0} emptyText={t('chat.noConversations')}>
            {#each orderedConversations as conv, index (conv.id)}
              <!-- 会话行可点击/可拖拽排序：li 按按钮语义暴露，豁免 a11y 静态检查 -->
              <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role, a11y_no_noninteractive_tabindex -->
              <li
                class="ant-list-item"
                role="button"
                tabindex={0}
                draggable={reorderMode}
                ondragstart={() => reorderMode && handleDragStart(index)}
                ondragover={(e) => reorderMode && e.preventDefault()}
                ondrop={() => reorderMode && handleDrop(index)}
                ondragend={() => (dragIndex = null)}
                onclick={() => (selectedConv = conv.id)}
                onkeydown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') {
                    e.preventDefault()
                    selectedConv = conv.id
                  }
                }}
                style="display:flex;align-items:center;gap:12px;padding:8px 12px;border-radius:6px;margin-bottom:2px;cursor:{reorderMode ? 'grab' : 'pointer'};background:{selectedConv === conv.id ? 'var(--chat-selected-bg)' : 'transparent'};opacity:{dragIndex === index ? 0.5 : 1};transition:background 0.2s"
              >
                <Badge dot={false}>
                  <Avatar
                    src={getOtherAvatar(conv)}
                    style="background:{conv.type === 'group' ? '#52c41a' : '#1677ff'};flex-shrink:0"
                  >
                    <span style="display:inline-flex">
                      <Icon name={conv.type === 'group' ? 'team' : 'user'} />
                    </span>
                  </Avatar>
                </Badge>
                <div style="flex:1;min-width:0">
                  <Text style="font-size:13px;font-weight:600">{getConvName(conv)}</Text>
                  <Text type="secondary" ellipsis={true} style="font-size:12px;margin-top:2px">
                    {conv.last_message || t('chat.noMessages')}
                  </Text>
                </div>
                {#if conv.last_time}
                  <span style="flex-shrink:0;font-size:10px;color:#bbb">{formatTime(conv.last_time, prefs)}</span>
                {/if}
                {#if reorderMode}
                  <span style="display:inline-flex;color:#bbb;flex-shrink:0"><Icon name="holder" /></span>
                {/if}
              </li>
            {/each}
          </List>
        {/if}
      </Card>
    </div>
  {/if}

  <!-- 右侧消息区 -->
  <Card style="flex:1;display:flex;flex-direction:column;min-width:0;min-height:0" bodyStyle="flex:1;display:flex;flex-direction:column;padding:0;min-height:0">
    {#snippet title()}
      {#if selectedConvData}
        <div style="display:flex;justify-content:space-between;align-items:center">
          <div style="display:flex;align-items:center;gap:12px;min-width:0">
            <Avatar
              src={getOtherAvatar(selectedConvData)}
              style="background:{selectedConvData.type === 'group' ? '#52c41a' : '#1677ff'};flex-shrink:0"
            >
              <span style="display:inline-flex">
                <Icon name={selectedConvData.type === 'group' ? 'team' : 'user'} />
              </span>
            </Avatar>
            <span style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{getConvName(selectedConvData)}</span>
            {#if selectedConvData.type === 'group'}
              <Text type="secondary" style="font-size:12px">{t('chat.peopleCount', { count: selectedConvData.participants.length })}</Text>
            {/if}
          </div>
          <Button type="text" tooltip={t('chat.settingsTooltip')} onClick={() => openSettings(selectedConvData)}>
            {#snippet icon()}<Icon name="setting" />{/snippet}
          </Button>
        </div>
      {:else}
        {t('chat.messages')}
      {/if}
    {/snippet}

    {#if selectedConv}
      <div
        bind:this={messagesContainerEl}
        role="region"
        aria-label={t('chat.messageAria')}
        style="flex:1;overflow:auto;padding:16px;position:relative;min-height:0"
        ondragenter={handleDragEnter}
        ondragover={handleDragOver}
        ondragleave={handleDragLeave}
        ondrop={handleDropFiles}
      >
        {#if dragOver}
          <div
            style="position:absolute;inset:8px;border:2px dashed #1677ff;border-radius:8px;background:rgba(22,119,255,0.08);display:flex;align-items:center;justify-content:center;pointer-events:none;z-index:10"
          >
            <span style="font-size:16px;color:#1677ff">{t('chat.releaseToSend')}</span>
          </div>
        {/if}
        {#if msgLoading}
          <div style="text-align:center;padding:40px 0"><Spin /></div>
        {:else if messages.length === 0}
          <div style="text-align:center;color:#999;margin-top:60px">{t('chat.noMessages')}</div>
        {:else}
          {#each messages as msg (msg.id)}
            {@const isMe = msg.sender_id === (msg.my_id || $authStore.user?.id)}
            {@const name = msg.file_name || msg.content || ''}
            {@const safeFileUrl =
              typeof msg.file_url === 'string' && msg.file_url.startsWith('/uploads/chat/')
                ? '/api/chat/file/' + msg.file_url.slice('/uploads/chat/'.length)
                : typeof msg.file_url === 'string' && msg.file_url.startsWith('/uploads/')
                  ? '/api/chat/file/' + msg.file_url.slice('/uploads/'.length)
                  : null}
            {@const ext = name.split('.').pop()?.toLowerCase() || ''}
            {@const canPreview = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'mp4', 'webm', 'ogg', 'mov', 'txt', 'md', 'log'].includes(ext)}
            <div style="display:flex;flex-direction:{isMe ? 'row-reverse' : 'row'};margin-bottom:12px;gap:8px">
              <Avatar size={32} src={msg.sender_avatar} style="background:{isMe ? '#1677ff' : '#52c41a'};flex-shrink:0">
                <span style="display:inline-flex"><Icon name="user" /></span>
              </Avatar>
              <div style="max-width:60%">
                <div style="text-align:{isMe ? 'right' : 'left'};font-size:11px;color:#999;margin-bottom:2px">
                  {isMe ? t('chat.me') : msg.sender_name}
                </div>
                <div
                  style="background:{isMe ? '#1677ff' : 'var(--chat-msg-other-bg)'};color:{isMe ? '#fff' : 'var(--chat-msg-other-color)'};padding:6px 12px;border-radius:12px;border-bottom-right-radius:{isMe ? 4 : 12}px;border-bottom-left-radius:{isMe ? 12 : 4}px;font-size:13px;word-break:break-word"
                >
                  {#if msg.type === 'file'}
                    {#if safeFileUrl}
                      <a
                        href={safeFileUrl}
                        target={canPreview ? '_blank' : undefined}
                        rel={canPreview ? 'noreferrer' : undefined}
                        download={!canPreview ? name : undefined}
                        style="color:{isMe ? '#fff' : '#1677ff'}"
                      >
                        <Icon name="paper-clip" /> {name}
                      </a>
                    {:else}
                      <span style="color:{isMe ? '#fff' : '#1677ff'}"><Icon name="paper-clip" /> {name} ({t('chat.invalidFileLink')})</span>
                    {/if}
                  {:else}
                    {msg.content}
                  {/if}
                </div>
                <div style="text-align:{isMe ? 'right' : 'left'};font-size:10px;color:#bbb;margin-top:2px">
                  {formatTime(msg.created_at, prefs)}
                </div>
              </div>
            </div>
          {/each}
        {/if}
        <div bind:this={messagesEndEl}></div>
      </div>

      <div style="padding:8px 16px;border-top:1px solid var(--chat-border-color)">
        <div style="display:flex;gap:8px;align-items:flex-end">
          <Button tooltip={t('chat.fileTooltip')} onClick={() => fileInputEl?.click()}>
            {#snippet icon()}<Icon name="paper-clip" />{/snippet}
          </Button>
          <input bind:this={fileInputEl} type="file" style="display:none" onchange={handleFileSend} />
          <textarea
            bind:this={textareaEl}
            class="chat-input"
            value={inputText}
            placeholder={t('chat.inputPlaceholder')}
            rows={1}
            oninput={onTextareaInput}
            onkeydown={onTextareaKeydown}
          ></textarea>
          <Button type="primary" tooltip={t('chat.sendTooltip')} onClick={handleSend}>
            {#snippet icon()}<Icon name="send" />{/snippet}{t('chat.send')}
          </Button>
        </div>
      </div>
    {:else}
      <div style="text-align:center;color:#999;margin-top:120px">
        <Icon name="message" style="font-size:48px;margin-bottom:16px" />
        <div>{t('chat.selectConv')}</div>
      </div>
    {/if}
  </Card>
</div>

<!-- 黑名单 -->
<Modal title={t('chat.blockModalTitle')} open={blockModal} onclose={() => (blockModal = false)}>
  {#if blockedUsers.length === 0}
    <Empty description={t('chat.noBlocked')} />
  {:else}
    <ul style="list-style:none;margin:0;padding:0">
      {#each blockedUsers as item (item.id)}
        <li
          class="ant-list-item"
          style="display:flex;align-items:center;justify-content:space-between;gap:12px;padding:12px 16px;border-bottom:1px solid var(--ant-list-item-border)"
        >
          <div style="display:flex;align-items:center;gap:12px">
            <Avatar><span style="display:inline-flex"><Icon name="user" /></span></Avatar>
            <span>{item.name}</span>
          </div>
          <Button size="small" tooltip={t('chat.unblockTooltip')} onClick={() => handleUnblock(item.id)}>{t('chat.unblock')}</Button>
        </li>
      {/each}
    </ul>
  {/if}
</Modal>

<!-- 创建群聊 -->
<Modal
  title={t('chat.createGroupModalTitle')}
  open={createGroupModal}
  onOk={createGroup}
  onclose={() => (createGroupModal = false)}
  okText={t('chat.createGroup')}
  cancelText={t('common.cancel')}
  width={480}
>
  <div style="display:flex;flex-direction:column;gap:16px">
    <div>
      <div style="font-weight:600;margin-bottom:8px">{t('chat.groupNameLabel')}</div>
      <Input
        value={createGroupName}
        onInput={(v) => (createGroupName = v)}
        placeholder={t('chat.groupNamePlaceholder')}
        maxlength={128}
      />
    </div>
    <div>
      <div style="font-weight:600;margin-bottom:8px">{t('chat.membersLabel')}</div>
      <Select
        multiple
        placeholder={t('chat.memberSelectPlaceholder')}
        value={createGroupMembers}
        onChange={(v) => (createGroupMembers = Array.isArray(v) ? v.map(String) : [])}
        options={allEmployees.map((e) => ({ value: e.id, label: e.name }))}
      />
    </div>
  </div>
</Modal>

<!-- 聊天设置 -->
<Modal title={t('chat.settings')} open={settingsOpen} onclose={() => (settingsOpen = false)} width={600}>
  {#if settingsConv}
    {#if settingsConv.type === 'single'}
      <div style="margin-bottom:16px">
        <span style="font-weight:600">{t('chat.remark')}</span>
        <div style="display:flex;gap:8px;margin-top:8px">
          <Input value={myNickname} onInput={(v) => (myNickname = v)} placeholder={t('chat.remarkPlaceholder')} />
          <Button tooltip={t('chat.remarkTooltip')} onClick={save1v1Remark}>{t('common.save')}</Button>
        </div>
      </div>
      <div>
        <span style="font-weight:600">{t('chat.operations')}</span>
        <div style="margin-top:8px">
          {#if isBlocked(otherUserId)}
            <Button danger tooltip={t('chat.unblockTooltip')} onClick={() => handleUnblock(otherUserId)}>{t('chat.unblock')}</Button>
          {:else}
            <Button danger tooltip={t('chat.blockTooltip')} onClick={() => handleBlock(otherUserId)}>
              {#snippet icon()}<Icon name="stop" />{/snippet}{t('chat.block')}
            </Button>
          {/if}
        </div>
      </div>
    {:else}
      {#if settingsConv.my_role === 'admin'}
        <div style="margin-bottom:16px">
          <span style="font-weight:600">{t('chat.groupName')}</span>
          <div style="display:flex;gap:8px;margin-top:8px">
            <Input value={newGroupName} onInput={(v) => (newGroupName = v)} />
            <Button tooltip={t('chat.saveGroupNameTooltip')} onClick={saveGroupName}>{t('common.save')}</Button>
          </div>
        </div>
        <div style="margin-bottom:16px">
          <Popconfirm title={t('chat.disbandConfirm')} onConfirm={disbandGroup}>
            <Button danger tooltip={t('chat.disbandTooltip')}>
              {#snippet icon()}<Icon name="delete" />{/snippet}{t('chat.disbandGroup')}
            </Button>
          </Popconfirm>
        </div>
      {/if}
      <div style="margin-bottom:16px">
        <span style="font-weight:600">{t('chat.myNickname')}</span>
        <div style="display:flex;gap:8px;margin-top:8px">
          <Input value={myNickname} onInput={(v) => (myNickname = v)} placeholder={t('chat.myNicknamePlaceholder')} />
          <Button tooltip={t('chat.saveNicknameTooltip')} onClick={saveMyNickname}>{t('common.save')}</Button>
        </div>
      </div>
      <div style="margin-bottom:16px">
        <span style="font-weight:600">{t('chat.groupNote')}</span>
        <div style="display:flex;gap:8px;margin-top:8px">
          <Input value={groupNote} onInput={(v) => (groupNote = v)} placeholder={t('chat.groupNotePlaceholder')} />
          <Button tooltip={t('chat.saveGroupNote')} onClick={saveGroupNote}>{t('common.save')}</Button>
        </div>
        <div style="margin-top:4px;font-size:12px;color:#999">{t('chat.groupNoteHint')}</div>
      </div>
      <div>
        <div style="display:flex;align-items:center;gap:8px;margin-bottom:8px">
          <span style="font-weight:600">{t('chat.memberManagement')}</span>
          {#if settingsConv.my_role === 'admin'}
            <Button type="text" size="small" tooltip={t('chat.addMemberTooltip')} onClick={openAddUser}>
              {#snippet icon()}<Icon name="plus" />{/snippet}
            </Button>
          {/if}
        </div>
        <ul style="list-style:none;margin:0;padding:0">
          {#each settingsConv.participants as p (p.id)}
            <li
              class="ant-list-item"
              style="display:flex;align-items:center;justify-content:space-between;gap:12px;padding:12px 0;border-bottom:1px solid var(--ant-list-item-border)"
            >
              <div style="display:flex;align-items:center;gap:12px;min-width:0">
                <Avatar src={p.avatar}>
                  <span style="display:inline-flex"><Icon name="user" /></span>
                </Avatar>
                <span style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis">{p.name}</span>
                {#if p.role === 'admin'}
                  <Tag color="gold"><span style="display:inline-flex"><Icon name="crown" /></span>{t('chat.admin')}</Tag>
                {/if}
                {#if p.nickname}<Text type="secondary">({p.nickname})</Text>{/if}
              </div>
              {#if settingsConv.my_role === 'admin' && p.id !== $authStore.user?.id}
                <div style="display:flex;gap:4px;flex-shrink:0">
                  {#if p.role === 'admin'}
                    <Button type="text" size="small" style="border-radius:50%" tooltip={t('chat.demoteTooltip')} onClick={() => changeRole(p.id, 'member')}>
                      {#snippet icon()}<Icon name="swap" />{/snippet}
                    </Button>
                  {:else}
                    <Button type="text" size="small" style="border-radius:50%" tooltip={t('chat.promoteTooltip')} onClick={() => changeRole(p.id, 'admin')}>
                      {#snippet icon()}<Icon name="crown" />{/snippet}
                    </Button>
                  {/if}
                  <Popconfirm title={t('chat.removeMemberConfirm')} onConfirm={() => removeMember(p.id)}>
                    <Button type="text" size="small" danger={true} style="border-radius:50%" tooltip={t('chat.removeMemberTooltip')}>
                      {#snippet icon()}<Icon name="minus-circle" />{/snippet}
                    </Button>
                  </Popconfirm>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  {/if}
</Modal>

<!-- 添加成员 -->
<Modal
  title={t('chat.addMemberModalTitle')}
  open={addUserModal}
  onOk={addMember}
  onclose={() => {
    addUserModal = false
    addUserId = null
  }}
  okText={t('chat.addMember')}
  cancelText={t('common.cancel')}
>
  <Select
    placeholder={t('chat.addMemberPlaceholder')}
    value={addUserId ?? undefined}
    onChange={(v) => (addUserId = typeof v === 'string' ? v : null)}
    options={allEmployees
      .filter((e) => !settingsConv?.participants.some((p) => p.id === e.id))
      .map((e) => ({ value: e.id, label: e.name }))}
  />
</Modal>

<style>
  .chat-input {
    flex: 1;
    min-height: 32px;
    max-height: 96px;
    padding: 4px 11px;
    font-size: var(--ant-font-size);
    font-family: inherit;
    line-height: 1.5715;
    color: var(--ant-color-text);
    background: var(--ant-color-bg-container);
    border: 1px solid var(--ant-color-border);
    border-radius: var(--ant-border-radius);
    outline: none;
    resize: none;
    box-sizing: border-box;
    transition: border-color 0.2s, box-shadow 0.2s;
  }
  .chat-input:focus {
    border-color: var(--ant-color-primary);
    box-shadow: 0 0 0 2px rgba(22, 119, 255, 0.1);
  }
  .chat-input::placeholder {
    color: var(--ant-color-text-quaternary);
  }
</style>
