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
  // 财务报表：期间收支汇总 + 部门费用排行 + 收支趋势（按月/年）+ CSV 导出
  // 权限：finance:report_view
  import { onMount } from 'svelte'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { t } from '$lib/i18n'
  import { getApiError } from '$lib/api/client'
  import {
    getReportSummary,
    getReportDepartments,
    getReportTrend,
    exportReportUrl,
  } from '$lib/api/finance'
  import type { ReportSummary, DepartmentReportRow, TrendRow } from '$lib/types'
  import Button from '$lib/components/Button.svelte'
  import Input from '$lib/components/Input.svelte'
  import Space from '$lib/components/Space.svelte'
  import Card from '$lib/components/Card.svelte'
  import Result from '$lib/components/Result.svelte'
  import Statistic from '$lib/components/Statistic.svelte'
  import Row from '$lib/components/Row.svelte'
  import Col from '$lib/components/Col.svelte'
  import Table from '$lib/components/Table.svelte'
  import type { TableColumn } from '$lib/components/Table.svelte'
  import Empty from '$lib/components/Empty.svelte'
  import { Icon } from '$lib/icons'
  import { message } from '$lib/components/message'

  let canView = $derived($authStore.permissions.includes('finance:report_view'))

  // 默认统计当前自然年
  const now = new Date()
  let from = $state(`${now.getFullYear()}-01-01`)
  let to = $state(`${now.getFullYear()}-12-31`)
  let granularity = $state('month')

  let summary = $state<ReportSummary | null>(null)
  let deptRows = $state<DepartmentReportRow[]>([])
  let trendRows = $state<TrendRow[]>([])
  let loading = $state(false)

  async function fetchAll() {
    loading = true
    try {
      const params = {
        from: from || undefined,
        to: to || undefined,
        granularity,
      }
      const [sRes, dRes, tRes] = await Promise.all([
        getReportSummary(params),
        getReportDepartments(params),
        getReportTrend(params),
      ])
      if (sRes.code !== 0 || dRes.code !== 0 || tRes.code !== 0) {
        message.error(sRes.message || dRes.message || tRes.message || t('finance.reportFetchFailed'))
        return
      }
      summary = sRes.data
      deptRows = dRes.data.items
      trendRows = tRes.data.items
    } catch (err: unknown) {
      message.error(getApiError(err, t('finance.reportFetchFailed')))
    } finally {
      loading = false
    }
  }

  const deptColumns: TableColumn<DepartmentReportRow>[] = $derived([
    { title: t('finance.budgetDepartment'), key: 'department_name', width: 220, render: (r) => r.department_name },
    {
      title: t('finance.budgetSpent'),
      key: 'expense',
      width: 160,
      align: 'right',
      render: (r) => `¥${r.expense.toFixed(2)}`,
    },
    {
      title: t('finance.reportTrend'),
      key: 'ratio',
      width: 240,
      snippet: 'ratio',
    },
  ])

  let maxDeptExpense = $derived(deptRows.length > 0 ? Math.max(...deptRows.map((r) => r.expense)) : 0)

  // 趋势图（纯 CSS 柱状图）
  let maxTrend = $derived(
    trendRows.length > 0
      ? Math.max(...trendRows.flatMap((r) => [r.income, r.expense]), 1)
      : 1,
  )

  function fmtMoney(v: number): string {
    return `¥${v.toFixed(2)}`
  }

  onMount(() => {
    if (!canView) return
    fetchAll()
  })
</script>

