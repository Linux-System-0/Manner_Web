<script lang="ts">
  // 员工管理 - 列表（复刻 React 版 frontend/src/pages/employees/List.tsx）
  // 角色/部门机制已移除：仅保留直接权限授权。
  import { onMount } from 'svelte'
  import { get } from 'svelte/store'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { preferencesStore, formatTimestamp } from '$lib/stores/preferences'
  import { getApiError } from '$lib/api/client'
  import {
    getEmployees,
    getEmployee,
    deleteEmployee,
    resetPassword,
    updateEmployeePermissions,
  } from '$lib/api/employees'
  import { getPermissions } from '$lib/api/system'
  import type { Employee, PermissionModule } from '$lib/types'
  import Table from '$lib/components/Table.svelte'
  import type { TableColumn } from '$lib/components/Table.svelte'
  import Button from '$lib/components/Button.svelte'
  import Input from '$lib/components/Input.svelte'
  import Space from '$lib/components/Space.svelte'
  import Popconfirm from '$lib/components/Popconfirm.svelte'
  import Modal from '$lib/components/Modal.svelte'
  import Checkbox from '$lib/components/Checkbox.svelte'
  import Tag from '$lib/components/Tag.svelte'
  import Card from '$lib/components/Card.svelte'
  import Result from '$lib/components/Result.svelte'
  import { Icon } from '$lib/icons'
  import { message } from '$lib/components/message'

  const PAGE_SIZE = 10

  let data = $state<Employee[]>([])
  let loading = $state(false)
  let total = $state(0)

  // 搜索条件（查询/重置按钮触发）
  let keyword = $state('')
  let params = $state({ page: 1, page_size: PAGE_SIZE, keyword: '' })

  // 重置密码弹窗
  let resetModal = $state({ open: false, employee: null as Employee | null, password: '', confirm: '' })
  let resetting = $state(false)

  // 权限设置弹窗
  let permModal = $state({ open: false, employee: null as Employee | null })
  let permModules = $state<PermissionModule[]>([])
  let selectedPerms = $state<string[]>([])
  let savingPerms = $state(false)

  let canCreate = $derived($authStore.permissions.includes('employee:create'))
  let canEdit = $derived($authStore.permissions.includes('employee:edit'))
  let canPassword = $derived($authStore.permissions.includes('employee:password'))
  let canPermEdit = $derived($authStore.permissions.includes('employee:edit'))
  let canDelete = $derived($authStore.permissions.includes('employee:delete'))

  async function fetchData() {
    loading = true
    try {
      const res = await getEmployees({
        page: params.page,
        page_size: params.page_size,
        keyword: params.keyword || undefined,
      })
      if (res.code !== 0) {
        message.error(res.message || '获取员工列表失败')
        return
      }
      data = res.data.items
      total = res.data.total
    } catch (err: unknown) {
      message.error(getApiError(err, '获取员工列表失败'))
    } finally {
      loading = false
    }
  }

  function handleSearch() {
    params = { page: 1, page_size: PAGE_SIZE, keyword: keyword.trim() }
    fetchData()
  }

  function handleReset() {
    keyword = ''
    handleSearch()
  }

  function handleTableChange(page: number) {
    params = { ...params, page }
    fetchData()
  }

  // ---- 删除 ----
  async function handleDelete(id: string) {
    try {
      const res = await deleteEmployee(id)
      if (res.code !== 0) {
        message.error(res.message || '删除失败')
        return
      }
      message.success('删除成功')
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, '删除失败'))
    }
  }

  // ---- 重置密码 ----
  function openResetModal(emp: Employee) {
    resetModal = { open: true, employee: emp, password: '', confirm: '' }
  }

  async function handleResetPassword() {
    const emp = resetModal.employee
    if (!emp) return
    if (!resetModal.password) {
      message.error('请输入新密码')
      return
    }
    if (resetModal.password.length < 8) {
      message.error('密码至少 8 位')
      return
    }
    if (resetModal.password !== resetModal.confirm) {
      message.error('两次输入的密码不一致')
      return
    }
    resetting = true
    try {
      const res = await resetPassword(emp.id, resetModal.password)
      if (res.code !== 0) {
        message.error(res.message || '密码重置失败')
        return
      }
      // F-02: 重置后的密码即该员工下次登录的一次性初始密码
      message.success(`密码重置成功，初始密码：${resetModal.password}`)
      resetModal = { open: false, employee: null, password: '', confirm: '' }
    } catch (err: unknown) {
      message.error(getApiError(err, '密码重置失败'))
    } finally {
      resetting = false
    }
  }

  // ---- 权限矩阵 ----
  async function openPermModal(emp: Employee) {
    try {
      const [permRes, empRes] = await Promise.all([getPermissions(), getEmployee(emp.id)])
      if (permRes.code !== 0 || empRes.code !== 0) {
        message.error(permRes.message || empRes.message || '获取权限数据失败')
        return
      }
      permModules = permRes.data.modules
      selectedPerms = ((empRes.data as Employee & { permissions?: string[] }).permissions) || []
      permModal = { open: true, employee: emp }
    } catch (err: unknown) {
      message.error(getApiError(err, '获取权限数据失败'))
    }
  }

  function togglePerm(code: string) {
    selectedPerms = selectedPerms.includes(code)
      ? selectedPerms.filter((c) => c !== code)
      : [...selectedPerms, code]
  }

  async function handleSavePerms() {
    const emp = permModal.employee
    if (!emp) return
    savingPerms = true
    try {
      const res = await updateEmployeePermissions(emp.id, selectedPerms)
      if (res.code !== 0) {
        message.error(res.message || '保存权限失败')
        return
      }
      message.success('权限已更新')
      permModal = { open: false, employee: null }
    } catch (err: unknown) {
      message.error(getApiError(err, '保存权限失败'))
    } finally {
      savingPerms = false
    }
  }

  // ---- 权限矩阵辅助逻辑 ----
  const actionLabels: Record<string, string> = {
    list: '查看列表',
    view: '查看详情',
    create: '新增',
    edit: '编辑',
    delete: '删除',
    password: '重置密码',
    protect_block: '防拉黑保护',
    group_create: '群聊创建',
    config: '系统配置',
    settings: '设置',
  }
  const moduleOrder: Record<string, number> = { employee: 0, chat: 3, system: 4 }
  const chatKeepActions = new Set(['protect_block', 'group_create'])
  const hideActions = new Set(['config', 'upload'])
  const hasSystemConfig = $derived(selectedPerms.includes('system:config'))

  function sortedModules(mods: PermissionModule[]): PermissionModule[] {
    return [...mods]
      .sort((a, b) => (moduleOrder[a.module] ?? 99) - (moduleOrder[b.module] ?? 99))
  }

  function matrixActions(mods: PermissionModule[]): string[] {
    const set = new Set<string>()
    for (const m of mods) {
      for (const p of m.permissions) {
        const a = p.code.split(':')[1]
        if (a) set.add(a)
      }
    }
    return [...set].filter((a) => !hideActions.has(a))
  }

  function isAnyOtherChecked(mod: PermissionModule): boolean {
    return mod.permissions.some((p) => {
      const action = p.code.split(':')[1]
      return action !== 'list' && selectedPerms.includes(p.code)
    })
  }

  function isChatStruck(mod: PermissionModule, permExists: boolean, action: string): boolean {
    return mod.module === 'chat' && permExists && !chatKeepActions.has(action)
  }

  /** 模块内当前可勾选的权限码（排除聊天划线项与 list 锁定项） */
  function checkableCodes(mod: PermissionModule): string[] {
    const out: string[] = []
    for (const p of mod.permissions) {
      const action = p.code.split(':')[1]
      if (!action) continue
      if (isChatStruck(mod, true, action)) continue
      if (action === 'list' && isAnyOtherChecked(mod)) continue
      out.push(p.code)
    }
    return out
  }

  /** 模块级全选/半选状态 */
  function moduleCheckState(mod: PermissionModule): { all: boolean; partial: boolean } {
    const codes = checkableCodes(mod)
    if (codes.length === 0) return { all: false, partial: false }
    const checked = codes.filter((c) => selectedPerms.includes(c)).length
    return { all: checked === codes.length, partial: checked > 0 && checked < codes.length }
  }

  function toggleModuleAll(mod: PermissionModule) {
    const codes = checkableCodes(mod)
    const { all } = moduleCheckState(mod)
    const next = new Set(selectedPerms)
    if (all) {
      codes.forEach((c) => next.delete(c))
    } else {
      codes.forEach((c) => next.add(c))
    }
    selectedPerms = [...next]
  }

  // ---- 表格列 ----
  const columns: TableColumn<Employee>[] = [
    { title: '用户名', dataIndex: 'username', key: 'username', width: 110 },
    { title: '姓名', dataIndex: 'name', key: 'name', width: 100 },
    {
      title: '职位',
      key: 'title',
      width: 100,
      render: (r) => r.title || '-',
    },
    {
      title: '电话',
      key: 'phone',
      width: 120,
      render: (r) => r.phone || '-',
    },
    { title: '状态', key: 'status', width: 80, align: 'center', snippet: 'status' },
    {
      title: '创建时间',
      key: 'created_at',
      width: 150,
      render: (r) => formatTimestamp(r.created_at, get(preferencesStore)),
    },
    { title: '操作', key: 'action', width: 320, snippet: 'action' },
  ]

  onMount(() => {
    if (!$authStore.permissions.includes('employee:list')) return
    fetchData()
    getPermissions()
      .then((res) => {
        if (res.code === 0) permModules = res.data.modules
      })
      .catch(() => {})
  })
