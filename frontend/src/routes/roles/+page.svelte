<script lang="ts">
  // 角色管理（RBAC + 数据范围 + 部门角色继承）
  // 权限：role:manage
  import { onMount } from 'svelte'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { getApiError } from '$lib/api/client'
  import { t } from '$lib/i18n'
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

  const SCOPE_LABELS: Record<ScopeType, string> = $derived({
    all: t('scope.all'),
    subtree: t('scope.subtree'),
    department: t('scope.department'),
    self: t('scope.self'),
    custom: t('scope.custom'),
  })

  const SCOPE_OPTIONS = $derived((Object.keys(SCOPE_LABELS) as ScopeType[]).map((k) => ({
    value: k,
    label: k === 'all' ? t('scope.allUnlimited') : SCOPE_LABELS[k],
  })))

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
        message.error(res.message || t('roles.fetchFailed'))
        return
      }
      roles = res.data.items
    } catch (err: unknown) {
      message.error(getApiError(err, t('roles.fetchFailed')))
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
        label: `${r.name} (${SCOPE_LABELS[r.scope_type] || r.scope_type})`,
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
      message.error(t('roles.errName'))
      return
    }
    if (modal.scope_type === 'custom' && modal.scope_department_ids.length === 0) {
      message.error(t('roles.errCustomScope'))
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
          message.error(res.message || t('common.createdFailed'))
          return
        }
        message.success(t('common.createdSuccess'))
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
          message.error(res.message || t('common.updatedSuccess'))
          return
        }
        message.success(t('common.updatedSuccess'))
      }
      modal.open = false
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, t('common.savedFailed')))
    } finally {
      saving = false
    }
  }

  async function handleDelete(r: Role) {
    try {
      const res = await deleteRole(r.id)
      if (res.code !== 0) {
        message.error(res.message || t('common.deletedFailed'))
        return
      }
      message.success(t('common.deletedSuccess'))
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, t('common.deletedFailed')))
    }
  }

  const columns: TableColumn<Role>[] = $derived([
    { title: t('roles.roleName'), dataIndex: 'name', key: 'name', width: 150 },
    { title: t('roles.parent'), key: 'parent', width: 120, render: (r) => r.parent_name || '-' },
    {
      title: t('roles.scope'),
      key: 'scope',
      width: 130,
      render: (r) => SCOPE_LABELS[r.scope_type] || r.scope_type,
    },
    {
      title: t('roles.permCount'),
      key: 'permCount',
      width: 80,
      align: 'center',
      render: (r) => String(r.permission_codes.length),
    },
    {
      title: t('roles.memberCount'),
      key: 'member_count',
      width: 80,
      align: 'center',
      render: (r) => String(r.member_count),
    },
    { title: t('common.builtin'), key: 'system', width: 70, align: 'center', snippet: 'system' },
    { title: t('common.actions'), key: 'action', width: 150, snippet: 'action' },
  ])
</script>

{#if !$authStore.permissions.includes('role:manage')}
  <Result status="403" title="403" subTitle={t('common.noAccess')}>
    {#snippet extra()}
      <Button type="primary" tooltip={t('common.backHome')} onClick={() => goto('/')}>{t('common.backHome')}</Button>
    {/snippet}
  </Result>
{:else}
  {#snippet system(row: Role)}
    {#if row.is_system === 1}
      <Tag color="red">{t('common.builtin')}</Tag>
    {:else}
      <Tag>{t('common.custom')}</Tag>
    {/if}
  {/snippet}

  {#snippet action(row: Role)}
    <Space size="small" wrap={true}>
      <Button type="link" size="small" tooltip={t('roles.editTooltip')} onClick={() => openEdit(row)}>
        <Icon name="edit" style="font-size:14px" />{t('common.edit')}
      </Button>
      {#if row.is_system !== 1}
        <Popconfirm title={t('roles.deleteConfirm')} onConfirm={() => handleDelete(row)}>
          <Button type="link" size="small" danger={true} tooltip={t('roles.deleteTooltip')}>
            <Icon name="delete" style="font-size:14px" />{t('common.delete')}
          </Button>
        </Popconfirm>
      {/if}
    </Space>
  {/snippet}

  <div class="page-scroll" style="height:100%;overflow:auto">
    <Card title={t('roles.title')} bodyStyle="padding:16px 24px">
      {#snippet extra()}
        <Button type="primary" tooltip={t('roles.createTooltip')} onClick={openCreate}>
          <Icon name="plus" style="font-size:14px" />{t('roles.addNew')}
        </Button>
      {/snippet}
      <div style="margin-bottom:12px;padding:8px 12px;border:1px solid #d1e9ff;border-radius:6px;background:#e6f4ff;color:#0958d9;font-size:13px;line-height:1.6">
        {t('roles.info')}
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
    title={modal.mode === 'create' ? t('roles.create') : t('roles.edit')}
    onclose={() => (modal.open = false)}
    onOk={handleSave}
    confirmLoading={saving}
    okText={t('common.save')}
    width={860}
    bodyStyle="padding:16px 24px"
  >
    <Form class="ant-form-vertical">
      <FormItem label={t('roles.roleName')} required={true}>
        <Input value={modal.name} disabled={modal.is_system === 1} onInput={(v) => (modal = { ...modal, name: v })} placeholder={t('roles.roleNamePlaceholder')} />
      </FormItem>
      <div style="display:grid;grid-template-columns:1fr 1fr;column-gap:16px">
        <FormItem label={t('roles.parentRole')}>
          <Select
            value={modal.parent_id || undefined}
            options={parentOptions()}
            allowClear={true}
            disabled={modal.is_system === 1}
            placeholder={t('roles.parentPlaceholder')}
            onChange={(v) => (modal = { ...modal, parent_id: String(v || '') })}
          />
        </FormItem>
        <FormItem label={t('roles.scopeLabel')}>
          <Select
            value={modal.scope_type}
            options={SCOPE_OPTIONS}
            disabled={modal.is_system === 1}
            onChange={(v) => (modal = { ...modal, scope_type: String(v || 'self') as ScopeType })}
          />
        </FormItem>
      </div>

      {#if modal.scope_type === 'custom'}
        <FormItem label={t('roles.customScope')}>
          <Select
            value={modal.scope_department_ids as never[]}
            options={deptOptions}
            multiple={true}
            allowClear={true}
            disabled={modal.is_system === 1}
            placeholder={t('roles.customPlaceholder')}
            onChange={(v) => (modal = { ...modal, scope_department_ids: (Array.isArray(v) ? v : []) as string[] })}
          />
        </FormItem>
      {/if}

      <FormItem label={t('roles.permissions')}>
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

      <FormItem label={t('roles.description')}>
        <Input
          type="textarea"
          rows={2}
          value={modal.description}
          onInput={(v) => (modal = { ...modal, description: v })}
          placeholder={t('roles.descriptionPlaceholder')}
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
