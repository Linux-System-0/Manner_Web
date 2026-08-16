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
  // 报销管理：提交（草稿即提交）/ 编辑重新提交 / 审批 / 复核 / 付款 / 撤回 / 删除，全程留痕
  // 权限：finance:reimburse_view（查看）、finance:reimburse_create（提交）、
  //       finance:reimburse_approve（部门审批）、finance:reimburse_manage（财务复核/付款/删除）
  import { onMount } from 'svelte'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { preferencesStore, formatTimestamp } from '$lib/stores/preferences'
  import { get } from 'svelte/store'
  import { t } from '$lib/i18n'
  import { getApiError } from '$lib/api/client'
  import {
    getReimbursements,
    getReimbursement,
    createReimbursement,
    updateReimbursement,
    deleteReimbursement,
    approveReimbursement,
    reviewReimbursement,
    payReimbursement,
    withdrawReimbursement,
    getInvoices,
    exportReportUrl,
  } from '$lib/api/finance'
  import { getDepartments } from '$lib/api/departments'
  import type { Reimbursement, ReimbursementDetail, ReimbursementStatus, Invoice } from '$lib/types'
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
  import Descriptions from '$lib/components/Descriptions.svelte'
  import FormItem from '$lib/components/FormItem.svelte'
  import { Icon } from '$lib/icons'
  import { message } from '$lib/components/message'
  import { modal } from '$lib/components/modal'

  const PAGE_SIZE = 10

  const STATUS_ORDER: ReimbursementStatus[] = [
    'pending_leader',
    'pending_finance',
    'approved',
    'paid',
    'rejected',
    'withdrawn',
  ]
  const CATEGORIES = ['travel', 'office', 'meal', 'transport', 'other']

  let canSubmit = $derived($authStore.permissions.includes('finance:reimburse_create'))
  let canApprove = $derived($authStore.permissions.includes('finance:reimburse_approve'))
  let canManage = $derived($authStore.permissions.includes('finance:reimburse_manage'))
  let canView = $derived(
    canManage ||
      $authStore.permissions.includes('finance:reimburse_view') ||
      $authStore.permissions.includes('finance:reimburse_approve'),
  )

  let data = $state<Reimbursement[]>([])
  let loading = $state(false)
  let total = $state(0)
  let keyword = $state('')
  let statusFilter = $state<'' | ReimbursementStatus>('')
  let deptFilter = $state('')
  let deptOptions = $state<{ value: string; label: string }[]>([])
  let params = $state<{ page: number; page_size: number; status: '' | ReimbursementStatus; keyword: string; department_id: string }>({ page: 1, page_size: PAGE_SIZE, status: '', keyword: '', department_id: '' })

  // 发票选择器数据（未关联的发票）
  let invoices = $state<Invoice[]>([])
  let invoiceOptions = $derived(
    invoices
      .filter((i) => i.status === 'unused')
      .map((i) => ({ value: i.id, label: `${i.invoice_code}（${i.issuer_name} · ¥${i.amount.toFixed(2)}）` })),
  )

  async function fetchInvoicesForSelect() {
    try {
      const res = await getInvoices({ page: 1, page_size: 100 })
      if (res.code === 0) invoices = res.data.items
    } catch {
      /* ignore */
    }
  }

  async function fetchData() {
    loading = true
    try {
      const res = await getReimbursements({
        page: params.page,
        page_size: params.page_size,
        status: params.status || undefined,
        keyword: params.keyword || undefined,
        department_id: params.department_id || undefined,
      })
      if (res.code !== 0) {
        message.error(res.message || t('finance.fetchFailed'))
        return
      }
      data = res.data.items
      total = res.data.total
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.fetchFailed')))
    } finally {
      loading = false
    }
  }

  function handleSearch() {
    params = { page: 1, page_size: PAGE_SIZE, status: statusFilter, keyword: keyword.trim(), department_id: deptFilter }
    fetchData()
  }

  function handleReset() {
    keyword = ''
    statusFilter = ''
    deptFilter = ''
    handleSearch()
  }

  function handleTableChange(page: number) {
    params = { ...params, page }
    fetchData()
  }

  // ---- 提交 / 编辑弹窗 ----
  let formModal = $state({
    open: false,
    mode: 'create' as 'create' | 'edit',
    id: '',
    title: '',
    category: 'travel',
    amount: '',
    reason: '',
    invoice_ids: [] as string[],
  })
  let saving = $state(false)

  function openCreate() {
    formModal = {
      open: true,
      mode: 'create',
      id: '',
      title: '',
      category: 'travel',
      amount: '',
      reason: '',
      invoice_ids: [],
    }
    fetchInvoicesForSelect()
  }

  async function openEdit(row: Reimbursement) {
    try {
      const res = await getReimbursement(row.id)
      if (res.code !== 0 || !res.data) {
        message.error(res.message || t('finance.fetchFailed'))
        return
      }
      const d = res.data
      formModal = {
        open: true,
        mode: 'edit',
        id: d.id,
        title: d.title,
        category: d.category,
        amount: String(d.amount),
        reason: d.reason || '',
        invoice_ids: d.invoices.map((i) => i.id),
      }
      fetchInvoicesForSelect()
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.fetchFailed')))
    }
  }

  async function handleSaveForm() {
    const f = formModal
    if (!f.title.trim()) {
      message.error(t('finance.errTitle'))
      return
    }
    if (!f.category) {
      message.error(t('finance.errCategory'))
      return
    }
    const amount = Number(f.amount)
    if (!Number.isFinite(amount) || amount <= 0) {
      message.error(t('finance.errAmount'))
      return
    }
    if (amount > 99999999.99) {
      message.error(t('finance.amountTooLarge'))
      return
    }
    saving = true
    try {
      const payload = {
        title: f.title.trim(),
        category: f.category,
        amount,
        reason: f.reason || undefined,
        invoice_ids: f.invoice_ids,
      }
      const res =
        f.mode === 'create'
          ? await createReimbursement(payload)
          : await updateReimbursement(f.id, payload)
      if (res.code !== 0) {
        message.error(res.message || t('finance.operationFailed'))
        return
      }
      message.success(
        f.mode === 'create' ? t('finance.createdSuccess') : t('finance.updatedSuccess'),
      )
      formModal.open = false
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.operationFailed')))
    } finally {
      saving = false
    }
  }

  // ---- 详情弹窗（含审批流水）----
  let detailModal = $state({ open: false, id: '' })
  let detail = $state<ReimbursementDetail | null>(null)
  let detailLoading = $state(false)

  async function openDetail(row: Reimbursement) {
    detailModal = { open: true, id: row.id }
    detail = null
    detailLoading = true
    try {
      const res = await getReimbursement(row.id)
      if (res.code !== 0 || !res.data) {
        message.error(res.message || t('finance.fetchFailed'))
        detailModal.open = false
        return
      }
      detail = res.data
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.fetchFailed')))
      detailModal.open = false
    } finally {
      detailLoading = false
    }
  }

  // ---- 审批 / 复核（共用驳回意见弹窗）----
  let reviewModal = $state({
    open: false,
    kind: '' as 'approve' | 'review',
    action: 'approve' as 'approve' | 'reject',
    id: '',
    comment: '',
  })
  let reviewing = $state(false)

  function askApprove(row: Reimbursement) {
    reviewModal = { open: true, kind: 'approve', action: 'approve', id: row.id, comment: '' }
  }
  function askReview(row: Reimbursement) {
    reviewModal = { open: true, kind: 'review', action: 'approve', id: row.id, comment: '' }
  }
  function askReject(row: Reimbursement) {
    reviewModal = {
      open: true,
      kind: row.status === 'pending_leader' ? 'approve' : 'review',
      action: 'reject',
      id: row.id,
      comment: '',
    }
  }

  async function handleReview() {
    const rm = reviewModal
    if (rm.action === 'reject' && !rm.comment.trim()) {
      message.error(rm.kind === 'approve' ? t('finance.rejectPlaceholder') : t('finance.rejectPlaceholder'))
      return
    }
    reviewing = true
    try {
      const res =
        rm.kind === 'approve'
          ? await approveReimbursement(rm.id, rm.action, rm.comment || undefined)
          : await reviewReimbursement(rm.id, rm.action, rm.comment || undefined)
      if (res.code !== 0) {
        message.error(res.message || t('finance.operationFailed'))
        return
      }
      message.success(
        rm.action === 'approve'
          ? rm.kind === 'approve'
            ? t('finance.approvedSuccess')
            : t('finance.reviewedSuccess')
          : t('finance.rejectedSuccess'),
      )
      reviewModal.open = false
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.operationFailed')))
    } finally {
      reviewing = false
    }
  }

  // ---- 付款 / 撤回 / 删除 ----
  async function handlePay(row: Reimbursement) {
    const ok = await modal.confirm({ title: t('finance.pay'), content: t('finance.payConfirm') })
    if (!ok) return
    try {
      const res = await payReimbursement(row.id)
      if (res.code !== 0) {
        message.error(res.message || t('finance.operationFailed'))
        return
      }
      message.success(t('finance.paidSuccess'))
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.operationFailed')))
    }
  }

  async function handleWithdraw(row: Reimbursement) {
    const ok = await modal.confirm({ title: t('finance.withdraw'), content: t('finance.withdrawConfirm') })
    if (!ok) return
    try {
      const res = await withdrawReimbursement(row.id)
      if (res.code !== 0) {
        message.error(res.message || t('finance.operationFailed'))
        return
      }
      message.success(t('finance.withdrawnSuccess'))
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.operationFailed')))
    }
  }

  async function handleDelete(row: Reimbursement) {
    const ok = await modal.confirm({ title: t('finance.delete'), content: t('finance.deleteConfirm') })
    if (!ok) return
    try {
      const res = await deleteReimbursement(row.id)
      if (res.code !== 0) {
        message.error(res.message || t('finance.operationFailed'))
        return
      }
      message.success(t('finance.deletedSuccess'))
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.operationFailed')))
    }
  }

  function statusLabel(status: string): string {
    return (
      {
        pending_leader: t('finance.statusPendingLeader'),
        pending_finance: t('finance.statusPendingFinance'),
        approved: t('finance.statusApproved'),
        rejected: t('finance.statusRejected'),
        withdrawn: t('finance.statusWithdrawn'),
        paid: t('finance.statusPaid'),
      }[status] || status
    )
  }

  function statusColor(status: string): string {
    return (
      {
        pending_leader: 'orange',
        pending_finance: 'blue',
        approved: 'cyan',
        rejected: 'red',
        withdrawn: 'default',
        paid: 'green',
      }[status] || 'default'
    )
  }

  function categoryLabel(category: string): string {
    return (
      {
        travel: t('finance.categoryTravel'),
        office: t('finance.categoryOffice'),
        meal: t('finance.categoryMeal'),
        transport: t('finance.categoryTransport'),
        other: t('finance.categoryOther'),
      }[category] || category
    )
  }

  function logActionLabel(action: string): string {
    return (
      {
        submit: t('finance.logActionSubmit'),
        approve: t('finance.logActionApprove'),
        reject: t('finance.logActionReject'),
        review: t('finance.logActionReview'),
        pay: t('finance.logActionPay'),
        withdraw: t('finance.logActionWithdraw'),
        edit: t('finance.logActionEdit'),
        resubmit: t('finance.logActionResubmit'),
      }[action] || action
    )
  }

  // 当前用户可操作的行
  let statusOptions = $derived(
    STATUS_ORDER.map((s) => ({ value: s, label: statusLabel(s) })),
  )

  const columns: TableColumn<Reimbursement>[] = $derived([
    {
      title: t('finance.titleField'),
      key: 'title',
      width: 180,
      render: (r) => r.title,
    },
    {
      title: t('finance.employee'),
      key: 'employee_name',
      width: 90,
      render: (r) => r.employee_name,
    },
    {
      title: t('finance.department'),
      key: 'department_name',
      width: 110,
      render: (r) => r.department_name,
    },
    {
      title: t('finance.category'),
      key: 'category',
      width: 90,
      render: (r) => categoryLabel(r.category),
    },
    {
      title: t('finance.amount'),
      key: 'amount',
      width: 110,
      align: 'right',
      render: (r) => `¥${r.amount.toFixed(2)}`,
    },
    {
      title: t('finance.status'),
      key: 'status',
      width: 100,
      align: 'center',
      snippet: 'status',
    },
    {
      title: t('finance.createdAt'),
      key: 'created_at',
      width: 150,
      render: (r) => formatTimestamp(r.created_at, get(preferencesStore)),
    },
    { title: t('finance.actions'), key: 'action', width: 340, snippet: 'action' },
  ])

  function isMine(row: Reimbursement): boolean {
    return row.employee_id === $authStore.user?.id
  }

  onMount(async () => {
    if (!canView) return
    fetchData()
    getDepartments()
      .then((res) => {
        if (res.code === 0) deptOptions = res.data.items.map((d) => ({ value: d.id, label: d.name }))
      })
      .catch(() => {})
  })
