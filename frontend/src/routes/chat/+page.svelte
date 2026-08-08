<script lang="ts">
  // 聊天页（复刻原 frontend/src/pages/Chat.tsx，Svelte 5 runes 版）
  // - 会话列表 5s 增量轮询、消息 3s 增量轮询（setInterval + onDestroy/$effect cleanup）
  // - 时间显示统一 formatTime(iso, getGlobalPrefs())；新会话插入位置取偏好 newConvPosition
  // - 文件上传：uploadChatFile(file) → sendMessage(msg_type:'file')
  import { onMount, onDestroy } from 'svelte'
  import { page } from '$app/stores'
  import { authStore } from '$lib/stores/auth'
  import { formatTime, getGlobalPrefs, subscribe as subscribePrefs } from '$lib/stores/preferences'
  import {
    getConversations,
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

  async function fetchConversations() {
    loading = true
    try {
      const res = await getConversations()
      if (res.code === 0 && res.data) conversations = res.data
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
        message.error(res.message || '发送失败')
        return
      }
      messages = [...messages, res.data]
      await fetchConversations()
    } catch (err) {
      message.error(getApiError(err, '发送失败'))
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
        message.error(res.message || '发送文件失败')
        return
      }
      messages = [...messages, res.data]
      await fetchConversations()
    } catch (err) {
      message.error((err as Error).message || getApiError(err, '发送文件失败'))
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
        message.error(res.message || '操作失败')
        return
      }
      message.success('已拉黑')
      await fetchBlocked()
      settingsOpen = false
      const conv = conversations.find((c) => {
        if (c.type !== 'single') return false
        const other = c.participants.find((p) => p.id !== $authStore.user?.id)
        return other?.id === blockedId && c.id === selectedConv
      })
      if (conv) selectedConv = null
    } catch (err) {
      message.error(getApiError(err, '操作失败'))
    }
  }

  async function handleUnblock(blockedId: string) {
    if (!blockedId) return
    try {
      const res = await unblockUser(blockedId)
      if (res.code !== 0) {
        message.error(res.message || '操作失败')
        return
      }
      message.success('已取消拉黑')
      await fetchBlocked()
      const conv = conversations.find((c) => {
        if (c.type !== 'single') return false
        const other = c.participants.find((p) => p.id !== $authStore.user?.id)
        return other?.id === blockedId && c.id === selectedConv
      })
      if (conv) selectedConv = null
    } catch (err) {
      message.error(getApiError(err, '操作失败'))
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
      ? settingsConv.participants.find((p) => p.id !== $authStore.user?.id)?.id || ''
      : '',
  )

  async function save1v1Remark() {
    if (!settingsConv) return
    try {
      // 备注仅自己可见：写入当前用户自己的 group_note
      const res = await updateParticipant(settingsConv.id, $authStore.user?.id || '', { group_note: myNickname })
      if (res.code !== 0) {
        message.error(res.message || '操作失败')
        return
      }
      message.success('备注已更新')
      await fetchConversations()
    } catch (err) {
      message.error(getApiError(err, '操作失败'))
    }
  }

  async function saveGroupName() {
    if (!settingsConv || !newGroupName.trim()) return
    try {
      const res = await updateConversationName(settingsConv.id, newGroupName)
      if (res.code !== 0) {
        message.error(res.message || '操作失败')
        return
      }
      message.success('群名已更新')
      await fetchConversations()
    } catch (err) {
      message.error(getApiError(err, '操作失败'))
    }
  }

  async function saveMyNickname() {
    if (!settingsConv) return
    try {
      const res = await updateParticipant(settingsConv.id, $authStore.user?.id || '', { nickname: myNickname || '' })
      if (res.code !== 0) {
        message.error(res.message || '操作失败')
        return
      }
      message.success('群昵称已更新')
      await fetchConversations()
    } catch (err) {
      message.error(getApiError(err, '操作失败'))
    }
  }

  async function saveGroupNote() {
    if (!settingsConv) return
    try {
      const res = await updateParticipant(settingsConv.id, $authStore.user?.id || '', { group_note: groupNote || '' })
      if (res.code !== 0) {
        message.error(res.message || '操作失败')
        return
      }
      message.success('群聊备注已更新')
      await fetchConversations()
    } catch (err) {
      message.error(getApiError(err, '操作失败'))
    }
  }

  async function disbandGroup() {
    if (!settingsConv) return
    try {
      // 注意：$lib/api/chat.ts 的 disbandConversation 封装缺少 /disband 后缀（端点错误），
      // 此处直接用 client 调用后端正确路由 DELETE /chat/conversations/:id/disband。
      const res = await client.delete<null>(`/chat/conversations/${settingsConv.id}/disband`)
      if (res.code !== 0) {
        message.error(res.message || '操作失败')
        return
      }
      message.success('群聊已解散')
      settingsOpen = false
      if (selectedConv === settingsConv.id) selectedConv = null
      await fetchConversations()
    } catch (err) {
      message.error(getApiError(err, '操作失败'))
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
        message.error(res.message || '操作失败')
        return
      }
      message.success('角色已更新')
      await refreshSettings()
    } catch (err) {
      message.error(getApiError(err, '操作失败'))
    }
  }

  async function removeMember(targetId: string) {
    if (!settingsConv) return
    try {
      const res = await removeParticipant(settingsConv.id, targetId)
      if (res.code !== 0) {
        message.error(res.message || '操作失败')
        return
      }
      message.success('已移除')
      await refreshSettings()
    } catch (err) {
      message.error(getApiError(err, '操作失败'))
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
        message.error(res.message || '操作失败')
        return
      }
      message.success('已添加')
      addUserModal = false
      addUserId = null
      await refreshSettings()
    } catch (err) {
      message.error(getApiError(err, '操作失败'))
    }
  }

  // ---- 展示辅助 ----

  function getConvName(conv: Conversation): string {
    if (conv.type === 'group') return conv.my_group_note || conv.name || '群聊'
    const other = conv.participants.find((p) => p.id !== $authStore.user?.id)
    const otherInfo = conv.participants.find((p) => p.id === other?.id)
    return conv.my_group_note || otherInfo?.nickname || other?.name || '未知'
  }

  function getOtherAvatar(conv: Conversation): string | undefined {
    if (conv.type === 'group') return undefined
    const other = conv.participants.find((p) => p.id !== $authStore.user?.id)
    return other?.avatar || undefined
  }

  const filteredConversations = $derived(
    conversations.filter((conv) => {
      if (conv.type === 'single') {
        const other = conv.participants.find((p) => p.id !== $authStore.user?.id)
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
      <Tooltip title="展开会话列表">
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
            <span>会话</span>
            <div style="display:flex;gap:4px;align-items:center">
              <Tooltip title="黑名单">
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
              {#if reorderMode}
                <Tooltip title="完成排序">
                  <Button size="small" type="primary" onClick={() => (reorderMode = false)}>
                    {#snippet icon()}<Icon name="check" />{/snippet}
                  </Button>
                </Tooltip>
              {:else}
                <Tooltip title="调整会话顺序">
                  <Button size="small" onClick={() => (reorderMode = true)}>
                    {#snippet icon()}<Icon name="holder" />{/snippet}
                  </Button>
                </Tooltip>
              {/if}
              <Tooltip title="收起会话列表">
                <Button size="small" onClick={() => (chatCollapsed = true)}>
                  {#snippet icon()}<Icon name="menu-fold" />{/snippet}
                </Button>
              </Tooltip>
            </div>
          </div>
        {/snippet}

        {#if reorderMode}
          <div style="font-size:12px;color:#999;padding:0 4px 8px">拖拽列表项调整会话顺序</div>
        {/if}

        {#if loading}
          <div style="text-align:center;padding:40px 0"><Spin /></div>
        {:else}
          <List hasData={orderedConversations.length > 0} emptyText="暂无会话">
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
                    {conv.last_message || '暂无消息'}
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
              <Text type="secondary" style="font-size:12px">({selectedConvData.participants.length} 人)</Text>
            {/if}
          </div>
          <Button type="text" onClick={() => openSettings(selectedConvData)}>
            {#snippet icon()}<Icon name="setting" />{/snippet}
          </Button>
        </div>
      {:else}
        消息
      {/if}
    {/snippet}

    {#if selectedConv}
      <div
        bind:this={messagesContainerEl}
        role="region"
        aria-label="消息列表，可拖放文件发送"
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
            <span style="font-size:16px;color:#1677ff">松开鼠标发送文件</span>
          </div>
        {/if}
        {#if msgLoading}
          <div style="text-align:center;padding:40px 0"><Spin /></div>
        {:else if messages.length === 0}
          <div style="text-align:center;color:#999;margin-top:60px">暂无消息</div>
        {:else}
          {#each messages as msg (msg.id)}
            {@const isMe = msg.sender_id === $authStore.user?.id}
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
                  {isMe ? '我' : msg.sender_name}
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
                      <span style="color:{isMe ? '#fff' : '#1677ff'}"><Icon name="paper-clip" /> {name}（文件链接无效）</span>
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
          <Button onClick={() => fileInputEl?.click()}>
            {#snippet icon()}<Icon name="paper-clip" />{/snippet}
          </Button>
          <input bind:this={fileInputEl} type="file" style="display:none" onchange={handleFileSend} />
          <textarea
            bind:this={textareaEl}
            class="chat-input"
            value={inputText}
            placeholder="输入消息..."
            rows={1}
            oninput={onTextareaInput}
            onkeydown={onTextareaKeydown}
          ></textarea>
          <Button type="primary" onClick={handleSend}>
            {#snippet icon()}<Icon name="send" />{/snippet}发送
          </Button>
        </div>
      </div>
    {:else}
      <div style="text-align:center;color:#999;margin-top:120px">
        <Icon name="message" style="font-size:48px;margin-bottom:16px" />
        <div>选择一个会话开始聊天</div>
      </div>
    {/if}
  </Card>
</div>

<!-- 黑名单 -->
<Modal title="黑名单" open={blockModal} onclose={() => (blockModal = false)}>
  {#if blockedUsers.length === 0}
    <Empty description="暂无拉黑的用户" />
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
          <Button size="small" onClick={() => handleUnblock(item.id)}>取消拉黑</Button>
        </li>
      {/each}
    </ul>
  {/if}
</Modal>

<!-- 聊天设置 -->
<Modal title="聊天设置" open={settingsOpen} onclose={() => (settingsOpen = false)} width={600}>
  {#if settingsConv}
    {#if settingsConv.type === 'single'}
      <div style="margin-bottom:16px">
        <span style="font-weight:600">备注</span>
        <div style="display:flex;gap:8px;margin-top:8px">
          <Input value={myNickname} onInput={(v) => (myNickname = v)} placeholder="给对方设置备注（仅自己可见）" />
          <Button onClick={save1v1Remark}>保存</Button>
        </div>
      </div>
      <div>
        <span style="font-weight:600">操作</span>
        <div style="margin-top:8px">
          {#if isBlocked(otherUserId)}
            <Button danger onClick={() => handleUnblock(otherUserId)}>取消拉黑</Button>
          {:else}
            <Button danger onClick={() => handleBlock(otherUserId)}>
              {#snippet icon()}<Icon name="stop" />{/snippet}拉黑
            </Button>
          {/if}
        </div>
      </div>
    {:else}
      {#if settingsConv.my_role === 'admin'}
        <div style="margin-bottom:16px">
          <span style="font-weight:600">群聊名称</span>
          <div style="display:flex;gap:8px;margin-top:8px">
            <Input value={newGroupName} onInput={(v) => (newGroupName = v)} />
            <Button onClick={saveGroupName}>保存</Button>
          </div>
        </div>
        <div style="margin-bottom:16px">
          <Popconfirm title="确定解散群聊？此操作不可恢复！" onConfirm={disbandGroup}>
            <Button danger>
              {#snippet icon()}<Icon name="delete" />{/snippet}解散群聊
            </Button>
          </Popconfirm>
        </div>
      {/if}
      <div style="margin-bottom:16px">
        <span style="font-weight:600">我的群昵称</span>
        <div style="display:flex;gap:8px;margin-top:8px">
          <Input value={myNickname} onInput={(v) => (myNickname = v)} placeholder="设置我在群内的显示名称" />
          <Button onClick={saveMyNickname}>保存</Button>
        </div>
      </div>
      <div style="margin-bottom:16px">
        <span style="font-weight:600">群聊备注</span>
        <div style="display:flex;gap:8px;margin-top:8px">
          <Input value={groupNote} onInput={(v) => (groupNote = v)} placeholder="设置后覆盖群聊名称显示" />
          <Button onClick={saveGroupNote}>保存</Button>
        </div>
        <div style="margin-top:4px;font-size:12px;color:#999">设置后，会话列表中将以备注名称显示该群聊</div>
      </div>
      <div>
        <div style="display:flex;align-items:center;gap:8px;margin-bottom:8px">
          <span style="font-weight:600">成员管理</span>
          {#if settingsConv.my_role === 'admin'}
            <Button type="text" size="small" onClick={openAddUser}>
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
                  <Tag color="gold"><span style="display:inline-flex"><Icon name="crown" /></span>管理员</Tag>
                {/if}
                {#if p.nickname}<Text type="secondary">({p.nickname})</Text>{/if}
              </div>
              {#if settingsConv.my_role === 'admin' && p.id !== $authStore.user?.id}
                <div style="display:flex;gap:4px;flex-shrink:0">
                  {#if p.role === 'admin'}
                    <Button type="text" size="small" style="border-radius:50%" onClick={() => changeRole(p.id, 'member')}>
                      {#snippet icon()}<Icon name="swap" />{/snippet}
                    </Button>
                  {:else}
                    <Button type="text" size="small" style="border-radius:50%" onClick={() => changeRole(p.id, 'admin')}>
                      {#snippet icon()}<Icon name="crown" />{/snippet}
                    </Button>
                  {/if}
                  <Popconfirm title="确定移除该成员？" onConfirm={() => removeMember(p.id)}>
                    <Button type="text" size="small" danger={true} style="border-radius:50%">
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
  title="添加成员"
  open={addUserModal}
  onOk={addMember}
  onclose={() => {
    addUserModal = false
    addUserId = null
  }}
  okText="添加"
  cancelText="取消"
>
  <Select
    placeholder="选择成员"
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