{#if !canView}
  <Result status="403" title="403" subTitle={t('common.noAccess')}>
    {#snippet extra()}
      <Button type="primary" tooltip={t('common.backHome')} onClick={() => goto('/')}>{t('common.backHome')}</Button>
    {/snippet}
  </Result>
{:else}
  {#snippet ratio(row: DepartmentReportRow)}
    <div style="display:flex;align-items:center;gap:8px">
      <div style="flex:1;height:10px;background:var(--ant-color-fill-tertiary);border-radius:5px;overflow:hidden">
        <div
          style="height:100%;width:{maxDeptExpense > 0 ? (row.expense / maxDeptExpense) * 100 : 0}%;background:var(--ant-color-primary);border-radius:5px"
        ></div>
      </div>
      <span style="font-size:12px;color:var(--ant-color-text-secondary);white-space:nowrap">
        {maxDeptExpense > 0 ? ((row.expense / maxDeptExpense) * 100).toFixed(0) : 0}%
      </span>
    </div>
  {/snippet}

  <div class="page-scroll" style="height:100%;overflow:auto">
    <Card bodyStyle="padding:16px 24px" style="margin-bottom:16px">
      <div style="display:flex;align-items:center;gap:12px;flex-wrap:wrap">
        <span style="color:var(--ant-color-text-secondary)">{t('finance.reportFrom')}</span>
        <Input value={from} onInput={(v) => (from = v)} style="width:140px;flex-shrink:0" />
        <span style="color:var(--ant-color-text-secondary)">{t('finance.reportTo')}</span>
        <Input value={to} onInput={(v) => (to = v)} style="width:140px;flex-shrink:0" />
        <Space size="small">
          <Button
            type={granularity === 'month' ? 'primary' : 'default'}
            onClick={() => {
              granularity = 'month'
              fetchAll()
            }}
          >
            {t('finance.reportTrendMonth')}
          </Button>
          <Button
            type={granularity === 'year' ? 'primary' : 'default'}
            onClick={() => {
              granularity = 'year'
              fetchAll()
            }}
          >
            {t('finance.reportTrendYear')}
          </Button>
        </Space>
        <div style="flex:1"></div>
        <Button type="primary" tooltip={t('common.search')} onClick={fetchAll} loading={loading}>
          <Icon name="search" style="font-size:14px" />{t('common.search')}
        </Button>
        <a href={exportReportUrl('reimbursements', { from: from || undefined, to: to || undefined })} style="text-decoration:none">
          <Button tooltip={t('finance.reportExportReimburse')}>
            <Icon name="upload" style="font-size:14px" />{t('finance.reportExportReimburse')}
          </Button>
        </a>
        <a href={exportReportUrl('payments', { from: from || undefined, to: to || undefined })} style="text-decoration:none">
          <Button tooltip={t('finance.reportExportPayments')}>
            <Icon name="upload" style="font-size:14px" />{t('finance.reportExportPayments')}
          </Button>
        </a>
      </div>
    </Card>

    {#if loading && !summary}
      <div style="text-align:center;padding:48px">Loading...</div>
    {:else if summary}
      <Card bodyStyle="padding:24px" style="margin-bottom:16px">
        <Row gutter={[16, 16]}>
          <Col span={8}>
            <Statistic title={t('finance.reportIncome')} value={fmtMoney(summary.income)} prefix="+" style="text-align:left" />
            <div style="color:var(--ant-color-text-secondary);font-size:12px;margin-top:4px">
              {t('finance.reportIncomeCount')}: {summary.income_count}
            </div>
          </Col>
          <Col span={8}>
            <Statistic title={t('finance.reportExpense')} value={fmtMoney(summary.expense)} prefix="-" style="text-align:left" />
            <div style="color:var(--ant-color-text-secondary);font-size:12px;margin-top:4px">
              {t('finance.reportExpenseCount')}: {summary.expense_count}
            </div>
          </Col>
          <Col span={8}>
            <Statistic
              title={t('finance.reportNet')}
              value={fmtMoney(summary.net)}
              style={`text-align:left;color:${summary.net < 0 ? 'var(--ant-color-error)' : 'var(--ant-color-success)'}`}
            />
            <div style="color:var(--ant-color-text-secondary);font-size:12px;margin-top:4px">
              {t('finance.reportPendingCount')}: {summary.reimbursement_pending_count} · {t('finance.reportPendingReimburse')}: {fmtMoney(summary.reimbursement_pending)}
            </div>
          </Col>
        </Row>
      </Card>

      <Row gutter={[16, 16]}>
        <Col span={12}>
          <Card title={t('finance.reportDeptRank')} bodyStyle="padding:16px 24px">
            {#if deptRows.length === 0}
              <Empty description={t('finance.reportNoData')} />
            {:else}
              <Table
                columns={deptColumns}
                dataSource={deptRows as never[]}
                rowKey="department_id"
                size="small"
                snippets={{ ratio }}
              />
            {/if}
          </Card>
        </Col>
        <Col span={12}>
          <Card title={t('finance.reportTrend')} bodyStyle="padding:16px 24px">
            {#if trendRows.length === 0}
              <Empty description={t('finance.reportNoData')} />
            {:else}
              <div style="display:flex;flex-direction:column;gap:10px">
                {#each trendRows as row (row.period)}
                  <div style="display:flex;align-items:center;gap:10px">
                    <span style="width:64px;font-size:12px;color:var(--ant-color-text-secondary);white-space:nowrap;flex-shrink:0">{row.period}</span>
                    <div style="flex:1;display:flex;flex-direction:column;gap:3px">
                      <div style="display:flex;align-items:center;gap:6px">
                        <span style="width:32px;font-size:11px;color:var(--ant-color-success);flex-shrink:0">+{fmtMoney(row.income)}</span>
                        <div style="flex:1;height:8px;background:var(--ant-color-fill-tertiary);border-radius:4px;overflow:hidden">
                          <div style="height:100%;width:{row.income / maxTrend * 100}%;background:var(--ant-color-success);border-radius:4px"></div>
                        </div>
                      </div>
                      <div style="display:flex;align-items:center;gap:6px">
                        <span style="width:32px;font-size:11px;color:var(--ant-color-error);flex-shrink:0">-{fmtMoney(row.expense)}</span>
                        <div style="flex:1;height:8px;background:var(--ant-color-fill-tertiary);border-radius:4px;overflow:hidden">
                          <div style="height:100%;width:{row.expense / maxTrend * 100}%;background:var(--ant-color-error);border-radius:4px"></div>
                        </div>
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </Card>
        </Col>
      </Row>
    {/if}
  </div>
{/if}
