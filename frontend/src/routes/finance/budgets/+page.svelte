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
  // 预算管理：按部门 × 期间（年/月）设置额度，实时显示已用/剩余/使用率，超支红色预警
  // 权限：finance:budget_manage
  import { onMount } from 'svelte'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { t } from '$lib/i18n'
  import { getApiError } from '$lib/api/client'
  import { getBudgets, createBudget, updateBudget, deleteBudget } from '$lib/api/finance'
  import { getDepartments } from '$lib/api/departments'
  import type { Budget } from '$lib/types'
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
  import { Icon } from '$lib/icons'
  import { message } from '$lib/components/message'

  const PAGE_SIZE = 10

  let canManage = $derived($authStore.permissions.includes('finance:budget_manage'))

  let data = $state<Budget[]>([])
  let loading = $state(false)
  let total = $state(0)
  let periodTypeFilter = $state('')
  let periodValueFilter = $state('')
  let deptFilter = $state('')
  let deptOptions = $state<{ value: string; label: string }[]>([])
  let params = $state({ page: 1, page_size: PAGE_SIZE, period_type: '', period_value: '', department_id: '' })

  async function fetchData() {
    loading = true
    try {
      const res = await getBudgets({
        page: params.page,
        page_size: params.page_size,
        period_type: params.period_type || undefined,
        period_value: params.period_value || undefined,
        department_id: params.department_id || undefined,
      })
      if (res.code !== 0) {
        message.error(res.message || t('finance.budgetFetchFailed'))
        return
      }
      data = res.data.items
      total = res.data.total
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.budgetFetchFailed')))
    } finally {
      loading = false
    }
  }

  function handleSearch() {
    params = {
      page: 1,
      page_size: PAGE_SIZE,
      period_type: periodTypeFilter,
      period_value: periodValueFilter.trim(),
      department_id: deptFilter,
    }
    fetchData()
  }

  function handleReset() {
    periodTypeFilter = ''
    periodValueFilter = ''
    deptFilter = ''
    handleSearch()
  }

  function handleTableChange(page: number) {
    params = { ...params, page }
    fetchData()
  }

  // ---- 新增 / 编辑弹窗 ----
  let formModal = $state({
    open: false,
    mode: 'create' as 'create' | 'edit',
    id: '',
    department_id: '',
    period_type: 'month' as 'year' | 'month',
    period_value: '',
    amount: '',
  })
  let saving = $state(false)

  function openCreate() {
    formModal = {
      open: true,
      mode: 'create',
      id: '',
      department_id: '',
      period_type: 'month',
      period_value: new Date().toISOString().slice(0, 7),
      amount: '',
    }
  }

  function openEdit(row: Budget) {
    formModal = {
      open: true,
      mode: 'edit',
      id: row.id,
      department_id: row.department_id,
      period_type: row.period_type,
      period_value: row.period_value,
      amount: String(row.amount),
    }
  }

  async function handleSaveForm() {
    const f = formModal
    if (!f.department_id) {
      message.error(t('finance.errBudgetDepartment'))
      return
    }
    if (!f.period_value.trim()) {
      message.error(t('finance.errBudgetPeriod'))
      return
    }
    const amount = Number(f.amount)
    if (!Number.isFinite(amount) || amount <= 0) {
      message.error(t('finance.errBudgetAmount'))
      return
    }
    saving = true
    try {
      const payload = {
        department_id: f.department_id,
        period_type: f.period_type,
        period_value: f.period_value.trim(),
        amount,
      }
      const res =
        f.mode === 'create' ? await createBudget(payload) : await updateBudget(f.id, payload)
      if (res.code !== 0) {
        message.error(
          res.message || (res.code === 409 ? t('finance.budgetDup') : t('finance.operationFailed')),
        )
        return
      }
      message.success(
        f.mode === 'create' ? t('finance.budgetCreated') : t('finance.budgetUpdated'),
      )
      formModal.open = false
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.operationFailed')))
    } finally {
      saving = false
    }
  }

  async function handleDelete(row: Budget) {
    try {
      const res = await deleteBudget(row.id)
      if (res.code !== 0) {
        message.error(res.message || t('finance.operationFailed'))
        return
      }
      message.success(t('finance.budgetDeleted'))
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.operationFailed')))
    }
  }

  function periodLabel(row: Budget): string {
    return `${row.period_type === 'year' ? t('finance.budgetYear') : t('finance.budgetMonth')} ${row.period_value}`
  }

  const columns: TableColumn<Budget>[] = $derived([
    { title: t('finance.budgetDepartment'), key: 'department_name', width: 160, render: (r) => r.department_name },
    { title: t('finance.budgetPeriodType'), key: 'period', width: 140, render: (r) => periodLabel(r) },
    {
      title: t('finance.budgetAmount'),
      key: 'amount',
      width: 120,
      align: 'right',
      render: (r) => `¥${r.amount.toFixed(2)}`,
    },
    {
      title: t('finance.budgetSpent'),
      key: 'spent',
      width: 120,
      align: 'right',
      render: (r) => `¥${r.spent.toFixed(2)}`,
    },
    {
      title: t('finance.budgetRemaining'),
      key: 'remaining',
      width: 130,
      align: 'right',
      snippet: 'remaining',
    },
    {
      title: t('finance.budgetProgress'),
      key: 'progress',
      width: 200,
      snippet: 'progress',
    },
    {
      title: t('finance.status'),
      key: 'overrun',
      width: 100,
      align: 'center',
      snippet: 'overrun',
    },
    { title: t('finance.actions'), key: 'action', width: 150, snippet: 'action' },
  ])

  onMount(() => {
    if (!canManage) return
    fetchData()
    getDepartments()
      .then((res) => {
        if (res.code === 0) deptOptions = res.data.items.map((d) => ({ value: d.id, label: d.name }))
      })
      .catch(() => {})
  })
