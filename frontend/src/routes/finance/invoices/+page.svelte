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
  // 发票管理：录入（发票号码唯一查重）/ 编辑 / 删除；已关联报销单的发票不可编辑/删除
  // 权限：finance:invoice_manage
  import { onMount } from 'svelte'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { preferencesStore, formatTimestamp } from '$lib/stores/preferences'
  import { get } from 'svelte/store'
  import { t } from '$lib/i18n'
  import { getApiError } from '$lib/api/client'
  import { getInvoices, createInvoice, updateInvoice, deleteInvoice } from '$lib/api/finance'
  import type { Invoice } from '$lib/types'
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
  const INVOICE_TYPES = ['vat', 'normal', 'electronic']

  let canManage = $derived($authStore.permissions.includes('finance:invoice_manage'))

  let data = $state<Invoice[]>([])
  let loading = $state(false)
  let total = $state(0)
  let keyword = $state('')
  let statusFilter = $state('')
  let params = $state({ page: 1, page_size: PAGE_SIZE, keyword: '', status: '' })

  async function fetchData() {
    loading = true
    try {
      const res = await getInvoices({
        page: params.page,
        page_size: params.page_size,
        keyword: params.keyword || undefined,
        status: params.status || undefined,
      })
      if (res.code !== 0) {
        message.error(res.message || t('finance.invoiceFetchFailed'))
        return
      }
      data = res.data.items
      total = res.data.total
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.invoiceFetchFailed')))
    } finally {
      loading = false
    }
  }

  function handleSearch() {
    params = { page: 1, page_size: PAGE_SIZE, keyword: keyword.trim(), status: statusFilter }
    fetchData()
  }

  function handleReset() {
    keyword = ''
    statusFilter = ''
    handleSearch()
  }

  function handleTableChange(page: number) {
    params = { ...params, page }
    fetchData()
  }

  // ---- 录入 / 编辑弹窗 ----
  let formModal = $state({
    open: false,
    mode: 'create' as 'create' | 'edit',
    id: '',
    invoice_code: '',
    invoice_type: 'normal',
    amount: '',
    tax_amount: '',
    issued_at: '',
    issuer_name: '',
    buyer_name: '',
    image_url: '',
  })
  let saving = $state(false)

  function openCreate() {
    formModal = {
      open: true,
      mode: 'create',
      id: '',
      invoice_code: '',
      invoice_type: 'normal',
      amount: '',
      tax_amount: '',
      issued_at: '',
      issuer_name: '',
      buyer_name: '',
      image_url: '',
    }
  }

  function openEdit(row: Invoice) {
    formModal = {
      open: true,
      mode: 'edit',
      id: row.id,
      invoice_code: row.invoice_code,
      invoice_type: row.invoice_type === '增值税专用发票' ? 'vat' : row.invoice_type === '电子发票' ? 'electronic' : 'normal',
      amount: String(row.amount),
      tax_amount: row.tax_amount !== null ? String(row.tax_amount) : '',
      issued_at: row.issued_at || '',
      issuer_name: row.issuer_name,
      buyer_name: row.buyer_name || '',
      image_url: row.image_url || '',
    }
  }

  function typeLabel(type: string): string {
    return (
      {
        vat: t('finance.invoiceTypeVat'),
        normal: t('finance.invoiceTypeNormal'),
        electronic: t('finance.invoiceTypeElectronic'),
        增值税专用发票: t('finance.invoiceTypeVat'),
        普通发票: t('finance.invoiceTypeNormal'),
        电子发票: t('finance.invoiceTypeElectronic'),
      }[type] || type
    )
  }

  async function handleSaveForm() {
    const f = formModal
    if (!f.invoice_code.trim()) {
      message.error(t('finance.errInvoiceCode'))
      return
    }
    const amount = Number(f.amount)
    if (!Number.isFinite(amount) || amount <= 0) {
      message.error(t('finance.errInvoiceAmount'))
      return
    }
    if (!f.issuer_name.trim()) {
      message.error(t('finance.errInvoiceIssuer'))
      return
    }
    const tax = f.tax_amount === '' ? null : Number(f.tax_amount)
    if (tax !== null && (!Number.isFinite(tax) || tax < 0 || tax > amount)) {
      message.error(t('finance.errInvoiceAmount'))
      return
    }
    saving = true
    try {
      const payload: {
        invoice_code: string
        invoice_type: string
        amount: number
        tax_amount?: number
        issued_at?: string
        issuer_name: string
        buyer_name?: string
        image_url?: string
      } = {
        invoice_code: f.invoice_code.trim(),
        invoice_type: typeLabel(f.invoice_type),
        amount,
        issued_at: f.issued_at || undefined,
        issuer_name: f.issuer_name.trim(),
        buyer_name: f.buyer_name.trim() || undefined,
        image_url: f.image_url.trim() || undefined,
      }
      if (tax !== null) payload.tax_amount = tax
      const res =
        f.mode === 'create' ? await createInvoice(payload) : await updateInvoice(f.id, payload)
      if (res.code !== 0) {
        const msg = res.message || (res.code === 409 ? t('finance.invoiceDup') : t('finance.operationFailed'))
        message.error(msg)
        return
      }
      message.success(
        f.mode === 'create' ? t('finance.invoiceCreated') : t('finance.invoiceUpdated'),
      )
      formModal.open = false
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.operationFailed')))
    } finally {
      saving = false
    }
  }

  async function handleDelete(row: Invoice) {
    try {
      const res = await deleteInvoice(row.id)
      if (res.code !== 0) {
        message.error(res.message || t('finance.operationFailed'))
        return
      }
      message.success(t('finance.invoiceDeleted'))
      fetchData()
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.operationFailed')))
    }
  }

  const columns: TableColumn<Invoice>[] = $derived([
    { title: t('finance.invoiceCode'), key: 'invoice_code', width: 150, render: (r) => r.invoice_code },
    {
      title: t('finance.invoiceType'),
      key: 'invoice_type',
      width: 110,
      render: (r) => typeLabel(r.invoice_type),
    },
    {
      title: t('finance.invoiceAmount'),
      key: 'amount',
      width: 110,
      align: 'right',
      render: (r) => `¥${r.amount.toFixed(2)}`,
    },
    {
      title: t('finance.invoiceTax'),
      key: 'tax_amount',
      width: 100,
      align: 'right',
      render: (r) => (r.tax_amount !== null ? `¥${r.tax_amount.toFixed(2)}` : '-'),
    },
    {
      title: t('finance.invoiceIssuedAt'),
      key: 'issued_at',
      width: 110,
      render: (r) => r.issued_at || '-',
    },
    { title: t('finance.invoiceIssuer'), key: 'issuer_name', width: 160, render: (r) => r.issuer_name },
    { title: t('finance.invoiceBuyer'), key: 'buyer_name', width: 140, render: (r) => r.buyer_name || '-' },
    {
      title: t('finance.invoiceStatus'),
      key: 'status',
      width: 90,
      align: 'center',
      snippet: 'status',
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
  })
</script>

{#if !canManage}
  <Result status="403" title="403" subTitle={t('common.noAccess')}>
    {#snippet extra()}
      <Button type="primary" tooltip={t('common.backHome')} onClick={() => goto('/')}>{t('common.backHome')}</Button>
    {/snippet}
  </Result>
{:else}
  {#snippet status(row: Invoice)}
    <Tag color={row.status === 'unused' ? 'default' : 'blue'}>
      {row.status === 'unused' ? t('finance.invoiceStatusUnused') : t('finance.invoiceStatusClaimed')}
    </Tag>
  {/snippet}

  {#snippet action(row: Invoice)}
    <Space size="small">
      {#if row.status === 'unused'}
        <Button type="link" size="small" tooltip={t('common.edit')} onClick={() => openEdit(row)}>
          <Icon name="edit" style="font-size:14px" />{t('common.edit')}
        </Button>
        <Popconfirm title={t('finance.invoiceDeleteConfirm')} onConfirm={() => handleDelete(row)}>
          <Button type="link" size="small" danger={true} tooltip={t('common.delete')}>
            <Icon name="delete" style="font-size:14px" />{t('common.delete')}
          </Button>
        </Popconfirm>
      {:else}
        <span style="color:var(--ant-color-text-secondary);font-size:12px">{t('finance.claimed')}</span>
      {/if}
    </Space>
  {/snippet}

  <div class="page-scroll" style="height:100%;overflow:auto">
    <Card bodyStyle="padding:16px 24px" style="margin-bottom:16px">
      <div style="display:flex;align-items:center;gap:12px;flex-wrap:wrap">
        <Input
          placeholder={t('finance.invoiceCodePlaceholder')}
          prefix="search"
          value={keyword}
          onInput={(v) => (keyword = v)}
          onEnter={handleSearch}
          style="width:260px;flex-shrink:0"
        />
        <Select
          value={statusFilter || undefined}
          options={[
            { value: '', label: t('finance.allStatus') },
            { value: 'unused', label: t('finance.invoiceStatusUnused') },
            { value: 'claimed', label: t('finance.invoiceStatusClaimed') },
          ]}
          allowClear={true}
          placeholder={t('finance.invoiceStatus')}
          width="140px"
          onChange={(v) => (statusFilter = String(v || ''))}
        />
        <Space size="small">
          <Button type="primary" tooltip={t('common.search')} onClick={handleSearch}>{t('common.search')}</Button>
          <Button tooltip={t('common.reset')} onClick={handleReset}>{t('common.reset')}</Button>
        </Space>
        <div style="flex:1"></div>
        <Button type="primary" tooltip={t('finance.invoiceCreate')} onClick={openCreate}>
          <Icon name="plus" style="font-size:14px" />{t('finance.invoiceCreate')}
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
      snippets={{ status, action }}
    />
  </div>

  <!-- 录入/编辑弹窗 -->
  <Modal
    open={formModal.open}
    title={formModal.mode === 'create' ? t('finance.invoiceCreate') : t('finance.edit')}
    onclose={() => (formModal.open = false)}
    onOk={handleSaveForm}
    confirmLoading={saving}
    okText={formModal.mode === 'create' ? t('finance.invoiceCreate') : t('common.save')}
    cancelText={t('common.cancel')}
    width={620}
  >
    <div style="display:flex;flex-direction:column;gap:4px">
      <FormItem label={t('finance.invoiceCode')}>
        <Input
          placeholder={t('finance.invoiceCodePlaceholder')}
          value={formModal.invoice_code}
          onInput={(v) => (formModal = { ...formModal, invoice_code: v })}
        />
      </FormItem>
      <FormItem label={t('finance.invoiceType')}>
        <Select
          value={formModal.invoice_type}
          options={INVOICE_TYPES.map((tp) => ({ value: tp, label: typeLabel(tp) }))}
          onChange={(v) => (formModal = { ...formModal, invoice_type: String(v || 'normal') })}
        />
      </FormItem>
      <FormItem label={t('finance.invoiceAmount')}>
        <Input
          placeholder={t('finance.invoiceAmount')}
          value={formModal.amount}
          onInput={(v) => (formModal = { ...formModal, amount: v })}
        />
      </FormItem>
      <FormItem label={t('finance.invoiceTax')}>
        <Input
          placeholder={t('finance.invoiceTax')}
          value={formModal.tax_amount}
          onInput={(v) => (formModal = { ...formModal, tax_amount: v })}
        />
      </FormItem>
      <FormItem label={t('finance.invoiceIssuedAt')}>
        <Input
          placeholder="YYYY-MM-DD"
          value={formModal.issued_at}
          onInput={(v) => (formModal = { ...formModal, issued_at: v })}
        />
      </FormItem>
      <FormItem label={t('finance.invoiceIssuer')}>
        <Input
          placeholder={t('finance.invoiceIssuerPlaceholder')}
          value={formModal.issuer_name}
          onInput={(v) => (formModal = { ...formModal, issuer_name: v })}
        />
      </FormItem>
      <FormItem label={t('finance.invoiceBuyer')}>
        <Input
          placeholder={t('finance.invoiceBuyerPlaceholder')}
          value={formModal.buyer_name}
          onInput={(v) => (formModal = { ...formModal, buyer_name: v })}
        />
      </FormItem>
    </div>
  </Modal>
{/if}