</script>

{#if !$authStore.permissions.includes('employee:list')}
  <Result status="403" title="403" subTitle="抱歉，你无权访问该页面">
    {#snippet extra()}
      <Button type="primary" onClick={() => goto('/')}>返回首页</Button>
    {/snippet}
  </Result>
{:else}
  {#snippet status(row: Employee)}
    <Tag color={row.status === 1 ? 'success' : 'default'}>{row.status === 1 ? '在职' : '禁用'}</Tag>
  {/snippet}

  {#snippet action(row: Employee)}
    <Space size="small" wrap={true}>
      {#if !(row.id === $authStore.user?.id) && canEdit}
        <Button type="link" size="small" onClick={() => goto(`/employees/${row.id}/edit`)}>
          <Icon name="edit" style="font-size:14px" />编辑
        </Button>
      {/if}
      {#if !(row.id === $authStore.user?.id) && canPermEdit}
        <Button type="link" size="small" onClick={() => openPermModal(row)}>
          <Icon name="setting" style="font-size:14px" />权限
        </Button>
      {/if}
      {#if !(row.id === $authStore.user?.id) && canPassword}
        <Button type="link" size="small" onClick={() => openResetModal(row)}>
          <Icon name="key" style="font-size:14px" />密码
        </Button>
      {/if}
      {#if !(row.id === $authStore.user?.id) && canDelete}
        <Popconfirm title="确定要删除该员工吗？" onConfirm={() => handleDelete(row.id)}>
          <Button type="link" size="small" danger={true}>
            <Icon name="delete" style="font-size:14px" />删除
          </Button>
        </Popconfirm>
      {/if}
    </Space>
  {/snippet}

  <div class="page-scroll" style="height:100%;overflow:auto">
    <Card bodyStyle="padding:16px 24px" style="margin-bottom:16px">
      <div style="display:flex;align-items:center;gap:12px;flex-wrap:wrap">
        <Input
          placeholder="搜索姓名/邮箱/手机号"
          prefix="search"
          value={keyword}
          onInput={(v) => (keyword = v)}
          onEnter={handleSearch}
          style="width:320px;flex-shrink:0"
        />
        <Space size="small">
          <Button type="primary" onClick={handleSearch}>查询</Button>
          <Button onClick={handleReset}>重置</Button>
        </Space>
        <div style="flex:1"></div>
        {#if canCreate}
          <Button type="primary" onClick={() => goto('/employees/new')}>
            <Icon name="plus" style="font-size:14px" />新增员工
          </Button>
        {/if}
      </div>
    </Card>

    <Table
      columns={columns}
      dataSource={data as never[]}
      rowKey="id"
      loading={loading}
      scroll={{ x: 1100 }}
      pagination={{
        current: params.page,
        pageSize: params.page_size,
        total,
        onChange: handleTableChange,
        showTotal: (t) => `共 ${t} 条`,
      }}
      snippets={{ status, action }}
    />
  </div>

  <!-- 重置密码弹窗 -->
  <Modal
    open={resetModal.open}
    title={`重置密码 - ${resetModal.employee?.name || ''}`}
    onclose={() => (resetModal = { open: false, employee: null, password: '', confirm: '' })}
    onOk={handleResetPassword}
    confirmLoading={resetting}
    okText="确认重置"
    cancelText="取消"
  >
    <div style="display:flex;flex-direction:column;gap:12px">
      <Input
        type="password"
        placeholder="请输入新密码"
        value={resetModal.password}
        onInput={(v) => (resetModal = { ...resetModal, password: v })}
      />
      <Input
        type="password"
        placeholder="请再次输入新密码"
        value={resetModal.confirm}
        onInput={(v) => (resetModal = { ...resetModal, confirm: v })}
      />
    </div>
  </Modal>

  <!-- 权限设置弹窗 -->
  {#snippet permFooter()}
    <Button onClick={() => (permModal = { open: false, employee: null })}>取消</Button>
    <Button type="primary" loading={savingPerms} onClick={handleSavePerms}>保存权限</Button>
  {/snippet}

  {#snippet permMatrix()}
    <div style="min-width:600px">
      <span style="color:var(--ant-color-text-secondary);display:block;margin-bottom:16px">
        勾选需要分配给该用户的权限
      </span>
      {#if permModules.length === 0}
        <span style="color:var(--ant-color-warning)">暂无权限数据</span>
      {:else}
        <table style="border-collapse:collapse;width:100%">
          <thead>
            <tr>
              <th
                style="padding:8px 12px;border:1px solid var(--ant-color-border-secondary);background:var(--ant-color-fill-quaternary);text-align:center;white-space:nowrap"
              >
                资源模块
              </th>
              {#each matrixActions(permModules) as action}
                <th
                  style="padding:8px 12px;border:1px solid var(--ant-color-border-secondary);background:var(--ant-color-fill-quaternary);text-align:center;white-space:nowrap"
                >
                  {actionLabels[action] || action}
                </th>
              {/each}
              {#if hasSystemConfig}
                <th
                  style="padding:8px 12px;border:1px solid var(--ant-color-border-secondary);background:var(--ant-color-fill-quaternary);text-align:center;white-space:nowrap"
                >
                  设置
                </th>
              {/if}
            </tr>
          </thead>
          <tbody>
            {#each sortedModules(permModules) as mod (mod.module)}
              {@const state = moduleCheckState(mod)}
              <tr>
                <td
                  style="padding:8px 12px;border:1px solid var(--ant-color-border-secondary);font-weight:500;white-space:nowrap"
                >
                  <Checkbox
                    checked={state.all}
                    indeterminate={state.partial}
                    onChange={() => toggleModuleAll(mod)}
                  />
                  <span style="margin-left:6px">{mod.module_name}</span>
                </td>
                {#each matrixActions(permModules) as action}
                  {@const permCode = `${mod.module}:${action}`}
                  {@const permExists = mod.permissions.some((p) => p.code === permCode)}
                  {@const isChatOther = isChatStruck(mod, permExists, action)}
                  {@const isListDisabled = action === 'list' && isAnyOtherChecked(mod)}
                  <td style="padding:8px 12px;border:1px solid var(--ant-color-border-secondary);text-align:center">
                    {#if permExists && !isChatOther}
                      <Checkbox
                        checked={isListDisabled || selectedPerms.includes(permCode)}
                        disabled={isListDisabled}
                        style={isListDisabled ? 'cursor:not-allowed' : ''}
                        onChange={() => {
                          if (isListDisabled && action === 'list') return
                          togglePerm(permCode)
                        }}
                      />
                    {:else if permExists && isChatOther}
                      <span style="color:var(--ant-color-text-quaternary);text-decoration:line-through">
                        {actionLabels[action] || action}
                      </span>
                    {:else}
                      <span style="color:var(--ant-color-text-quaternary)">—</span>
                    {/if}
                  </td>
                {/each}
                {#if hasSystemConfig}
                  <td style="padding:8px 12px;border:1px solid var(--ant-color-border-secondary);text-align:center">
                    {#if mod.module === 'system'}
                      <Checkbox
                        checked={selectedPerms.includes(`${mod.module}:settings`)}
                        onChange={() => togglePerm(`${mod.module}:settings`)}
                      />
                    {:else}
                      <span style="color:var(--ant-color-text-quaternary);text-decoration:line-through">—</span>
                    {/if}
                  </td>
                {/if}
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>
  {/snippet}

  <Modal
    open={permModal.open}
    title={`权限设置 - ${permModal.employee?.name || ''}`}
    onclose={() => (permModal = { open: false, employee: null })}
    footer={permFooter}
    width={900}
    bodyStyle="padding:16px 24px;overflow-x:auto"
  >
    {@render permMatrix()}
  </Modal>
{/if}
