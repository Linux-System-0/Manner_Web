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
  import { getDepartments } from '$lib/api/departments'
  import { getOrCreateDirectConversation } from '$lib/api/chat'
  import type { Employee, PermissionModule } from '$lib/types'
  import Table from '$lib/components/Table.svelte'
  import type { TableColumn } from '$lib/components/Table.svelte'
  import Button from '$lib/components/Button.svelte'
  import Input from '$lib/components/Input.svelte'
  import Select from '$lib/components/Select.svelte'
  import Space from '$lib/components/Space.svelte'
  import Popconfirm from '$lib/components/Popconfirm.svelte'
  import Modal from '$lib/components/Modal.svelte'
  import Checkbox from '$lib/components/Checkbox.svelte'
  import Tag from '$lib/components/Tag.svelte'
  import Card from '$lib/components/Card.svelte'
  import Result from '$lib/components/Result.svelte'
  import { Icon } from '$lib/icons'
  import { message } from '$lib/components/message'
  import { modal } from '$lib/components/modal'

  const PAGE_SIZE = 10

  let data = $state<Employee[]>([])
  let loading = $state(false)
  let total = $state(0)

  // 搜索条件（查询/重置按钮触发）
  let keyword = $state('')
  let deptFilter = $state('')
  let deptOptions = $state<{ value: string; label: string }[]>([])
  let params = $state({ page: 1, page_size: PAGE_SIZE, keyword: '', department_id: '' })

  // 重置密码弹窗
  let resetModal = $state({ open: false, employee: null as Employee | null, password: '', confirm: '' })
  let resetting = $state(false)

  // F-08: 重置密码成功后的初始密码展示弹窗（页面内弹窗展示，不用 toast 提示显示密码）
  let pwdResult = $state({ open: false, password: '', name: '' })

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
  let canViewSensitive = $derived($authStore.permissions.includes('employee:view_sensitive'))

  async function fetchData() {
    loading = true
    try {
      const res = await getEmployees({
        page: params.page,
        page_size: params.page_size,
        keyword: params.keyword || undefined,
        department_id: params.department_id || undefined,
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
    params = {
      page: 1,
      page_size: PAGE_SIZE,
      keyword: keyword.trim(),
      department_id: deptFilter,
    }
    fetchData()
  }

  function handleReset() {
    keyword = ''
    deptFilter = ''
    handleSearch()
  }

  function handleTableChange(page: number) {
    params = { ...params, page }
    fetchData()
  }

  // ---- 点击姓名发起聊天：获取/创建与该员工的单聊会话并跳转 ----
  async function openChat(emp: Employee) {
    try {
      const res = await getOrCreateDirectConversation(emp.id)
      if (res.code !== 0 || !res.data) {
        message.error(res.message || '打开会话失败')
        return
      }
      goto(`/chat?conv=${res.data.id}`)
    } catch (err: unknown) {
      message.error(getApiError(err, '打开会话失败'))
    }
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

  // ---- 查看敏感信息 ----
  // 第一次确认（操作入口弹窗）：提示查看将记录日志，确认后进入敏感信息页。
  async function openSensitive(emp: Employee) {
    const ok = await modal.confirm({
      title: '查看完整信息',
      content: '查看该员工的敏感信息（手机号 / 邮箱 / 身份证号 / 地址）将记录到系统日志。确认继续？',
      okText: '确认查看',
    })
    if (ok) goto(`/employees/${emp.id}/sensitive`)
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
      message.success('密码重置成功')
      // F-08: 初始密码通过页面内弹窗展示
      pwdResult = { open: true, password: resetModal.password, name: emp.name || emp.username }
      resetModal = { open: false, employee: null, password: '', confirm: '' }
    } catch (err: unknown) {
      message.error(getApiError(err, '密码重置失败'))
    } finally {
      resetting = false
    }
  }

  async function copyResetPwd() {
    try {
      await navigator.clipboard.writeText(pwdResult.password)
      message.success('已复制到剪贴板')
    } catch {
      message.error('复制失败，请手动选择复制')
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
    if (!canManagePerm(code)) return
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
    view_sensitive: '查看敏感信息',
    password: '重置密码',
    protect_block: '防拉黑保护',
    group_create: '群聊创建',
    upload: '上传文件',
    config: '系统配置',
    settings: '设置',
  }
  const moduleOrder: Record<string, number> = { employee: 0, department: 1, chat: 3, system: 4 }
  const chatKeepActions = new Set(['protect_block', 'group_create', 'upload'])
  const hideActions = new Set(['config'])

  /** 操作者能否管理该权限码（增删）：只能操作自己拥有的权限 */
  const canManagePerm = (code: string) => $authStore.permissions.includes(code)

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

  /** 模块内当前可勾选的权限码（排除聊天划线项、list 锁定项与操作者无权增删的锁定项） */
  function checkableCodes(mod: PermissionModule): string[] {
    const out: string[] = []
    for (const p of mod.permissions) {
      const action = p.code.split(':')[1]
      if (!action) continue
      if (isChatStruck(mod, true, action)) continue
      if (action === 'list' && isAnyOtherChecked(mod)) continue
      if (!canManagePerm(p.code)) continue
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
    { title: '姓名', key: 'name', width: 100, snippet: 'name' },
    {
      title: '职位',
      key: 'title',
      width: 100,
      render: (r) => r.title || '-',
    },
    {
      title: '所属部门',
      key: 'departments',
      width: 140,
      render: (r) => r.departments || '-',
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
    { title: '操作', key: 'action', width: 420, snippet: 'action' },
  ]

  onMount(() => {
    if (!$authStore.permissions.includes('employee:list')) return
    fetchData()
    getDepartments()
      .then((res) => {
        if (res.code === 0) {
          deptOptions = res.data.items.map((d) => ({ value: d.id, label: d.name }))
        }
      })
      .catch(() => {})
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
      <Button type="primary" tooltip="返回系统首页" onClick={() => goto('/')}>返回首页</Button>
    {/snippet}
  </Result>
{:else}
  {#snippet name(row: Employee)}
    {#if row.id === $authStore.user?.id}
      {row.name}
    {:else}
      <Button type="link" size="small" tooltip={`与 ${row.name} 发起聊天`} onClick={() => openChat(row)} style="padding:0;font-weight:500">
        {row.name}
      </Button>
    {/if}
  {/snippet}

  {#snippet status(row: Employee)}
    <Tag color={row.status === 1 ? 'success' : 'default'}>{row.status === 1 ? '在职' : '禁用'}</Tag>
  {/snippet}

  {#snippet action(row: Employee)}
    <Space size="small" wrap={true}>
      {#if !(row.id === $authStore.user?.id) && canEdit}
        <Button type="link" size="small" tooltip="编辑该员工的基本信息" onClick={() => goto(`/employees/${row.id}/edit`)}>
          <Icon name="edit" style="font-size:14px" />编辑
        </Button>
      {/if}
      {#if !(row.id === $authStore.user?.id) && canPermEdit}
        <Button type="link" size="small" tooltip="设置该员工的权限" onClick={() => openPermModal(row)}>
          <Icon name="setting" style="font-size:14px" />权限
        </Button>
      {/if}
      {#if !(row.id === $authStore.user?.id) && canPassword}
        <Button type="link" size="small" tooltip="重置该员工的登录密码" onClick={() => openResetModal(row)}>
          <Icon name="key" style="font-size:14px" />密码
        </Button>
      {/if}
      {#if !(row.id === $authStore.user?.id) && canDelete}
        <Popconfirm title="确定要删除该员工吗？" onConfirm={() => handleDelete(row.id)}>
          <Button type="link" size="small" danger={true} tooltip="删除该员工">
            <Icon name="delete" style="font-size:14px" />删除
          </Button>
        </Popconfirm>
      {/if}
      {#if canViewSensitive}
        <Button type="link" size="small" tooltip="查看该员工的敏感完整信息" onClick={() => openSensitive(row)}>
          <Icon name="eye" style="font-size:14px" />查看完整信息
        </Button>
      {/if}
    </Space>
  {/snippet}

  <div class="page-scroll" style="height:100%;overflow:auto">
    <Card bodyStyle="padding:16px 24px" style="margin-bottom:16px">
      <div style="display:flex;align-items:center;gap:12px;flex-wrap:wrap">
        <Input
          placeholder="搜索姓名/用户名"
          prefix="search"
          value={keyword}
          onInput={(v) => (keyword = v)}
          onEnter={handleSearch}
          style="width:280px;flex-shrink:0"
        />
        <Select
          value={deptFilter || undefined}
          options={deptOptions}
          allowClear={true}
          placeholder="按部门筛选"
          width="200px"
          onChange={(v) => (deptFilter = String(v || ''))}
        />
        <Space size="small">
          <Button type="primary" tooltip="按当前条件搜索员工" onClick={handleSearch}>查询</Button>
          <Button tooltip="清空搜索条件" onClick={handleReset}>重置</Button>
        </Space>
        <div style="flex:1"></div>
        {#if canCreate}
          <Button type="primary" tooltip="创建新员工" onClick={() => goto('/employees/new')}>
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

  <!-- F-08: 重置密码成功 - 初始密码展示弹窗（一次性密码，禁止遮罩点击误关） -->
  <Modal
    open={pwdResult.open}
    title={`密码重置成功 - ${pwdResult.name}`}
    onclose={() => (pwdResult = { open: false, password: '', name: '' })}
    onOk={() => (pwdResult = { open: false, password: '', name: '' })}
    okText="我知道了"
    cancelText="关闭"
    maskClosable={false}
  >
    <div style="display:flex;flex-direction:column;gap:12px">
      <span style="color:var(--ant-color-text-secondary)">
        密码已重置，以下为一次性初始密码（仅显示一次，请复制并妥善保存）：
      </span>
      <div style="display:flex;align-items:center;gap:8px">
        <code
          style="flex:1;padding:8px 12px;border:1px solid var(--ant-color-border-secondary);border-radius:6px;background:var(--ant-color-fill-secondary);font-size:16px;letter-spacing:1px;user-select:all"
        >{pwdResult.password}</code>
        <Button size="small" tooltip="复制初始密码到剪贴板" onClick={copyResetPwd}>复制</Button>
      </div>
      <span style="color:var(--ant-color-warning)">该员工下次登录需使用此新密码</span>
    </div>
  </Modal>

  <!-- 权限设置弹窗 -->
  {#snippet permFooter()}
    <Button tooltip="关闭弹窗，不保存修改" onClick={() => (permModal = { open: false, employee: null })}>取消</Button>
    <Button type="primary" tooltip="保存权限设置" loading={savingPerms} onClick={handleSavePerms}>保存权限</Button>
  {/snippet}

  {#snippet permMatrix()}
    <div style="min-width:0">
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
                style="padding:6px 8px;border:1px solid var(--ant-color-border-secondary);background:var(--ant-color-fill-quaternary);text-align:center;white-space:nowrap;font-size:13px"
              >
                资源模块
              </th>
              {#each matrixActions(permModules) as action}
                <th
                  style="padding:6px 8px;border:1px solid var(--ant-color-border-secondary);background:var(--ant-color-fill-quaternary);text-align:center;white-space:nowrap;font-size:13px"
                >
                  {actionLabels[action] || action}
                </th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#each sortedModules(permModules) as mod (mod.module)}
              {@const state = moduleCheckState(mod)}
              <tr>
                <td
                  style="padding:6px 8px;border:1px solid var(--ant-color-border-secondary);font-weight:500;white-space:nowrap"
                >
                  <Checkbox
                    checked={state.all}
                    indeterminate={state.partial}
                    disabled={checkableCodes(mod).length === 0}
                    onChange={() => toggleModuleAll(mod)}
                  />
                  <span style="margin-left:6px">{mod.module_name}</span>
                </td>
                {#each matrixActions(permModules) as action}
                  {@const permCode = `${mod.module}:${action}`}
                  {@const permExists = mod.permissions.some((p) => p.code === permCode)}
                  {@const isChatOther = isChatStruck(mod, permExists, action)}
                  {@const isListDisabled = action === 'list' && isAnyOtherChecked(mod)}
                  <td style="padding:6px 8px;border:1px solid var(--ant-color-border-secondary);text-align:center">
                    {#if permExists && !isChatOther}
                      {@const locked = !canManagePerm(permCode)}
                      <Checkbox
                        checked={isListDisabled || selectedPerms.includes(permCode)}
                        disabled={isListDisabled || locked}
                        style={isListDisabled || locked ? 'cursor:not-allowed' : ''}
                        onChange={() => {
                          if (isListDisabled || locked) return
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
    width={1000}
    bodyStyle="padding:16px 24px;overflow-x:auto"
  >
    {@render permMatrix()}
  </Modal>
{/if}
