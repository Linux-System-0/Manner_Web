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
  // 任务管理：员工创建/完成个人任务（负责人可标记完成/未完成），
  // 持有 task:view_all 的管理员可查看全员任务并按负责人筛选。
  // 权限：task:create（创建）、task:view_all（查看全员）、task:manage（管理任意）
  import { onMount } from 'svelte'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { preferencesStore, formatTimestamp } from '$lib/stores/preferences'
  import { get } from 'svelte/store'
  import { t } from '$lib/i18n'
  import { getApiError } from '$lib/api/client'
  import { getTasks, getTaskStats, createTask, updateTask, deleteTask } from '$lib/api/tasks'
  import { getEmployees } from '$lib/api/employees'
  import type { Task } from '$lib/types'
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
  import FormItem from '$lib/components/FormItem.svelte'
  import Statistic from '$lib/components/Statistic.svelte'
  import Row from '$lib/components/Row.svelte'
  import Col from '$lib/components/Col.svelte'
  import { Icon } from '$lib/icons'
  import { message } from '$lib/components/message'
  import { modal } from '$lib/components/modal'

  const PAGE_SIZE = 10

  let canViewAll = $derived($authStore.permissions.includes('task:view_all'))
  let canManage = $derived($authStore.permissions.includes('task:manage'))
  let canCreate = $derived($authStore.permissions.includes('task:create'))

  let data = $state<Task[]>([])
  let loading = $state(false)
  let total = $state(0)
  let stats = $state({ total: 0, todo: 0, done: 0, overdue: 0 })
  let keyword = $state('')
  let statusFilter = $state('')
  let assigneeFilter = $state('')
  let scopeFilter = $state<'' | 'mine'>('')
  let employeeOptions = $state<{ value: string; label: string }[]>([])
  let params = $state({ page: 1, page_size: PAGE_SIZE, status: '', assignee_id: '', scope: '' })

  async function fetchData() {
    loading = true
    try {
      const res = await getTasks({
        page: params.page,
        page_size: params.page_size,
        status: (params.status as 'todo' | 'done' | '') || undefined,
        assignee_id: params.assignee_id || undefined,
        scope: (params.scope as 'all' | 'mine' | undefined) || undefined,
      })
      if (res.code !== 0) {
        message.error(res.message || t('tasks.fetchFailed'))
        return
      }
      data = res.data.items
      total = res.data.total
      if (typeof res.data.can_view_all === 'boolean') {
        // 服务端为准（权限可能中途变化）。
        canViewAll = res.data.can_view_all
      }
    } catch (err: unknown) {
      message.error(getApiError(err, t('tasks.fetchFailed')))
    } finally {
      loading = false
    }
  }

  async function fetchStats() {
    try {
      const res = await getTaskStats()
      if (res.code === 0) stats = res.data
    } catch {
      /* ignore */
    }
  }

  async function fetchEmployees() {
    try {
      const res = await getEmployees({ page: 1, page_size: 100 })
      if (res.code === 0) {
        employeeOptions = res.data.items.map((e) => ({ value: e.id, label: `${e.name}（${e.username}）` }))
      }
    } catch {
      /* ignore */
    }
  }

  function handleSearch() {
    params = {
      page: 1,
      page_size: PAGE_SIZE,
      status: statusFilter,
      assignee_id: assigneeFilter,
      scope: scopeFilter,
    }
    fetchData()
  }

  function handleReset() {
    keyword = ''
    statusFilter = ''
    assigneeFilter = ''
    scopeFilter = ''
    handleSearch()
  }

  function handleTableChange(page: number) {
    params = { ...params, page }
    fetchData()
  }

  // ---- 新建 / 编辑弹窗 ----
  let formModal = $state({
    open: false,
    mode: 'create' as 'create' | 'edit',
    id: '',
    title: '',
    description: '',
    assignee_id: '',
    due_date: '',
  })
  let saving = $state(false)

  function openCreate() {
    formModal = {
      open: true,
      mode: 'create',
      id: '',
      title: '',
      description: '',
      assignee_id: $authStore.user?.id || '',
      due_date: '',
    }
    fetchEmployees()
  }

  function openEdit(row: Task) {
    formModal = {
      open: true,
      mode: 'edit',
      id: row.id,
      title: row.title,
      description: row.description || '',
      assignee_id: row.assignee_id,
      due_date: row.due_date || '',
    }
    fetchEmployees()
  }

  async function handleSaveForm() {
    const f = formModal
    if (!f.title.trim()) {
      message.error(t('tasks.errTitle'))
      return
    }
    saving = true
    try {
      const payload = {
        title: f.title.trim(),
        description: f.description || undefined,
        assignee_id: f.assignee_id || undefined,
        due_date: f.due_date || undefined,
      }
      const res =
        f.mode === 'create' ? await createTask(payload) : await updateTask(f.id, payload)
      if (res.code !== 0) {
        message.error(res.message || t('tasks.operationFailed'))
        return
      }
      message.success(f.mode === 'create' ? t('tasks.createdSuccess') : t('tasks.updatedSuccess'))
      formModal.open = false
      fetchData()
      fetchStats()
    } catch (err: unknown) {
      message.error(getApiError(err, t('tasks.operationFailed')))
    } finally {
      saving = false
    }
  }

  // ---- 标记完成 / 未完成 ----
  async function toggleStatus(row: Task) {
    const target = row.status === 'done' ? 'todo' : 'done'
    try {
      const res = await updateTask(row.id, { status: target })
      if (res.code !== 0) {
        message.error(res.message || t('tasks.operationFailed'))
        return
      }
      message.success(
        target === 'done' ? t('tasks.doneSuccess') : t('tasks.todoSuccess'),
      )
      fetchData()
      fetchStats()
    } catch (err: unknown) {
      message.error(getApiError(err, t('tasks.operationFailed')))
    }
  }

  async function handleDelete(row: Task) {
    const ok = await modal.confirm({ title: t('tasks.delete'), content: t('tasks.deleteConfirm') })
    if (!ok) return
    try {
      const res = await deleteTask(row.id)
      if (res.code !== 0) {
        message.error(res.message || t('tasks.operationFailed'))
        return
      }
      message.success(t('tasks.deletedSuccess'))
      fetchData()
      fetchStats()
    } catch (err: unknown) {
      message.error(getApiError(err, t('tasks.operationFailed')))
    }
  }

  function canOperate(row: Task): boolean {
    return (
      canManage ||
      row.creator_id === $authStore.user?.id ||
      row.assignee_id === $authStore.user?.id
    )
  }

  function isOverdue(row: Task): boolean {
    if (row.status !== 'todo' || !row.due_date) return false
    return row.due_date < new Date().toISOString().slice(0, 10)
  }

  const columns: TableColumn<Task>[] = $derived([
    { title: t('tasks.title'), key: 'title', width: 220, render: (r) => r.title },
    {
      title: t('tasks.description'),
      key: 'description',
      width: 220,
      render: (r) => r.description || '-',
    },
    {
      title: t('tasks.assignee'),
      key: 'assignee_name',
      width: 100,
      render: (r) => r.assignee_name,
    },
    {
      title: t('tasks.creator'),
      key: 'creator_name',
      width: 100,
      render: (r) => r.creator_name,
    },
    {
      title: t('tasks.dueDate'),
      key: 'due_date',
      width: 110,
      snippet: 'due',
    },
    {
      title: t('tasks.status'),
      key: 'status',
      width: 90,
      align: 'center',
      snippet: 'status',
    },
    {
      title: t('tasks.createdAt'),
      key: 'created_at',
      width: 150,
      render: (r) => formatTimestamp(r.created_at, get(preferencesStore)),
    },
    { title: t('tasks.actions'), key: 'action', width: 200, snippet: 'action' },
  ])

  onMount(async () => {
    await fetchData()
    fetchStats()
  })
