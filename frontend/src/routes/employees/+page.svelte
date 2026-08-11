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
  } from '$lib/api/employees'
  import { getRoles, updateEmployeeRoles } from '$lib/api/roles'
  import { getDepartments } from '$lib/api/departments'
  import { getOrCreateDirectConversation } from '$lib/api/chat'
  import type { Employee, Role } from '$lib/types'
  import Table from '$lib/components/Table.svelte'
  import type { TableColumn } from '$lib/components/Table.svelte'
  import Button from '$lib/components/Button.svelte'
  import Input from '$lib/components/Input.svelte'
  import Select from '$lib/components/Select.svelte'
  import Space from '$lib/components/Space.svelte'
  import Popconfirm from '$lib/components/Popconfirm.svelte'
  import Modal from '$lib/components/Modal.svelte'
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

  // 角色设置弹窗
  let roleModal = $state({ open: false, employee: null as Employee | null })
  let roleList = $state<Role[]>([])
  let selectedRoleIds = $state<string[]>([])
  let savingRoles = $state(false)

  let canCreate = $derived($authStore.permissions.includes('employee:create'))
  let canEdit = $derived($authStore.permissions.includes('employee:edit'))
  let canPassword = $derived($authStore.permissions.includes('employee:password'))
  let canRoleEdit = $derived($authStore.permissions.includes('role:manage'))
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

  // ---- 角色分配 ----
  async function openRoleModal(emp: Employee) {
    try {
      const [roleRes, empRes] = await Promise.all([getRoles(), getEmployee(emp.id)])
      if (roleRes.code !== 0 || empRes.code !== 0) {
        message.error(roleRes.message || empRes.message || '获取角色数据失败')
        return
      }
      roleList = roleRes.data.items
      selectedRoleIds = ((empRes.data as Employee & { role_ids?: string[] }).role_ids) || []
      roleModal = { open: true, employee: emp }
    } catch (err: unknown) {
      message.error(getApiError(err, '获取角色数据失败'))
    }
  }

  async function handleSaveRoles() {
    const emp = roleModal.employee
    if (!emp) return
    savingRoles = true
    try {
      const res = await updateEmployeeRoles(emp.id, selectedRoleIds)
      if (res.code !== 0) {
        message.error(res.message || '保存角色失败')
        return
      }
      message.success('角色已更新')
      roleModal = { open: false, employee: null }
    } catch (err: unknown) {
      message.error(getApiError(err, '保存角色失败'))
    } finally {
      savingRoles = false
    }
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
      {#if !(row.id === $authStore.user?.id) && canRoleEdit}
        <Button type="link" size="small" tooltip="分配该员工的角色（权限随角色派生）" onClick={() => openRoleModal(row)}>
          <Icon name="setting" style="font-size:14px" />角色
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

  <!-- 角色分配弹窗 -->
  {#snippet roleFooter()}
    <Button tooltip="关闭弹窗，不保存修改" onClick={() => (roleModal = { open: false, employee: null })}>取消</Button>
    <Button type="primary" tooltip="保存角色分配" loading={savingRoles} onClick={handleSaveRoles}>保存角色</Button>
  {/snippet}

  {#snippet roleBody()}
    <div style="min-width:0">
      <span style="color:var(--ant-color-text-secondary);display:block;margin-bottom:12px">
        选择分配给该员工的角色（支持多选；最终权限 = 员工角色 + 部门角色的并集，含父子角色继承）。
      </span>
      {#if roleList.length === 0}
        <span style="color:var(--ant-color-warning)">暂无可分配的角色，请先在「角色管理」中创建</span>
      {:else}
        <div style="display:flex;flex-direction:column;gap:8px;max-height:360px;overflow-y:auto">
          {#each roleList as role}
            <label
              style="display:flex;align-items:center;gap:8px;padding:10px 12px;border:1px solid var(--ant-color-border-secondary);border-radius:6px;cursor:pointer"
            >
              <input
                type="checkbox"
                style="accent-color:var(--ant-color-primary)"
                checked={selectedRoleIds.includes(role.id)}
                onchange={() => {
                  selectedRoleIds = selectedRoleIds.includes(role.id)
                    ? selectedRoleIds.filter((id) => id !== role.id)
                    : [...selectedRoleIds, role.id]
                }}
              />
              <span style="flex:1;font-weight:500">
                {role.name}
                {#if role.is_system === 1}<span style="color:var(--ant-color-error);font-size:12px">内置</span>{/if}
                {#if role.parent_name}<span style="color:var(--ant-color-text-secondary);font-size:12px">继承自 {role.parent_name}</span>{/if}
              </span>
              <span style="color:var(--ant-color-text-secondary);font-size:12px">{role.permission_codes.length} 项权限</span>
            </label>
          {/each}
        </div>
      {/if}
    </div>
  {/snippet}

  <Modal
    open={roleModal.open}
    title={`角色分配 - ${roleModal.employee?.name || ''}`}
    onclose={() => (roleModal = { open: false, employee: null })}
    footer={roleFooter}
    width={620}
    bodyStyle="padding:16px 24px"
  >
    {@render roleBody()}
  </Modal>
{/if}
