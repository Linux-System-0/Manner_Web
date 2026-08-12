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
  // 员工管理 - 列表（复刻 React 版 frontend/src/pages/employees/List.tsx）
  // 角色/部门机制已移除：仅保留直接权限授权。
  import { onMount } from 'svelte'
  import { get } from 'svelte/store'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { preferencesStore, formatTimestamp } from '$lib/stores/preferences'
  import { t } from '$lib/i18n'
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
        message.error(res.message || t('employees.fetchFailed'))
        return
      }
      data = res.data.items
      total = res.data.total
    } catch (err: unknown) {
      message.error(getApiError(err, t('employees.fetchFailed')))
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
        message.error(res.message || t('employees.openConvFailed'))
        return
      }
      goto(`/chat?conv=${res.data.id}`)
    } catch (err: unknown) {
      message.error(getApiError(err, t('employees.openConvFailed')))
    }
  }

  // ---- 删除 ----
  async function handleDelete(id: string) {
    try {
      const res = await deleteEmployee(id)
      if (res.code !== 0) {
        message.error(res.message || t('employees.deleteFailed'))
        return
      }
      message.success(t('employees.deletedSuccess'))
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, t('employees.deleteFailed')))
    }
  }

  // ---- 查看敏感信息 ----
  // 第一次确认（操作入口弹窗）：提示查看将记录日志，确认后进入敏感信息页。
  async function openSensitive(emp: Employee) {
    const ok = await modal.confirm({
      title: t('employees.viewFullConfirmTitle'),
      content: t('employees.viewFullConfirmContent'),
      okText: t('employees.confirmView'),
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
      message.error(t('employees.errNewPassword'))
      return
    }
    if (resetModal.password.length < 8) {
      message.error(t('employees.errNewPasswordLen'))
      return
    }
    if (resetModal.password !== resetModal.confirm) {
      message.error(t('employees.errPasswordMismatch'))
      return
    }
    resetting = true
    try {
      const res = await resetPassword(emp.id, resetModal.password)
      if (res.code !== 0) {
        message.error(res.message || t('employees.resetFailed'))
        return
      }
      // F-02: 重置后的密码即该员工下次登录的一次性初始密码
      message.success(t('employees.resetSuccess'))
      // F-08: 初始密码通过页面内弹窗展示
      pwdResult = { open: true, password: resetModal.password, name: emp.name || emp.username }
      resetModal = { open: false, employee: null, password: '', confirm: '' }
    } catch (err: unknown) {
      message.error(getApiError(err, t('employees.resetFailed')))
    } finally {
      resetting = false
    }
  }

  async function copyResetPwd() {
    try {
      await navigator.clipboard.writeText(pwdResult.password)
      message.success(t('common.copied'))
    } catch {
      message.error(t('common.copyFailed'))
    }
  }

  // ---- 角色分配 ----
  async function openRoleModal(emp: Employee) {
    try {
      const [roleRes, empRes] = await Promise.all([getRoles(), getEmployee(emp.id)])
      if (roleRes.code !== 0 || empRes.code !== 0) {
        message.error(roleRes.message || empRes.message || t('employees.rolesFetchFailed'))
        return
      }
      roleList = roleRes.data.items
      selectedRoleIds = ((empRes.data as Employee & { role_ids?: string[] }).role_ids) || []
      roleModal = { open: true, employee: emp }
    } catch (err: unknown) {
      message.error(getApiError(err, t('employees.rolesFetchFailed')))
    }
  }

  async function handleSaveRoles() {
    const emp = roleModal.employee
    if (!emp) return
    savingRoles = true
    try {
      const res = await updateEmployeeRoles(emp.id, selectedRoleIds)
      if (res.code !== 0) {
        message.error(res.message || t('employees.saveRolesFailed'))
        return
      }
      message.success(t('employees.rolesUpdated'))
      roleModal = { open: false, employee: null }
    } catch (err: unknown) {
      message.error(getApiError(err, t('employees.saveRolesFailed')))
    } finally {
      savingRoles = false
    }
  }

  // ---- 表格列 ----
  const columns: TableColumn<Employee>[] = $derived([
    { title: t('employees.username'), dataIndex: 'username', key: 'username', width: 110 },
    { title: t('employees.name'), key: 'name', width: 100, snippet: 'name' },
    {
      title: t('employees.title'),
      key: 'title',
      width: 100,
      render: (r) => r.title || '-',
    },
    {
      title: t('employees.department'),
      key: 'departments',
      width: 140,
      render: (r) => r.departments || '-',
    },
    {
      title: t('employees.phone'),
      key: 'phone',
      width: 120,
      render: (r) => r.phone || '-',
    },
    { title: t('employees.status'), key: 'status', width: 80, align: 'center', snippet: 'status' },
    {
      title: t('employees.createdAt'),
      key: 'created_at',
      width: 150,
      render: (r) => formatTimestamp(r.created_at, get(preferencesStore)),
    },
    { title: t('employees.actions'), key: 'action', width: 420, snippet: 'action' },
  ])

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
  <Result status="403" title="403" subTitle={t('common.noAccess')}>
    {#snippet extra()}
      <Button type="primary" tooltip={t('common.backHome')} onClick={() => goto('/')}>{t('common.backHome')}</Button>
    {/snippet}
  </Result>
{:else}
  {#snippet name(row: Employee)}
    {#if row.id === $authStore.user?.id}
      {row.name}
    {:else}
      <Button type="link" size="small" tooltip={t('employees.openChat', { name: row.name })} onClick={() => openChat(row)} style="padding:0;font-weight:500">
        {row.name}
      </Button>
    {/if}
  {/snippet}

  {#snippet status(row: Employee)}
    <Tag color={row.status === 1 ? 'success' : 'default'}>{row.status === 1 ? t('common.onJob') : t('common.offJob')}</Tag>
  {/snippet}

  {#snippet action(row: Employee)}
    <Space size="small" wrap={true}>
      {#if !(row.id === $authStore.user?.id) && canEdit}
        <Button type="link" size="small" tooltip={t('employees.editTooltip')} onClick={() => goto(`/employees/${row.id}/edit`)}>
          <Icon name="edit" style="font-size:14px" />{t('common.edit')}
        </Button>
      {/if}
      {#if !(row.id === $authStore.user?.id) && canRoleEdit}
        <Button type="link" size="small" tooltip={t('employees.roleTooltip')} onClick={() => openRoleModal(row)}>
          <Icon name="setting" style="font-size:14px" />{t('common.role')}
        </Button>
      {/if}
      {#if !(row.id === $authStore.user?.id) && canPassword}
        <Button type="link" size="small" tooltip={t('employees.passwordTooltip')} onClick={() => openResetModal(row)}>
          <Icon name="key" style="font-size:14px" />{t('common.password')}
        </Button>
      {/if}
      {#if !(row.id === $authStore.user?.id) && canDelete}
        <Popconfirm title={t('employees.deleteConfirm')} onConfirm={() => handleDelete(row.id)}>
          <Button type="link" size="small" danger={true} tooltip={t('employees.deleteTooltip')}>
            <Icon name="delete" style="font-size:14px" />{t('common.delete')}
          </Button>
        </Popconfirm>
      {/if}
      {#if canViewSensitive}
        <Button type="link" size="small" tooltip={t('employees.sensitiveTooltip')} onClick={() => openSensitive(row)}>
          <Icon name="eye" style="font-size:14px" />{t('employees.viewFull')}
        </Button>
      {/if}
    </Space>
  {/snippet}

  <div class="page-scroll" style="height:100%;overflow:auto">
    <Card bodyStyle="padding:16px 24px" style="margin-bottom:16px">
      <div style="display:flex;align-items:center;gap:12px;flex-wrap:wrap">
        <Input
          placeholder={t('employees.searchPlaceholder')}
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
          placeholder={t('employees.deptFilter')}
          width="200px"
          onChange={(v) => (deptFilter = String(v || ''))}
        />
        <Space size="small">
          <Button type="primary" tooltip={t('common.search')} onClick={handleSearch}>{t('common.search')}</Button>
          <Button tooltip={t('common.reset')} onClick={handleReset}>{t('common.reset')}</Button>
        </Space>
        <div style="flex:1"></div>
        {#if canCreate}
          <Button type="primary" tooltip={t('employees.createTooltip')} onClick={() => goto('/employees/new')}>
            <Icon name="plus" style="font-size:14px" />{t('employees.addNew')}
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
        showTotal: (n) => t('common.total', { count: n }),
      }}
      snippets={{ status, action }}
    />
  </div>

  <!-- 重置密码弹窗 -->
  <Modal
    open={resetModal.open}
    title={t('employees.resetTitle', { name: resetModal.employee?.name || '' })}
    onclose={() => (resetModal = { open: false, employee: null, password: '', confirm: '' })}
    onOk={handleResetPassword}
    confirmLoading={resetting}
    okText={t('employees.confirmReset')}
    cancelText={t('common.cancel')}
  >
    <div style="display:flex;flex-direction:column;gap:12px">
      <Input
        type="password"
        placeholder={t('employees.errNewPassword')}
        value={resetModal.password}
        onInput={(v) => (resetModal = { ...resetModal, password: v })}
      />
      <Input
        type="password"
        placeholder={t('login.errConfirmNewPassword')}
        value={resetModal.confirm}
        onInput={(v) => (resetModal = { ...resetModal, confirm: v })}
      />
    </div>
  </Modal>

  <!-- F-08: 重置密码成功 - 初始密码展示弹窗（一次性密码，禁止遮罩点击误关） -->
  <Modal
    open={pwdResult.open}
    title={t('employees.resetResultTitle', { name: pwdResult.name })}
    onclose={() => (pwdResult = { open: false, password: '', name: '' })}
    onOk={() => (pwdResult = { open: false, password: '', name: '' })}
    okText={t('employees.gotIt')}
    cancelText={t('common.closeBtn')}
    maskClosable={false}
  >
    <div style="display:flex;flex-direction:column;gap:12px">
      <span style="color:var(--ant-color-text-secondary)">
        {t('employees.initialPasswordNote')}
      </span>
      <div style="display:flex;align-items:center;gap:8px">
        <code
          style="flex:1;padding:8px 12px;border:1px solid var(--ant-color-border-secondary);border-radius:6px;background:var(--ant-color-fill-secondary);font-size:16px;letter-spacing:1px;user-select:all"
        >{pwdResult.password}</code>
        <Button size="small" tooltip={t('employees.copyInitialPwd')} onClick={copyResetPwd}>{t('common.copy')}</Button>
      </div>
      <span style="color:var(--ant-color-warning)">{t('employees.nextLoginNote')}</span>
    </div>
  </Modal>

  <!-- 角色分配弹窗 -->
  {#snippet roleFooter()}
    <Button tooltip={t('common.closeDialogNoSave')} onClick={() => (roleModal = { open: false, employee: null })}>{t('common.cancel')}</Button>
    <Button type="primary" tooltip={t('employees.saveRoles')} loading={savingRoles} onClick={handleSaveRoles}>{t('employees.saveRoles')}</Button>
  {/snippet}

  {#snippet roleBody()}
    <div style="min-width:0">
      <span style="color:var(--ant-color-text-secondary);display:block;margin-bottom:12px">
        {t('employees.roleAssignHint')}
      </span>
      {#if roleList.length === 0}
        <span style="color:var(--ant-color-warning)">{t('employees.noAssignableRoles')}</span>
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
                {#if role.is_system === 1}<span style="color:var(--ant-color-error);font-size:12px">{t('common.builtin')}</span>{/if}
                {#if role.parent_name}<span style="color:var(--ant-color-text-secondary);font-size:12px">{t('common.inheritedFrom', { name: role.parent_name })}</span>{/if}
              </span>
              <span style="color:var(--ant-color-text-secondary);font-size:12px">{t('employees.permissionCount', { count: role.permission_codes.length })}</span>
            </label>
          {/each}
        </div>
      {/if}
    </div>
  {/snippet}

  <Modal
    open={roleModal.open}
    title={t('employees.roleAssignTitle', { name: roleModal.employee?.name || '' })}
    onclose={() => (roleModal = { open: false, employee: null })}
    footer={roleFooter}
    width={620}
    bodyStyle="padding:16px 24px"
  >
    {@render roleBody()}
  </Modal>
{/if}
