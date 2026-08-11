<script lang="ts">
  // 部门管理：树形展示 + CRUD + 成员管理
  // 权限：department:list（查看）、department:create/edit/delete（管理）、
  //       department:view（查看成员）、employee:edit（调整员工归属）
  import { onMount } from 'svelte'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { getApiError } from '$lib/api/client'
  import {
    getDepartments,
    createDepartment,
    updateDepartment,
    deleteDepartment,
    getDepartmentMembers,
    updateEmployeeDepartments,
  } from '$lib/api/departments'
  import { getEmployees, getEmployee } from '$lib/api/employees'
  import { getRoles, getDepartmentRoles, updateDepartmentRoles } from '$lib/api/roles'
  import type { Department, DepartmentMember, Employee, Role } from '$lib/types'
  import Card from '$lib/components/Card.svelte'
  import Tree from '$lib/components/Tree.svelte'
  import type { TreeNode } from '$lib/components/Tree.svelte'
  import Button from '$lib/components/Button.svelte'
  import Input from '$lib/components/Input.svelte'
  import Select from '$lib/components/Select.svelte'
  import FormItem from '$lib/components/FormItem.svelte'
  import Form from '$lib/components/Form.svelte'
  import Modal from '$lib/components/Modal.svelte'
  import Table from '$lib/components/Table.svelte'
  import type { TableColumn } from '$lib/components/Table.svelte'
  import Tag from '$lib/components/Tag.svelte'
  import Empty from '$lib/components/Empty.svelte'
  import Result from '$lib/components/Result.svelte'
  import Space from '$lib/components/Space.svelte'
  import { Icon } from '$lib/icons'
  import { message } from '$lib/components/message'
  import { modal } from '$lib/components/modal'

  let depts = $state<Department[]>([])
  let loading = $state(false)
  let selectedKey = $state<string>('')

  // 员工选择器（负责人/成员共用）
  let employees = $state<{ id: string; label: string }[]>([])
  let loadingEmployees = $state(false)

  let employeeOptions = $derived(
    employees.map((e) => ({ value: e.id, label: e.label })),
  )

  let canCreate = $derived($authStore.permissions.includes('department:create'))
  let canEdit = $derived($authStore.permissions.includes('department:edit'))
  let canDelete = $derived($authStore.permissions.includes('department:delete'))
  let canViewMembers = $derived($authStore.permissions.includes('department:view'))
  let canManageEmployeeDept = $derived(
    $authStore.permissions.includes('employee:edit') &&
      $authStore.permissions.includes('employee:view'),
  )
  let canManageRoles = $derived($authStore.permissions.includes('role:manage'))

  // 部门角色绑定弹窗
  let roleModal = $state({ open: false, deptId: '', deptName: '' })
  let roleList = $state<Role[]>([])
  let selectedDeptRoleIds = $state<string[]>([])
  let savingDeptRoles = $state(false)

  async function openDeptRoles(d: Department) {
    roleModal = { open: true, deptId: d.id, deptName: d.name }
    selectedDeptRoleIds = []
    const [roleRes, deptRoleRes] = await Promise.all([getRoles(), getDepartmentRoles(d.id)])
    if (roleRes.code === 0) roleList = roleRes.data.items
    if (deptRoleRes.code === 0) {
      selectedDeptRoleIds = deptRoleRes.data.items.map((r) => r.id)
    }
  }

  async function handleSaveDeptRoles() {
    savingDeptRoles = true
    try {
      const res = await updateDepartmentRoles(roleModal.deptId, selectedDeptRoleIds)
      if (res.code !== 0) {
        message.error(res.message || '保存角色失败')
        return
      }
      message.success('角色绑定已更新')
      roleModal.open = false
      fetchDepartments()
    } catch (err: unknown) {
      message.error(getApiError(err, '保存角色失败'))
    } finally {
      savingDeptRoles = false
    }
  }

  function buildTree(list: Department[]): TreeNode[] {
    const map = new Map<string, Department>()
    for (const d of list) map.set(d.id, d)
    const roots: TreeNode[] = []
    const childrenOf = new Map<string, TreeNode[]>()
    for (const d of list) {
      const node: TreeNode = {
        key: d.id,
        title: `${d.name}${d.member_count > 0 ? ` (${d.member_count})` : ''}${d.leader_names ? ` · 负责人:${d.leader_names}` : ''}${d.role_names ? ` · 角色:${d.role_names}` : ''}`,
      }
      if (d.parent_id && map.has(d.parent_id)) {
        const arr = childrenOf.get(d.parent_id) || []
        arr.push(node)
        childrenOf.set(d.parent_id, arr)
      } else {
        roots.push(node)
      }
    }
    const attach = (node: TreeNode): TreeNode => {
      const children = childrenOf.get(node.key)
      if (children?.length) node.children = children.map(attach)
      return node
    }
    return roots.map(attach)
  }

  let treeData = $derived(buildTree(depts))

  async function fetchDepartments() {
    loading = true
    try {
      const res = await getDepartments()
      if (res.code !== 0) {
        message.error(res.message || '获取部门列表失败')
        return
      }
      depts = res.data.items
    } catch (err: unknown) {
      message.error(getApiError(err, '获取部门列表失败'))
    } finally {
      loading = false
    }
  }

  async function fetchEmployees() {
    if (loadingEmployees) return
    loadingEmployees = true
    try {
      const res = await getEmployees({ page: 1, page_size: 100 })
      if (res.code === 0) {
        employees = res.data.items.map((e) => ({ id: e.id, label: `${e.name}（${e.username}）` }))
      }
    } catch {
      /* ignore */
    } finally {
      loadingEmployees = false
    }
  }

  onMount(async () => {
    if (!$authStore.permissions.includes('department:list')) return
    await fetchDepartments()
  })

  // ---- 部门 CRUD 弹窗 ----
  let formModal = $state({
    open: false,
    mode: 'create' as 'create' | 'edit',
    id: '',
    name: '',
    parent_id: '',
    leader_ids: [] as string[],
    sort_order: '0',
    // 编辑时用于级联选择，避免选择自身/子部门
    editingKey: '',
  })
  let saving = $state(false)

  function openCreate(parentId: string | null = null) {
    formModal = {
      open: true,
      mode: 'create',
      id: '',
      name: '',
      parent_id: parentId || '',
      leader_ids: [],
      sort_order: '0',
      editingKey: '',
    }
    fetchEmployees()
  }

  function openEdit(d: Department) {
    formModal = {
      open: true,
      mode: 'edit',
      id: d.id,
      name: d.name,
      parent_id: d.parent_id || '',
      leader_ids: d.leader_ids || [],
      sort_order: String(d.sort_order ?? 0),
      editingKey: d.id,
    }
    fetchEmployees()
  }

  /** 父部门下拉选项：禁用自身及其所有后代，防止环 */
  function parentOptions(): { value: string; label: string; disabled?: boolean }[] {
    const forbidden = new Set<string>()
    if (formModal.editingKey) {
      const findSubtree = (id: string) => {
        forbidden.add(id)
        for (const d of depts) if (d.parent_id === id) findSubtree(d.id)
      }
      findSubtree(formModal.editingKey)
    }
    return depts
      .filter((d) => d.id !== formModal.editingKey)
      .map((d) => ({ value: d.id, label: d.name, disabled: forbidden.has(d.id) }))
  }

  async function handleSave() {
    const name = formModal.name.trim()
    if (!name) {
      message.error('请输入部门名称')
      return
    }
    saving = true
    try {
      if (formModal.mode === 'create') {
        const res = await createDepartment({
          name,
          parent_id: formModal.parent_id || undefined,
          leader_ids: formModal.leader_ids,
          sort_order: Number(formModal.sort_order) || 0,
        })
        if (res.code !== 0) {
          message.error(res.message || '创建失败')
          return
        }
        message.success('创建成功')
      } else {
        const res = await updateDepartment(formModal.id, {
          name,
          parent_id: formModal.parent_id || null,
          leader_ids: formModal.leader_ids,
          sort_order: Number(formModal.sort_order) || 0,
        })
        if (res.code !== 0) {
          message.error(res.message || '更新失败')
          return
        }
        message.success('更新成功')
      }
      formModal.open = false
      fetchDepartments()
    } catch (err: unknown) {
      message.error(getApiError(err, '保存失败'))
    } finally {
      saving = false
    }
  }

  async function handleDelete(d: Department) {
    const ok = await modal.confirm({
      title: '删除部门',
      content: `确定要删除「${d.name}」吗？删除后该部门的成员归属将一并解除（存在子部门时需先删除子部门）。`,
      okText: '删除',
      okDanger: true,
    })
    if (!ok) return
    try {
      const res = await deleteDepartment(d.id)
      if (res.code !== 0) {
        message.error(res.message || '删除失败')
        return
      }
      message.success('删除成功')
      if (selectedKey === d.id) selectedKey = ''
      fetchDepartments()
    } catch (err: unknown) {
      message.error(getApiError(err, '删除失败'))
    }
  }

  // ---- 成员管理 ----
  let memberModal = $state({ open: false, deptId: '', deptName: '', leaderIds: [] as string[] })
  let members = $state<DepartmentMember[]>([])
  let loadingMembers = $state(false)
  let selectedMemberIds = $state<string[]>([])
  let savingMembers = $state(false)
  let togglingLeader = $state('')

  async function openMembers(d: Department) {
    memberModal = { open: true, deptId: d.id, deptName: d.name, leaderIds: d.leader_ids || [] }
    selectedMemberIds = []
    await fetchMembers(d.id)
    if (canManageEmployeeDept) fetchEmployees()
  }

  async function fetchMembers(deptId: string) {
    loadingMembers = true
    try {
      const res = await getDepartmentMembers(deptId)
      if (res.code !== 0) {
        message.error(res.message || '获取成员失败')
        return
      }
      members = res.data.items
    } catch (err: unknown) {
      message.error(getApiError(err, '获取成员失败'))
    } finally {
      loadingMembers = false
    }
  }

  const memberColumns: TableColumn<DepartmentMember>[] = [
    { title: '姓名', dataIndex: 'name', key: 'name', width: 120 },
    { title: '用户名', dataIndex: 'username', key: 'username', width: 120 },
    {
      title: '职位',
      key: 'title',
      width: 120,
      render: (r) => r.title || '-',
    },
    { title: '状态', key: 'status', width: 80, align: 'center', snippet: 'status' },
    { title: '身份', key: 'leader', width: 100, align: 'center', snippet: 'leader' },
    { title: '操作', key: 'action', width: 160, snippet: 'action' },
  ]

  /** 切换某成员的负责人身份 */
  async function toggleLeader(m: DepartmentMember) {
    const deptId = memberModal.deptId
    togglingLeader = m.id
    try {
      const isLeader = memberModal.leaderIds.includes(m.id)
      const next = isLeader
        ? memberModal.leaderIds.filter((id) => id !== m.id)
        : [...memberModal.leaderIds, m.id]
      const res = await updateDepartment(deptId, { leader_ids: next })
      if (res.code !== 0) {
        message.error(res.message || '更新负责人失败')
        return
      }
      memberModal = { ...memberModal, leaderIds: next }
      message.success(isLeader ? '已取消负责人' : '已设为负责人')
      await fetchMembers(deptId)
      fetchDepartments()
    } catch (err: unknown) {
      message.error(getApiError(err, '更新负责人失败'))
    } finally {
      togglingLeader = ''
    }
  }

  async function loadEmployeeDeptIds(empId: string): Promise<string[]> {
    const res = await getEmployee(empId)
    return (res.data as Employee & { department_ids?: string[] })?.department_ids || []
  }

  /** 添加选中员工到当前部门（保留各自原有归属） */
  async function handleAddMembers() {
    const deptId = memberModal.deptId
    if (selectedMemberIds.length === 0) {
      message.warning('请先选择员工')
      return
    }
    savingMembers = true
    try {
      for (const empId of selectedMemberIds) {
        const current = await loadEmployeeDeptIds(empId)
        if (!current.includes(deptId)) {
          const res = await updateEmployeeDepartments(empId, [...current, deptId])
          if (res.code !== 0) {
            message.error(res.message || `添加员工失败：${empId}`)
            return
          }
        }
      }
      message.success('添加成功')
      selectedMemberIds = []
      await fetchMembers(deptId)
      fetchDepartments()
    } catch (err: unknown) {
      message.error(getApiError(err, '添加员工失败'))
    } finally {
      savingMembers = false
    }
  }

  /** 将某成员移出当前部门（保留其在其他部门的归属） */
  async function removeMember(m: DepartmentMember) {
    const deptId = memberModal.deptId
    const ok = await modal.confirm({
      title: '移出部门',
      content: `确定将 ${m.name} 移出「${memberModal.deptName}」吗？`,
      okText: '移出',
      okDanger: true,
    })
    if (!ok) return
    try {
      const current = await loadEmployeeDeptIds(m.id)
      const next = current.filter((id) => id !== deptId)
      const res = await updateEmployeeDepartments(m.id, next)
      if (res.code !== 0) {
        message.error(res.message || '移出失败')
        return
      }
      message.success('已移出')
      await fetchMembers(deptId)
      fetchDepartments()
    } catch (err: unknown) {
      message.error(getApiError(err, '移出失败'))
    }
  }
