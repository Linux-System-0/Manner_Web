<script lang="ts">
  // 角色管理（RBAC + 数据范围 + 部门角色继承）
  // 权限：role:manage
  import { onMount } from 'svelte'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { getApiError } from '$lib/api/client'
  import { getRoles, createRole, updateRole, deleteRole } from '$lib/api/roles'
  import { getPermissions } from '$lib/api/system'
  import { getDepartments } from '$lib/api/departments'
  import type { Role, PermissionModule, ScopeType } from '$lib/types'
  import Table from '$lib/components/Table.svelte'
  import type { TableColumn } from '$lib/components/Table.svelte'
  import Card from '$lib/components/Card.svelte'
  import Button from '$lib/components/Button.svelte'
  import Input from '$lib/components/Input.svelte'
  import Select from '$lib/components/Select.svelte'
  import FormItem from '$lib/components/FormItem.svelte'
  import Form from '$lib/components/Form.svelte'
  import Modal from '$lib/components/Modal.svelte'
  import Checkbox from '$lib/components/Checkbox.svelte'
  import Tag from '$lib/components/Tag.svelte'
  import Popconfirm from '$lib/components/Popconfirm.svelte'
  import Result from '$lib/components/Result.svelte'
  import Space from '$lib/components/Space.svelte'
  import { Icon } from '$lib/icons'
  import { message } from '$lib/components/message'

  const SCOPE_LABELS: Record<ScopeType, string> = {
    all: '全部数据',
    subtree: '本部门及子部门',
    department: '本部门',
    self: '仅本人',
    custom: '指定部门',
  }

  const SCOPE_OPTIONS = (Object.keys(SCOPE_LABELS) as ScopeType[]).map((k) => ({
    value: k,
    label: `${SCOPE_LABELS[k]}${k === 'all' ? '（不受限）' : ''}`,
  }))

  let roles = $state<Role[]>([])
  let loading = $state(false)
  let permModules = $state<PermissionModule[]>([])
  let deptOptions = $state<{ value: string; label: string }[]>([])

  let modal = $state({
    open: false,
    mode: 'create' as 'create' | 'edit',
    id: '',
    name: '',
    parent_id: '',
    scope_type: 'self' as ScopeType,
    description: '',
    permission_codes: [] as string[],
    scope_department_ids: [] as string[],
    is_system: 0,
  })
  let saving = $state(false)

  async function fetchData() {
    loading = true
    try {
      const res = await getRoles()
      if (res.code !== 0) {
        message.error(res.message || '获取角色列表失败')
        return
      }
      roles = res.data.items
    } catch (err: unknown) {
      message.error(getApiError(err, '获取角色列表失败'))
    } finally {
      loading = false
    }
  }

  onMount(async () => {
    if (!$authStore.permissions.includes('role:manage')) return
    await fetchData()
    getPermissions()
      .then((res) => {
        if (res.code === 0) permModules = res.data.modules
      })
      .catch(() => {})
    getDepartments()
      .then((res) => {
        if (res.code === 0) {
          deptOptions = res.data.items.map((d) => ({ value: d.id, label: d.name }))
        }
      })
      .catch(() => {})
  })

  /** 父角色选项：排除自身及其后代（防环），is_system 除外 */
  function parentOptions(): { value: string; label: string; disabled?: boolean }[] {
    const forbidden = new Set<string>()
    if (modal.id) {
      const findSubtree = (id: string) => {
        forbidden.add(id)
        for (const r of roles) if (r.parent_id === id) findSubtree(r.id)
      }
      findSubtree(modal.id)
    }
    return roles
      .filter((r) => r.id !== modal.id)
      .map((r) => ({
        value: r.id,
        label: `${r.name}（${SCOPE_LABELS[r.scope_type] || r.scope_type}）`,
        disabled: forbidden.has(r.id) || r.is_system === 1,
      }))
  }

  function openCreate() {
    modal = {
      open: true,
      mode: 'create',
      id: '',
      name: '',
      parent_id: '',
      scope_type: 'self',
      description: '',
      permission_codes: [],
      scope_department_ids: [],
      is_system: 0,
    }
  }

  function openEdit(r: Role) {
    modal = {
      open: true,
      mode: 'edit',
      id: r.id,
      name: r.name,
      parent_id: r.parent_id || '',
      scope_type: r.scope_type,
      description: r.description || '',
      permission_codes: [...r.permission_codes],
      scope_department_ids: [...r.scope_department_ids],
      is_system: r.is_system,
    }
  }

  function togglePerm(code: string) {
    modal = {
      ...modal,
      permission_codes: modal.permission_codes.includes(code)
        ? modal.permission_codes.filter((c) => c !== code)
        : [...modal.permission_codes, code],
    }
  }

  function toggleModuleAll(mod: PermissionModule) {
    const codes = mod.permissions.map((p) => p.code)
    const allChecked = codes.every((c) => modal.permission_codes.includes(c))
    modal = {
      ...modal,
      permission_codes: allChecked
        ? modal.permission_codes.filter((c) => !codes.includes(c))
        : [...new Set([...modal.permission_codes, ...codes])],
    }
  }

  function moduleState(mod: PermissionModule): { all: boolean; partial: boolean } {
    const codes = mod.permissions.map((p) => p.code)
    const checked = codes.filter((c) => modal.permission_codes.includes(c)).length
    return { all: checked === codes.length, partial: checked > 0 && checked < codes.length }
  }

  async function handleSave() {
    const name = modal.name.trim()
    if (!name) {
      message.error('请输入角色名称')
      return
    }
    if (modal.scope_type === 'custom' && modal.scope_department_ids.length === 0) {
      message.error('「指定部门」范围至少选择一个部门')
      return
    }
    saving = true
    try {
      if (modal.mode === 'create') {
        const res = await createRole({
          name,
          parent_id: modal.parent_id || undefined,
          scope_type: modal.scope_type,
          description: modal.description.trim() || undefined,
          permission_codes: modal.permission_codes,
          scope_department_ids:
            modal.scope_type === 'custom' ? modal.scope_department_ids : [],
        })
        if (res.code !== 0) {
          message.error(res.message || '创建失败')
          return
        }
        message.success('创建成功')
      } else {
        const isSystem = modal.is_system === 1
        const res = await updateRole(modal.id, {
          // is_system 角色后端仅允许修改描述
          ...(isSystem ? {} : { name: name || undefined }),
          ...(isSystem
            ? {}
            : {
                parent_id: modal.parent_id || null,
                scope_type: modal.scope_type,
                permission_codes: modal.permission_codes,
                scope_department_ids:
                  modal.scope_type === 'custom' ? modal.scope_department_ids : [],
              }),
          description: modal.description.trim() || null,
        })
        if (res.code !== 0) {
          message.error(res.message || '更新失败')
          return
        }
        message.success('更新成功')
      }
      modal.open = false
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, '保存失败'))
    } finally {
      saving = false
    }
  }

  async function handleDelete(r: Role) {
    try {
      const res = await deleteRole(r.id)
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

  const columns: TableColumn<Role>[] = [
    { title: '角色名称', dataIndex: 'name', key: 'name', width: 150 },
    { title: '父角色', key: 'parent', width: 120, render: (r) => r.parent_name || '-' },
    {
      title: '数据范围',
      key: 'scope',
      width: 130,
      render: (r) => SCOPE_LABELS[r.scope_type] || r.scope_type,
    },
    {
      title: '权限数',
      key: 'permCount',
      width: 80,
      align: 'center',
      render: (r) => String(r.permission_codes.length),
    },
    {
      title: '成员数',
      key: 'member_count',
      width: 80,
      align: 'center',
      render: (r) => String(r.member_count),
    },
    { title: '内置', key: 'system', width: 70, align: 'center', snippet: 'system' },
    { title: '操作', key: 'action', width: 150, snippet: 'action' },
  ]
</script>

{#if !$authStore.permissions.includes('role:manage')}
  <Result status="403" title="403" subTitle="抱歉，你无权访问该页面">
    {#snippet extra()}
      <Button type="primary" tooltip="返回系统首页" onClick={() => goto('/')}>返回首页</Button>
    {/snippet}
  </Result>
{:else}
  {#snippet system(row: Role)}
    {#if row.is_system === 1}
      <Tag color="red">内置</Tag>
    {:else}
      <Tag>自定义</Tag>
    {/if}
  {/snippet}

  {#snippet action(row: Role)}
    <Space size="small" wrap={true}>
      <Button type="link" size="small" tooltip="编辑该角色（内置角色仅可改描述）" onClick={() => openEdit(row)}>
        <Icon name="edit" style="font-size:14px" />编辑
      </Button>
      {#if row.is_system !== 1}
        <Popconfirm title="确定要删除该角色吗？持有该角色的员工将失去对应权限。" onConfirm={() => handleDelete(row)}>
          <Button type="link" size="small" danger={true} tooltip="删除该角色">
            <Icon name="delete" style="font-size:14px" />删除
          </Button>
        </Popconfirm>
      {/if}
    </Space>
  {/snippet}

  <div class="page-scroll" style="height:100%;overflow:auto">
    <Card title="角色管理" bodyStyle="padding:16px 24px">
      {#snippet extra()}
        <Button type="primary" tooltip="创建新角色" onClick={openCreate}>
          <Icon name="plus" style="font-size:14px" />新增角色
        </Button>
      {/snippet}
      <div style="margin-bottom:12px;padding:8px 12px;border:1px solid #d1e9ff;border-radius:6px;background:#e6f4ff;color:#0958d9;font-size:13px;line-height:1.6">
        员工最终权限 = 员工角色 + 部门角色（含父子继承）的并集；数据范围仅作用于员工数据类权限
        （查看列表/详情/敏感信息），super_admin 内置角色拥有全部权限且不可修改。
      </div>
      <Table
        columns={columns}
        dataSource={roles as never[]}
        rowKey="id"
        loading={loading}
        snippets={{ system, action }}
      />
    </Card>
  </div>

  <!-- 新增/编辑角色弹窗 -->
  <Modal
    open={modal.open}
    title={modal.mode === 'create' ? '新增角色' : '编辑角色'}
    onclose={() => (modal.open = false)}
    onOk={handleSave}
    confirmLoading={saving}
    okText="保存"
    width={860}
    bodyStyle="padding:16px 24px"
  >
    <Form class="ant-form-vertical">
      <FormItem label="角色名称" required={true}>
        <Input value={modal.name} disabled={modal.is_system === 1} onInput={(v) => (modal = { ...modal, name: v })} placeholder="如：部门主管" />
      </FormItem>
      <div style="display:grid;grid-template-columns:1fr 1fr;column-gap:16px">
        <FormItem label="父角色（继承权限）">
          <Select
            value={modal.parent_id || undefined}
            options={parentOptions()}
            allowClear={true}
            disabled={modal.is_system === 1}
            placeholder="不选则为顶级角色"
            onChange={(v) => (modal = { ...modal, parent_id: String(v || '') })}
          />
        </FormItem>
        <FormItem label="数据范围">
          <Select
            value={modal.scope_type}
            options={SCOPE_OPTIONS}
            disabled={modal.is_system === 1}
            onChange={(v) => (modal = { ...modal, scope_type: String(v || 'self') as ScopeType })}
          />
        </FormItem>
      </div>

      {#if modal.scope_type === 'custom'}
        <FormItem label="指定部门（custom 范围）">
          <Select
            value={modal.scope_department_ids as never[]}
            options={deptOptions}
            multiple={true}
            allowClear={true}
            disabled={modal.is_system === 1}
            placeholder="请选择可见部门（可多选）"
            onChange={(v) => (modal = { ...modal, scope_department_ids: (Array.isArray(v) ? v : []) as string[] })}
          />
        </FormItem>
      {/if}

      <FormItem label="权限（勾选授予该角色的权限码）">
        <div
          style="display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:6px 16px;border:1px solid var(--ant-color-border-secondary);border-radius:6px;padding:12px;max-height:280px;overflow-y:auto"
        >
          {#each permModules as mod}
            <div>
              <Checkbox
                checked={moduleState(mod).all}
                indeterminate={moduleState(mod).partial}
                disabled={modal.is_system === 1}
                label={mod.module_name}
                onChange={() => toggleModuleAll(mod)}
              />
              <div style="display:flex;flex-direction:column;gap:2px;padding-left:20px;margin-top:2px">
                {#each mod.permissions as p}
                  <Checkbox
                    checked={modal.permission_codes.includes(p.code)}
                    disabled={modal.is_system === 1}
                    label={p.name}
                    onChange={() => togglePerm(p.code)}
                  />
                {/each}
              </div>
            </div>
          {/each}
        </div>
      </FormItem>

      <FormItem label="描述">
        <Input
          type="textarea"
          rows={2}
          value={modal.description}
          onInput={(v) => (modal = { ...modal, description: v })}
          placeholder="角色用途说明（可选）"
        />
      </FormItem>
    </Form>
  </Modal>
{/if}

<style>
  :global(.ant-form-vertical .ant-form-item) {
    flex-direction: column;
    align-items: stretch;
    row-gap: 4px;
  }
  :global(.ant-form-vertical .ant-form-item-label) {
    flex: none !important;
    width: 100% !important;
    padding-right: 0 !important;
    text-align: left !important;
    line-height: 1.5715 !important;
  }
</style>