</script>

{#if !canCreate}
  <Result status="403" title="403" subTitle={t('common.noAccess')}>
    {#snippet extra()}
      <Button type="primary" tooltip={t('common.backHome')} onClick={() => goto('/')}>{t('common.backHome')}</Button>
    {/snippet}
  </Result>
{:else}
  {#snippet due(row: Task)}
    <span style={isOverdue(row) ? 'color:var(--ant-color-error);font-weight:600' : ''}>
      {row.due_date || '-'}
      {#if isOverdue(row)}<Icon name="exclamation-circle" style="font-size:12px" />{/if}
    </span>
  {/snippet}

  {#snippet status(row: Task)}
    <Tag color={row.status === 'done' ? 'green' : isOverdue(row) ? 'red' : 'orange'}>
      {row.status === 'done' ? t('tasks.statusDone') : isOverdue(row) ? t('tasks.statusOverdue') : t('tasks.statusTodo')}
    </Tag>
  {/snippet}

  {#snippet action(row: Task)}
    <Space size="small" wrap={true}>
      {#if canOperate(row)}
        <Button
          type="link"
          size="small"
          tooltip={row.status === 'done' ? t('tasks.markTodo') : t('tasks.markDone')}
          onClick={() => toggleStatus(row)}
        >
          <Icon name={row.status === 'done' ? 'reload' : 'check'} style="font-size:14px" />
          {row.status === 'done' ? t('tasks.markTodo') : t('tasks.markDone')}
        </Button>
        <Button type="link" size="small" tooltip={t('common.edit')} onClick={() => openEdit(row)}>
          <Icon name="edit" style="font-size:14px" />{t('common.edit')}
        </Button>
        <Popconfirm title={t('tasks.deleteConfirm')} onConfirm={() => handleDelete(row)}>
          <Button type="link" size="small" danger={true} tooltip={t('common.delete')}>
            <Icon name="delete" style="font-size:14px" />{t('common.delete')}
          </Button>
        </Popconfirm>
      {/if}
    </Space>
  {/snippet}

  <div class="page-scroll" style="height:100%;overflow:auto">
    {#if canViewAll}
      <Card bodyStyle="padding:24px" style="margin-bottom:16px">
        <Row gutter={[16, 16]}>
          <Col span={6}>
            <Statistic title={t('tasks.statTotal')} value={stats.total} />
          </Col>
          <Col span={6}>
            <Statistic title={t('tasks.statTodo')} value={stats.todo} />
          </Col>
          <Col span={6}>
            <Statistic title={t('tasks.statDone')} value={stats.done} />
          </Col>
          <Col span={6}>
            <Statistic
              title={t('tasks.statOverdue')}
              value={stats.overdue}
              style={stats.overdue > 0 ? 'color:var(--ant-color-error)' : ''}
            />
          </Col>
        </Row>
      </Card>
    {/if}

    <Card bodyStyle="padding:16px 24px" style="margin-bottom:16px">
      <div style="display:flex;align-items:center;gap:12px;flex-wrap:wrap">
        <Input
          placeholder={t('tasks.searchPlaceholder')}
          prefix="search"
          value={keyword}
          onInput={(v) => (keyword = v)}
          onEnter={handleSearch}
          style="width:200px;flex-shrink:0"
        />
        <Select
          value={statusFilter || undefined}
          options={[
            { value: '', label: t('tasks.allStatus') },
            { value: 'todo', label: t('tasks.statusTodo') },
            { value: 'done', label: t('tasks.statusDone') },
          ]}
          allowClear={true}
          placeholder={t('tasks.status')}
          width="130px"
          onChange={(v) => (statusFilter = String(v || ''))}
        />
        {#if canViewAll}
          <Select
            value={scopeFilter || undefined}
            options={[
              { value: '', label: t('tasks.scopeAll') },
              { value: 'mine', label: t('tasks.scopeMine') },
            ]}
            allowClear={true}
            placeholder={t('tasks.scope')}
            width="130px"
            onChange={(v) => (scopeFilter = (v as '' | 'mine') || '')}
          />
          <Select
            value={assigneeFilter || undefined}
            options={employeeOptions}
            allowClear={true}
            placeholder={t('tasks.assignee')}
            width="170px"
            onChange={(v) => (assigneeFilter = String(v || ''))}
          />
        {/if}
        <Space size="small">
          <Button type="primary" tooltip={t('common.search')} onClick={handleSearch}>{t('common.search')}</Button>
          <Button tooltip={t('common.reset')} onClick={handleReset}>{t('common.reset')}</Button>
        </Space>
        <div style="flex:1"></div>
        <Button type="primary" tooltip={t('tasks.createTooltip')} onClick={openCreate}>
          <Icon name="plus" style="font-size:14px" />{t('tasks.create')}
        </Button>
      </div>
    </Card>

    <Table
      columns={columns}
      dataSource={data as never[]}
      rowKey="id"
      loading={loading}
      scroll={{ x: 1250 }}
      pagination={{
        current: params.page,
        pageSize: params.page_size,
        total,
        onChange: handleTableChange,
        showTotal: (n) => t('common.total', { count: n }),
      }}
      snippets={{ due, status, action }}
    />
  </div>

  <!-- 新建/编辑弹窗 -->
  <Modal
    open={formModal.open}
    title={formModal.mode === 'create' ? t('tasks.create') : t('tasks.edit')}
    onclose={() => (formModal.open = false)}
    onOk={handleSaveForm}
    confirmLoading={saving}
    okText={formModal.mode === 'create' ? t('tasks.create') : t('common.save')}
    cancelText={t('common.cancel')}
    width={560}
  >
    <div style="display:flex;flex-direction:column;gap:4px">
      <FormItem label={t('tasks.title')}>
        <Input
          placeholder={t('tasks.titlePlaceholder')}
          value={formModal.title}
          onInput={(v) => (formModal = { ...formModal, title: v })}
        />
      </FormItem>
      <FormItem label={t('tasks.description')}>
        <Input
          placeholder={t('tasks.descriptionPlaceholder')}
          value={formModal.description}
          onInput={(v) => (formModal = { ...formModal, description: v })}
        />
      </FormItem>
      <FormItem label={t('tasks.assignee')}>
        <Select
          value={formModal.assignee_id || undefined}
          options={employeeOptions}
          placeholder={t('tasks.assigneePlaceholder')}
          onChange={(v) => (formModal = { ...formModal, assignee_id: String(v || '') })}
        />
      </FormItem>
      <FormItem label={t('tasks.dueDate')}>
        <Input
          placeholder="YYYY-MM-DD"
          value={formModal.due_date}
          onInput={(v) => (formModal = { ...formModal, due_date: v })}
        />
      </FormItem>
    </div>
  </Modal>
{/if}