</script>

{#if !canManage}
  <Result status="403" title="403" subTitle={t('common.noAccess')}>
    {#snippet extra()}
      <Button type="primary" tooltip={t('common.backHome')} onClick={() => goto('/')}>{t('common.backHome')}</Button>
    {/snippet}
  </Result>
{:else}
  {#snippet remaining(row: Budget)}
    <span style={row.remaining < 0 ? 'color:var(--ant-color-error);font-weight:600' : ''}>
      ¥{row.remaining.toFixed(2)}
    </span>
  {/snippet}

  {#snippet progress(row: Budget)}
    <div style="display:flex;align-items:center;gap:8px">
      <div
        style="flex:1;height:10px;background:var(--ant-color-fill-tertiary);border-radius:5px;overflow:hidden"
      >
        <div
          style="height:100%;width:{Math.min(Math.max(row.amount > 0 ? (row.spent / row.amount) * 100 : 0, 0), 100)}%;background:{row.spent > row.amount ? 'var(--ant-color-error)' : 'var(--ant-color-primary)'};border-radius:5px;transition:width .3s"
        ></div>
      </div>
      <span style="font-size:12px;color:var(--ant-color-text-secondary);white-space:nowrap">
        {t('finance.budgetProgress', { percent: row.amount > 0 ? ((row.spent / row.amount) * 100).toFixed(1) : '0.0' })}
      </span>
    </div>
  {/snippet}

  {#snippet overrun(row: Budget)}
    {#if row.spent > row.amount}
      <Tag color="red">{t('finance.budgetOverrun')}</Tag>
    {:else if row.remaining < row.amount * 0.2}
      <Tag color="orange">≥80%</Tag>
    {:else}
      <Tag color="green">OK</Tag>
    {/if}
  {/snippet}

  {#snippet action(row: Budget)}
    <Space size="small">
      <Button type="link" size="small" tooltip={t('common.edit')} onClick={() => openEdit(row)}>
        <Icon name="edit" style="font-size:14px" />{t('common.edit')}
      </Button>
      <Popconfirm title={t('finance.budgetDeleteConfirm')} onConfirm={() => handleDelete(row)}>
        <Button type="link" size="small" danger={true} tooltip={t('common.delete')}>
          <Icon name="delete" style="font-size:14px" />{t('common.delete')}
        </Button>
      </Popconfirm>
    </Space>
  {/snippet}

  <div class="page-scroll" style="height:100%;overflow:auto">
    <Card bodyStyle="padding:16px 24px" style="margin-bottom:16px">
      <div style="display:flex;align-items:center;gap:12px;flex-wrap:wrap">
        <Select
          value={periodTypeFilter || undefined}
          options={[
            { value: '', label: t('finance.allStatus') },
            { value: 'year', label: t('finance.budgetYear') },
            { value: 'month', label: t('finance.budgetMonth') },
          ]}
          allowClear={true}
          placeholder={t('finance.budgetPeriodType')}
          width="130px"
          onChange={(v) => (periodTypeFilter = String(v || ''))}
        />
        <Input
          placeholder={t('finance.budgetPeriodMonthPlaceholder')}
          value={periodValueFilter}
          onInput={(v) => (periodValueFilter = v)}
          onEnter={handleSearch}
          style="width:150px;flex-shrink:0"
        />
        <Select
          value={deptFilter || undefined}
          options={deptOptions}
          allowClear={true}
          placeholder={t('finance.budgetDepartment')}
          width="170px"
          onChange={(v) => (deptFilter = String(v || ''))}
        />
        <Space size="small">
          <Button type="primary" tooltip={t('common.search')} onClick={handleSearch}>{t('common.search')}</Button>
          <Button tooltip={t('common.reset')} onClick={handleReset}>{t('common.reset')}</Button>
        </Space>
        <div style="flex:1"></div>
        <Button type="primary" tooltip={t('finance.budgetCreate')} onClick={openCreate}>
          <Icon name="plus" style="font-size:14px" />{t('finance.budgetCreate')}
        </Button>
      </div>
    </Card>

    <Table
      columns={columns}
      dataSource={data as never[]}
      rowKey="id"
      loading={loading}
      scroll={{ x: 1150 }}
      pagination={{
        current: params.page,
        pageSize: params.page_size,
        total,
        onChange: handleTableChange,
        showTotal: (n) => t('common.total', { count: n }),
      }}
      snippets={{ remaining, progress, overrun, action }}
    />
  </div>

  <!-- 新增/编辑弹窗 -->
  <Modal
    open={formModal.open}
    title={formModal.mode === 'create' ? t('finance.budgetCreate') : t('finance.edit')}
    onclose={() => (formModal.open = false)}
    onOk={handleSaveForm}
    confirmLoading={saving}
    okText={formModal.mode === 'create' ? t('finance.budgetCreate') : t('common.save')}
    cancelText={t('common.cancel')}
    width={520}
  >
    <div style="display:flex;flex-direction:column;gap:4px">
      <FormItem label={t('finance.budgetDepartment')}>
        <Select
          value={formModal.department_id || undefined}
          options={deptOptions}
          placeholder={t('finance.budgetDepartment')}
          onChange={(v) => (formModal = { ...formModal, department_id: String(v || '') })}
        />
      </FormItem>
      <FormItem label={t('finance.budgetPeriodType')}>
        <Select
          value={formModal.period_type}
          options={[
            { value: 'month', label: t('finance.budgetMonth') },
            { value: 'year', label: t('finance.budgetYear') },
          ]}
          onChange={(v) => (formModal = { ...formModal, period_type: (v as 'year' | 'month') || 'month' })}
        />
      </FormItem>
      <FormItem label={t('finance.budgetPeriodValue')}>
        <Input
          placeholder={formModal.period_type === 'year' ? t('finance.budgetPeriodYearPlaceholder') : t('finance.budgetPeriodMonthPlaceholder')}
          value={formModal.period_value}
          onInput={(v) => (formModal = { ...formModal, period_value: v })}
        />
      </FormItem>
      <FormItem label={t('finance.budgetAmount')}>
        <Input
          placeholder={t('finance.budgetAmount')}
          value={formModal.amount}
          onInput={(v) => (formModal = { ...formModal, amount: v })}
        />
      </FormItem>
    </div>
  </Modal>
{/if}