</script>

{#if !canView}
  <Result status="403" title="403" subTitle={t('common.noAccess')}>
    {#snippet extra()}
      <Button type="primary" tooltip={t('common.backHome')} onClick={() => goto('/')}>{t('common.backHome')}</Button>
    {/snippet}
  </Result>
{:else}
  {#snippet status(row: Reimbursement)}
    <Tag color={statusColor(row.status)}>{statusLabel(row.status)}</Tag>
  {/snippet}

  {#snippet action(row: Reimbursement)}
    <Space size="small" wrap={true}>
      <Button type="link" size="small" tooltip={t('finance.detail')} onClick={() => openDetail(row)}>
        <Icon name="eye" style="font-size:14px" />{t('common.view')}
      </Button>
      {#if row.status === 'pending_leader' && isMine(row)}
        <Button type="link" size="small" tooltip={t('finance.edit')} onClick={() => openEdit(row)}>
          <Icon name="edit" style="font-size:14px" />{t('common.edit')}
        </Button>
      {/if}
      {#if (row.status === 'rejected' || row.status === 'withdrawn') && isMine(row)}
        <Button type="link" size="small" tooltip={t('finance.resubmit')} onClick={() => openEdit(row)}>
          <Icon name="reload" style="font-size:14px" />{t('finance.resubmit')}
        </Button>
      {/if}
      {#if row.status === 'pending_leader' && canApprove && !isMine(row)}
        <Button type="link" size="small" tooltip={t('finance.approveTooltip')} onClick={() => askApprove(row)}>
          <Icon name="check" style="font-size:14px" />{t('finance.approve')}
        </Button>
        <Button type="link" size="small" danger={true} tooltip={t('finance.reject')} onClick={() => askReject(row)}>
          <Icon name="close" style="font-size:14px" />{t('finance.reject')}
        </Button>
      {/if}
      {#if row.status === 'pending_finance' && canManage}
        <Button type="link" size="small" tooltip={t('finance.reviewTooltip')} onClick={() => askReview(row)}>
          <Icon name="check" style="font-size:14px" />{t('finance.review')}
        </Button>
        <Button type="link" size="small" danger={true} tooltip={t('finance.reject')} onClick={() => askReject(row)}>
          <Icon name="close" style="font-size:14px" />{t('finance.reject')}
        </Button>
      {/if}
      {#if row.status === 'approved' && canManage}
        <Button type="link" size="small" tooltip={t('finance.payTooltip')} onClick={() => handlePay(row)}>
          <Icon name="wallet" style="font-size:14px" />{t('finance.pay')}
        </Button>
      {/if}
      {#if (row.status === 'pending_leader' || row.status === 'pending_finance') && isMine(row)}
        <Button type="link" size="small" tooltip={t('finance.withdraw')} onClick={() => handleWithdraw(row)}>
          <Icon name="swap" style="font-size:14px" />{t('finance.withdraw')}
        </Button>
      {/if}
      {#if canManage || (isMine(row) && (row.status === 'rejected' || row.status === 'withdrawn'))}
        <Popconfirm title={t('finance.deleteConfirm')} onConfirm={() => handleDelete(row)}>
          <Button type="link" size="small" danger={true} tooltip={t('finance.delete')}>
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
          placeholder={t('finance.keywordPlaceholder')}
          prefix="search"
          value={keyword}
          onInput={(v) => (keyword = v)}
          onEnter={handleSearch}
          style="width:240px;flex-shrink:0"
        />
        <Select
          value={statusFilter || undefined}
          options={[{ value: '', label: t('finance.allStatus') }, ...statusOptions]}
          allowClear={true}
          placeholder={t('finance.statusFilter')}
          width="150px"
          onChange={(v) => (statusFilter = (v as '' | ReimbursementStatus) || '')}
        />
        <Select
          value={deptFilter || undefined}
          options={deptOptions}
          allowClear={true}
          placeholder={t('finance.department')}
          width="160px"
          onChange={(v) => (deptFilter = String(v || ''))}
        />
        <Space size="small">
          <Button type="primary" tooltip={t('common.search')} onClick={handleSearch}>{t('common.search')}</Button>
          <Button tooltip={t('common.reset')} onClick={handleReset}>{t('common.reset')}</Button>
        </Space>
        <div style="flex:1"></div>
        {#if canSubmit}
          <Button type="primary" tooltip={t('finance.createTooltip')} onClick={openCreate}>
            <Icon name="plus" style="font-size:14px" />{t('finance.create')}
          </Button>
        {/if}
        {#if canManage}
          <a
            href={exportReportUrl('reimbursements', {})}
            style="text-decoration:none"
          >
            <Button tooltip={t('finance.exportReimbursements')}>
              <Icon name="upload" style="font-size:14px" />{t('common.export')}
            </Button>
          </a>
        {/if}
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
      snippets={{ status, action }}
    />
  </div>

  <!-- 提交/编辑弹窗 -->
  <Modal
    open={formModal.open}
    title={formModal.mode === 'create' ? t('finance.create') : t('finance.edit')}
    onclose={() => (formModal.open = false)}
    onOk={handleSaveForm}
    confirmLoading={saving}
    okText={formModal.mode === 'create' ? t('finance.create') : t('common.save')}
    cancelText={t('common.cancel')}
    width={620}
  >
    <div style="display:flex;flex-direction:column;gap:4px">
      <FormItem label={t('finance.titleField')}>
        <Input
          placeholder={t('finance.titlePlaceholder')}
          value={formModal.title}
          onInput={(v) => (formModal = { ...formModal, title: v })}
        />
      </FormItem>
      <FormItem label={t('finance.category')}>
        <Select
          value={formModal.category}
          options={CATEGORIES.map((c) => ({ value: c, label: categoryLabel(c) }))}
          onChange={(v) => (formModal = { ...formModal, category: String(v || 'other') })}
        />
      </FormItem>
      <FormItem label={t('finance.amount')}>
        <Input
          placeholder={t('finance.amountPlaceholder')}
          value={formModal.amount}
          onInput={(v) => (formModal = { ...formModal, amount: v })}
        />
      </FormItem>
      <FormItem label={t('finance.reason')}>
        <Input
          placeholder={t('finance.reasonPlaceholder')}
          value={formModal.reason}
          onInput={(v) => (formModal = { ...formModal, reason: v })}
        />
      </FormItem>
      <FormItem label={t('finance.invoices')}>
        <Select
          value={formModal.invoice_ids}
          options={invoiceOptions}
          multiple={true}
          allowClear={true}
          placeholder={t('finance.invoicesPlaceholder')}
          onChange={(v) => (formModal = { ...formModal, invoice_ids: (v as string[]) || [] })}
        />
      </FormItem>
    </div>
  </Modal>

  <!-- 审批/复核弹窗 -->
  <Modal
    open={reviewModal.open}
    title={reviewModal.action === 'approve' ? t('finance.approve') : t('finance.reject')}
    onclose={() => (reviewModal.open = false)}
    onOk={handleReview}
    confirmLoading={reviewing}
    okText={reviewModal.action === 'approve' ? t('common.confirm') : t('finance.reject')}
    okDanger={reviewModal.action === 'reject'}
    cancelText={t('common.cancel')}
    width={480}
  >
    {#if reviewModal.action === 'reject'}
      <FormItem label={t('finance.reject')}>
        <Input
          placeholder={t('finance.rejectPlaceholder')}
          value={reviewModal.comment}
          onInput={(v) => (reviewModal = { ...reviewModal, comment: v })}
        />
      </FormItem>
    {:else}
      <span style="color:var(--ant-color-text-secondary)">
        {reviewModal.kind === 'approve'
          ? t('finance.approveTooltip')
          : t('finance.reviewTooltip')}
      </span>
    {/if}
  </Modal>

  <!-- 详情弹窗 -->
  <Modal
    open={detailModal.open}
    title={t('finance.detail')}
    onclose={() => (detailModal.open = false)}
    width={720}
    bodyStyle="padding:16px 24px;max-height:70vh;overflow:auto"
  >
    {#if detailLoading}
      <div style="text-align:center;padding:32px">Loading...</div>
    {:else if detail}
      <Descriptions
        column={2}
        items={[
          { label: t('finance.titleField'), value: detail.title },
          { label: t('finance.employee'), value: detail.employee_name },
          { label: t('finance.department'), value: detail.department_name },
          { label: t('finance.category'), value: categoryLabel(detail.category) },
          { label: t('finance.amount'), value: `¥${detail.amount.toFixed(2)} ${detail.currency}` },
          { label: t('finance.status'), value: statusLabel(detail.status) },
          { label: t('finance.reason'), value: detail.reason || '-' },
          { label: t('finance.createdAt'), value: formatTimestamp(detail.created_at, get(preferencesStore)) },
        ]}
      />
      <div style="margin-top:16px">
        <div style="font-weight:600;margin-bottom:8px">{t('finance.invoices')}</div>
        {#if detail.invoices.length === 0}
          <span style="color:var(--ant-color-text-secondary)">{t('finance.noInvoices')}</span>
        {:else}
          <div style="display:flex;flex-direction:column;gap:6px">
            {#each detail.invoices as inv}
              <div style="display:flex;justify-content:space-between;align-items:center;padding:8px 12px;border:1px solid var(--ant-color-border-secondary);border-radius:6px">
                <span>{inv.invoice_code} · {inv.issuer_name}</span>
                <span>¥{inv.amount.toFixed(2)}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>
      <div style="margin-top:16px">
        <div style="font-weight:600;margin-bottom:8px">{t('finance.logs')}</div>
        <div style="display:flex;flex-direction:column;gap:8px">
          {#each detail.logs as log}
            <div style="display:flex;justify-content:space-between;gap:12px;padding:8px 12px;border:1px solid var(--ant-color-border-secondary);border-radius:6px">
              <div>
                <span style="font-weight:500">{logActionLabel(log.action)}</span>
                {#if log.comment}<span style="color:var(--ant-color-text-secondary);margin-left:8px">{log.comment}</span>{/if}
              </div>
              <div style="color:var(--ant-color-text-secondary);white-space:nowrap">
                {log.actor_name} · {formatTimestamp(log.created_at, get(preferencesStore))}
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </Modal>
{/if}
