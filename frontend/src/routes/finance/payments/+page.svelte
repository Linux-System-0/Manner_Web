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
  // 收付款记录：收支流水 CRUD + 方向/部门/日期过滤；报销付款自动生成的记录带来源标记
  // 权限：finance:payment_manage
  import { onMount } from 'svelte'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { preferencesStore, formatTimestamp } from '$lib/stores/preferences'
  import { get } from 'svelte/store'
  import { t } from '$lib/i18n'
  import { getApiError } from '$lib/api/client'
  import { getPayments, createPayment, updatePayment, deletePayment, exportReportUrl } from '$lib/api/finance'
  import { getDepartments } from '$lib/api/departments'
  import type { Payment } from '$lib/types'
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

  let canManage = $derived($authStore.permissions.includes('finance:payment_manage'))

  let data = $state<Payment[]>([])
  let loading = $state(false)
  let total = $state(0)
  let keyword = $state('')
  let directionFilter = $state('')
  let deptFilter = $state('')
  let fromFilter = $state('')
  let toFilter = $state('')
  let deptOptions = $state<{ value: string; label: string }[]>([])
  let params = $state({ page: 1, page_size: PAGE_SIZE, keyword: '', direction: '', department_id: '', from: '', to: '' })

  async function fetchData() {
    loading = true
    try {
      const res = await getPayments({
        page: params.page,
        page_size: params.page_size,
        keyword: params.keyword || undefined,
        direction: params.direction || undefined,
        department_id: params.department_id || undefined,
        from: params.from || undefined,
        to: params.to || undefined,
      })
      if (res.code !== 0) {
        message.error(res.message || t('finance.paymentFetchFailed'))
        return
      }
      data = res.data.items
      total = res.data.total
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.paymentFetchFailed')))
    } finally {
      loading = false
    }
  }

  function handleSearch() {
    params = {
      page: 1,
      page_size: PAGE_SIZE,
      keyword: keyword.trim(),
      direction: directionFilter,
      department_id: deptFilter,
      from: fromFilter,
      to: toFilter,
    }
    fetchData()
  }

  function handleReset() {
    keyword = ''
    directionFilter = ''
    deptFilter = ''
    fromFilter = ''
    toFilter = ''
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
    direction: 'expense' as 'income' | 'expense',
    category: '',
    amount: '',
    counterparty: '',
    occurred_at: '',
    department_id: '',
    remark: '',
  })
  let saving = $state(false)

  function openCreate() {
    formModal = {
      open: true,
      mode: 'create',
      id: '',
      direction: 'expense',
      category: '',
      amount: '',
      counterparty: '',
      occurred_at: new Date().toISOString().slice(0, 10),
      department_id: '',
      remark: '',
    }
  }

  function openEdit(row: Payment) {
    formModal = {
      open: true,
      mode: 'edit',
      id: row.id,
      direction: row.direction,
      category: row.category,
      amount: String(row.amount),
      counterparty: row.counterparty || '',
      occurred_at: row.occurred_at,
      department_id: row.department_id || '',
      remark: row.remark || '',
    }
  }

  async function handleSaveForm() {
    const f = formModal
    if (!f.direction) {
      message.error(t('finance.errPaymentDirection'))
      return
    }
    if (!f.category.trim()) {
      message.error(t('finance.errPaymentCategory'))
      return
    }
    const amount = Number(f.amount)
    if (!Number.isFinite(amount) || amount <= 0) {
      message.error(t('finance.errPaymentAmount'))
      return
    }
    if (!f.occurred_at) {
      message.error(t('finance.errPaymentDate'))
      return
    }
    saving = true
    try {
      const payload = {
        direction: f.direction,
        category: f.category.trim(),
        amount,
        counterparty: f.counterparty.trim() || undefined,
        occurred_at: f.occurred_at,
        department_id: f.department_id || undefined,
        remark: f.remark.trim() || undefined,
      }
      const res =
        f.mode === 'create' ? await createPayment(payload) : await updatePayment(f.id, payload)
      if (res.code !== 0) {
        message.error(res.message || t('finance.operationFailed'))
        return
      }
      message.success(
        f.mode === 'create' ? t('finance.paymentCreated') : t('finance.paymentUpdated'),
      )
      formModal.open = false
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.operationFailed')))
    } finally {
      saving = false
    }
  }

  async function handleDelete(row: Payment) {
    try {
      const res = await deletePayment(row.id)
      if (res.code !== 0) {
        message.error(res.message || t('finance.operationFailed'))
        return
      }
      message.success(t('finance.paymentDeleted'))
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.operationFailed')))
    }
  }

  const columns: TableColumn<Payment>[] = $derived([
    {
      title: t('finance.paymentDirection'),
      key: 'direction',
      width: 90,
      align: 'center',
      snippet: 'direction',
    },
    { title: t('finance.paymentCategory'), key: 'category', width: 110, render: (r) => r.category },
    {
      title: t('finance.paymentAmount'),
      key: 'amount',
      width: 120,
      align: 'right',
      render: (r) => `¥${r.amount.toFixed(2)}`,
    },
    {
      title: t('finance.paymentCounterparty'),
      key: 'counterparty',
      width: 160,
      render: (r) => r.counterparty || '-',
    },
    { title: t('finance.paymentOccurredAt'), key: 'occurred_at', width: 110, render: (r) => r.occurred_at },
    {
      title: t('finance.paymentDepartment'),
      key: 'department_name',
      width: 120,
      render: (r) => r.department_name || '-',
    },
    {
      title: t('finance.paymentRemark'),
      key: 'remark',
      width: 180,
      render: (r) => (r.reimbursement_id ? `${t('finance.paymentFromReimburse')}${r.remark ? '：' + r.remark : ''}` : r.remark || '-'),
    },
    {
      title: t('finance.createdAt'),
      key: 'created_at',
      width: 150,
      render: (r) => formatTimestamp(r.created_at, get(preferencesStore)),
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
  {#snippet direction(row: Payment)}
    <Tag color={row.direction === 'income' ? 'green' : 'red'}>
      {row.direction === 'income' ? t('finance.paymentIncome') : t('finance.paymentExpense')}
    </Tag>
  {/snippet}

  {#snippet action(row: Payment)}
    <Space size="small">
      {#if row.reimbursement_id}
        <span style="color:var(--ant-color-text-secondary);font-size:12px">{t('finance.paymentFromReimburse')}</span>
      {:else}
        <Button type="link" size="small" tooltip={t('common.edit')} onClick={() => openEdit(row)}>
          <Icon name="edit" style="font-size:14px" />{t('common.edit')}
        </Button>
        <Popconfirm title={t('finance.paymentDeleteConfirm')} onConfirm={() => handleDelete(row)}>
          <Button type="link" size="small" danger={true} tooltip={t('common.delete')}>
            <Icon name="delete" style="font-size:14px" />{t('common.delete')}
          </Button>
        </Popconfirm>
      {/if}
    </Space>
  {/snippet}

  <div class="page-scroll" style="height:100%;overflow:auto">
    <Card bodyStyle="padding:16px 24px" style="margin-bottom:16px">
      <div style="display:flex;align-items:center;gap:12px;flex-wrap:wrap">
        <Input
          placeholder={t('finance.paymentCategoryPlaceholder')}
          prefix="search"
          value={keyword}
          onInput={(v) => (keyword = v)}
          onEnter={handleSearch}
          style="width:220px;flex-shrink:0"
        />
        <Select
          value={directionFilter || undefined}
          options={[
            { value: '', label: t('finance.allStatus') },
            { value: 'income', label: t('finance.paymentIncome') },
            { value: 'expense', label: t('finance.paymentExpense') },
          ]}
          allowClear={true}
          placeholder={t('finance.paymentDirection')}
          width="110px"
          onChange={(v) => (directionFilter = String(v || ''))}
        />
        <Select
          value={deptFilter || undefined}
          options={deptOptions}
          allowClear={true}
          placeholder={t('finance.paymentDepartment')}
          width="150px"
          onChange={(v) => (deptFilter = String(v || ''))}
        />
        <Input placeholder={t('finance.reportFrom')} value={fromFilter} onInput={(v) => (fromFilter = v)} style="width:130px;flex-shrink:0" />
        <Input placeholder={t('finance.reportTo')} value={toFilter} onInput={(v) => (toFilter = v)} style="width:130px;flex-shrink:0" />
        <Space size="small">
          <Button type="primary" tooltip={t('common.search')} onClick={handleSearch}>{t('common.search')}</Button>
          <Button tooltip={t('common.reset')} onClick={handleReset}>{t('common.reset')}</Button>
        </Space>
        <div style="flex:1"></div>
        <a href={exportReportUrl('payments', {})} style="text-decoration:none">
          <Button tooltip={t('finance.reportExportPayments')}>
            <Icon name="upload" style="font-size:14px" />{t('common.export')}
          </Button>
        </a>
        <Button type="primary" tooltip={t('finance.paymentCreate')} onClick={openCreate}>
          <Icon name="plus" style="font-size:14px" />{t('finance.paymentCreate')}
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
      snippets={{ direction, action }}
    />
  </div>

  <!-- 新增/编辑弹窗 -->
  <Modal
    open={formModal.open}
    title={formModal.mode === 'create' ? t('finance.paymentCreate') : t('finance.edit')}
    onclose={() => (formModal.open = false)}
    onOk={handleSaveForm}
    confirmLoading={saving}
    okText={formModal.mode === 'create' ? t('finance.paymentCreate') : t('common.save')}
    cancelText={t('common.cancel')}
    width={560}
  >
    <div style="display:flex;flex-direction:column;gap:4px">
      <FormItem label={t('finance.paymentDirection')}>
        <Select
          value={formModal.direction}
          options={[
            { value: 'income', label: t('finance.paymentIncome') },
            { value: 'expense', label: t('finance.paymentExpense') },
          ]}
          onChange={(v) => (formModal = { ...formModal, direction: (v as 'income' | 'expense') || 'expense' })}
        />
      </FormItem>
      <FormItem label={t('finance.paymentCategory')}>
        <Input
          placeholder={t('finance.paymentCategoryPlaceholder')}
          value={formModal.category}
          onInput={(v) => (formModal = { ...formModal, category: v })}
        />
      </FormItem>
      <FormItem label={t('finance.paymentAmount')}>
        <Input
          placeholder={t('finance.paymentAmount')}
          value={formModal.amount}
          onInput={(v) => (formModal = { ...formModal, amount: v })}
        />
      </FormItem>
      <FormItem label={t('finance.paymentCounterparty')}>
        <Input
          placeholder={t('finance.paymentCounterpartyPlaceholder')}
          value={formModal.counterparty}
          onInput={(v) => (formModal = { ...formModal, counterparty: v })}
        />
      </FormItem>
      <FormItem label={t('finance.paymentOccurredAt')}>
        <Input
          placeholder="YYYY-MM-DD"
          value={formModal.occurred_at}
          onInput={(v) => (formModal = { ...formModal, occurred_at: v })}
        />
      </FormItem>
      <FormItem label={t('finance.paymentDepartment')}>
        <Select
          value={formModal.department_id || undefined}
          options={deptOptions}
          allowClear={true}
          placeholder={t('finance.paymentDepartment')}
          onChange={(v) => (formModal = { ...formModal, department_id: String(v || '') })}
        />
      </FormItem>
      <FormItem label={t('finance.paymentRemark')}>
        <Input
          placeholder={t('finance.paymentRemarkPlaceholder')}
          value={formModal.remark}
          onInput={(v) => (formModal = { ...formModal, remark: v })}
        />
      </FormItem>
    </div>
  </Modal>
{/if}