</script>

{#if !$authStore.permissions.includes('department:list')}
  <Result status="403" title="403" subTitle="抱歉，你无权访问该页面">
    {#snippet extra()}
      <Button type="primary" tooltip="返回系统首页" onClick={() => goto('/')}>返回首页</Button>
    {/snippet}
  </Result>
{:else}
  {#snippet status(row: DepartmentMember)}
    <Tag color={row.status === 1 ? 'success' : 'default'}>{row.status === 1 ? '在职' : '禁用'}</Tag>
  {/snippet}

  {#snippet leader(row: DepartmentMember)}
    {#if row.is_leader === 1}
      <Tag color="orange">负责人</Tag>
    {:else}
      <Tag>成员</Tag>
    {/if}
  {/snippet}

  {#snippet action(row: DepartmentMember)}
    {#if canManageEmployeeDept}
      <Space size="small">
        <Button
          type="link"
          size="small"
          tooltip={row.is_leader === 1 ? '取消该成员的负责人身份' : '将该成员设为部门负责人'}
          loading={togglingLeader === row.id}
          onClick={() => toggleLeader(row)}
        >
          {#if row.is_leader === 1}
            取消负责人
          {:else}
            设为负责人
          {/if}
        </Button>
        <Button type="link" size="small" danger={true} tooltip="将该成员移出本部门" onClick={() => removeMember(row)}>
          移出部门
        </Button>
      </Space>
    {/if}
  {/snippet}

  <div class="page-scroll" style="height:100%;overflow:auto">
    <Card
      title="部门管理"
      bodyStyle="padding:16px 24px"
    >
      {#snippet extra()}
        {#if canCreate}
          <Button type="primary" tooltip="创建一个顶级部门" onClick={() => openCreate(null)}>
            <Icon name="plus" style="font-size:14px" />新增根部门
          </Button>
        {/if}
      {/snippet}
      {#if loading}
        <span>加载中...</span>
      {:else if depts.length === 0}
        <Empty description="暂无部门，点击右上角「新增根部门」创建" />
      {:else}
        <Tree
          treeData={treeData}
          defaultExpandAll={true}
          selectedKeys={selectedKey ? [selectedKey] : []}
          onSelect={(key) => (selectedKey = key)}
        >
          {#snippet action(node: TreeNode)}
            <Space size="small" wrap={true}>
              {#if canViewMembers}
                <Button type="link" size="small" tooltip="查看该部门的成员列表" onClick={() => {
                  const d = depts.find((x) => x.id === node.key)
                  if (d) openMembers(d)
                }}>
                  <Icon name="team" style="font-size:14px" />成员
                </Button>
              {/if}
              {#if canManageRoles}
                <Button type="link" size="small" tooltip="绑定该部门自动继承的角色" onClick={() => {
                  const d = depts.find((x) => x.id === node.key)
                  if (d) openDeptRoles(d)
                }}>
                  <Icon name="lock" style="font-size:14px" />角色
                </Button>
              {/if}
              {#if canCreate}
                <Button type="link" size="small" tooltip="在该部门下创建子部门" onClick={() => openCreate(node.key)}>
                  <Icon name="plus" style="font-size:14px" />子部门
                </Button>
              {/if}
              {#if canEdit}
                <Button type="link" size="small" tooltip="编辑该部门的名称与负责人" onClick={() => {
                  const d = depts.find((x) => x.id === node.key)
                  if (d) openEdit(d)
                }}>
                  <Icon name="edit" style="font-size:14px" />编辑
                </Button>
              {/if}
              {#if canDelete}
                <Button type="link" size="small" danger={true} tooltip="删除该部门及其子部门" onClick={() => {
                  const d = depts.find((x) => x.id === node.key)
                  if (d) handleDelete(d)
                }}>
                  <Icon name="delete" style="font-size:14px" />删除
                </Button>
              {/if}
            </Space>
          {/snippet}
        </Tree>
      {/if}
    </Card>
  </div>

  <!-- 部门新增/编辑弹窗 -->
  <Modal
    open={formModal.open}
    title={formModal.mode === 'create' ? '新增部门' : '编辑部门'}
    onclose={() => (formModal.open = false)}
    onOk={handleSave}
    confirmLoading={saving}
    okText="保存"
  >
    <Form>
      <FormItem label="部门名称" required={true}>
        <Input
          value={formModal.name}
          onInput={(v) => (formModal = { ...formModal, name: v })}
          placeholder="请输入部门名称"
        />
      </FormItem>

      <FormItem label="上级部门">
        <Select
          value={formModal.parent_id || undefined}
          options={parentOptions()}
          allowClear={true}
          placeholder="不选则为根部门"
          onChange={(v) => (formModal = { ...formModal, parent_id: String(v || '') })}
        />
      </FormItem>

      <FormItem label="部门负责人" extra="可多选">
        <Select
          value={formModal.leader_ids as never[]}
          options={employeeOptions}
          multiple={true}
          allowClear={true}
          placeholder="请选择负责人（可多选）"
          onChange={(v) => (formModal = { ...formModal, leader_ids: (Array.isArray(v) ? v : []) as string[] })}
        />
      </FormItem>

      <FormItem label="排序">
        <Input
          value={formModal.sort_order}
          onInput={(v) => (formModal = { ...formModal, sort_order: v })}
          placeholder="数字越小越靠前"
        />
      </FormItem>
    </Form>
  </Modal>

  <!-- 成员管理弹窗 -->
  <Modal
    open={memberModal.open}
    title={`部门成员 - ${memberModal.deptName}`}
    onclose={() => (memberModal.open = false)}
    width={760}
    bodyStyle="padding:16px 24px"
  >    {#snippet footer()}
      <Button tooltip="关闭弹窗" onClick={() => (memberModal.open = false)}>关闭</Button>
    {/snippet}

    {#if canManageEmployeeDept}
      <div style="margin-bottom:16px">
        <span style="color:var(--ant-color-text-secondary);display:block;margin-bottom:8px">
          添加员工到该部门（可多选，保留员工原有归属）
        </span>
        <Space>
          <Select
            value={selectedMemberIds as never[]}
            options={employeeOptions.filter(
              (e) =>
                e.value !== $authStore.user?.id &&
                !members.some((m) => m.id === e.value),
            )}
            multiple={true}
            placeholder="选择员工"
            onChange={(v) => (selectedMemberIds = (Array.isArray(v) ? v : []) as string[])}
          />
          <Button type="primary" loading={savingMembers} tooltip="将所选员工添加到该部门" onClick={handleAddMembers}>添加</Button>
        </Space>
      </div>
    {/if}

    <Table
      columns={memberColumns}
      dataSource={members as never[]}
      rowKey="id"
      loading={loadingMembers}
      snippets={{ status, leader, action }}
    />
  </Modal>

  <!-- 部门角色绑定弹窗 -->
  <Modal
    open={roleModal.open}
    title={`部门角色绑定 - ${roleModal.deptName}`}
    onclose={() => (roleModal.open = false)}
    width={560}
    bodyStyle="padding:16px 24px"
  >
    {#snippet footer()}
      <Button tooltip="关闭弹窗，不保存修改" onClick={() => (roleModal.open = false)}>取消</Button>
      <Button type="primary" tooltip="保存部门角色绑定" loading={savingDeptRoles} onClick={handleSaveDeptRoles}>保存</Button>
    {/snippet}
    <span style="color:var(--ant-color-text-secondary);display:block;margin-bottom:12px">
      绑定后，该部门所有员工自动获得所选角色的权限（super_admin 不允许经部门绑定）。
    </span>
    {#if roleList.length === 0}
      <span style="color:var(--ant-color-warning)">暂无可绑定的角色，请先在「角色管理」中创建</span>
    {:else}
      <div style="display:flex;flex-direction:column;gap:8px;max-height:320px;overflow-y:auto">
        {#each roleList as role}
          <label style="display:flex;align-items:center;gap:8px;padding:10px 12px;border:1px solid var(--ant-color-border-secondary);border-radius:6px;cursor:pointer">
            <input
              type="checkbox"
              style="accent-color:var(--ant-color-primary)"
              disabled={role.is_system === 1}
              checked={selectedDeptRoleIds.includes(role.id)}
              onchange={() => {
                selectedDeptRoleIds = selectedDeptRoleIds.includes(role.id)
                  ? selectedDeptRoleIds.filter((id) => id !== role.id)
                  : [...selectedDeptRoleIds, role.id]
              }}
            />
            <span style="flex:1;font-weight:500">
              {role.name}
              {#if role.is_system === 1}<span style="color:var(--ant-color-error);font-size:12px">内置（不可绑定）</span>{/if}
            </span>
            <span style="color:var(--ant-color-text-secondary);font-size:12px">{role.permission_codes.length} 项权限</span>
          </label>
        {/each}
      </div>
    {/if}
  </Modal>
{/if}
