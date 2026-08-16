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
  // 仪表盘：复刻原 React src/pages/Dashboard.tsx
  // - 欢迎信息
  // - 任务卡片（task:create/view_all 权限；本人待办/已完成/逾期，管理员为全员统计）
  // - 财务卡片（finance:report_view 权限；本月收入/支出/待付报销）
  // - 系统运行状态（system:settings 权限，健康检查 /system/health）
  // - 个人信息（Descriptions 视觉）
  import { onMount } from 'svelte'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { client, getApiError } from '$lib/api/client'
  import { getTaskStats } from '$lib/api/tasks'
  import { getReportSummary } from '$lib/api/finance'
  import { t } from '$lib/i18n'
  import Card from '$lib/components/Card.svelte'
  import Row from '$lib/components/Row.svelte'
  import Col from '$lib/components/Col.svelte'
  import Title from '$lib/components/Title.svelte'
  import Text from '$lib/components/Text.svelte'
  import Spin from '$lib/components/Spin.svelte'
  import Tag from '$lib/components/Tag.svelte'
  import Button from '$lib/components/Button.svelte'
  import { message } from '$lib/components/message'

  interface SystemHealth {
    server: string
    database: string
    version: string
  }

  interface TaskStatsData {
    total: number
    todo: number
    done: number
    overdue: number
    can_view_all: boolean
  }

  interface FinanceSummaryData {
    income: number
    expense: number
    net: number
    income_count: number
    expense_count: number
    reimbursement_pending: number
    reimbursement_pending_count: number
  }

  let health = $state<SystemHealth | null>(null)
  let healthLoading = $state(false)
  let canSettings = $derived($authStore.permissions.includes('system:settings'))
  let canTasks = $derived(
    $authStore.permissions.includes('task:create') ||
      $authStore.permissions.includes('task:view_all'),
  )
  let canFinance = $derived($authStore.permissions.includes('finance:report_view'))

  let taskStats = $state<TaskStatsData | null>(null)
  let financeSummary = $state<FinanceSummaryData | null>(null)

  // 本月日期范围
  const now = new Date()
  const monthFrom = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-01`
  const monthTo = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${new Date(now.getFullYear(), now.getMonth() + 1, 0).getDate()}`

  // 官方 antd outlined 图标 path（与 icon-data.ts 同源 @ant-design/icons-svg@4.5.0，
  // icon-data 未收录 cloud-server/check-circle/close-circle，故页面内嵌）
  const CLOUD_SERVER_PATHS = [
    'M704 446H320c-4.4 0-8 3.6-8 8v402c0 4.4 3.6 8 8 8h384c4.4 0 8-3.6 8-8V454c0-4.4-3.6-8-8-8zm-328 64h272v117H376V510zm272 290H376V683h272v117z',
    'M424 748a32 32 0 1064 0 32 32 0 10-64 0zm0-178a32 32 0 1064 0 32 32 0 10-64 0z',
    'M811.4 368.9C765.6 248 648.9 162 512.2 162S258.8 247.9 213 368.8C126.9 391.5 63.5 470.2 64 563.6 64.6 668 145.6 752.9 247.6 762c4.7.4 8.7-3.3 8.7-8v-60.4c0-4-3-7.4-7-7.9-27-3.4-52.5-15.2-72.1-34.5-24-23.5-37.2-55.1-37.2-88.6 0-28 9.1-54.4 26.2-76.4 16.7-21.4 40.2-36.9 66.1-43.7l37.9-10 13.9-36.7c8.6-22.8 20.6-44.2 35.7-63.5 14.9-19.2 32.6-36 52.4-50 41.1-28.9 89.5-44.2 140-44.2s98.9 15.3 140 44.3c19.9 14 37.5 30.8 52.4 50 15.1 19.3 27.1 40.7 35.7 63.5l13.8 36.6 37.8 10c54.2 14.4 92.1 63.7 92.1 120 0 33.6-13.2 65.1-37.2 88.6-19.5 19.2-44.9 31.1-71.9 34.5-4 .5-6.9 3.9-6.9 7.9V754c0 4.7 4.1 8.4 8.8 8 101.7-9.2 182.5-94 183.2-198.2.6-93.4-62.7-172.1-148.6-194.9z',
  ]
  const CHECK_CIRCLE_PATHS = [
    'M699 353h-46.9c-10.2 0-19.9 4.9-25.9 13.3L469 584.3l-71.2-98.8c-6-8.3-15.6-13.3-25.9-13.3H325c-6.5 0-10.3 7.4-6.5 12.7l124.6 172.8a31.8 31.8 0 0051.7 0l210.6-292c3.9-5.3.1-12.7-6.4-12.7z',
    'M512 64C264.6 64 64 264.6 64 512s200.6 448 448 448 448-200.6 448-448S759.4 64 512 64zm0 820c-205.4 0-372-166.6-372-372s166.6-372 372-372 372 166.6 372 372-166.6 372-372 372z',
  ]
  const CLOSE_CIRCLE_PATHS = [
    'M512 64c247.4 0 448 200.6 448 448S759.4 960 512 960 64 759.4 64 512 264.6 64 512 64zm0 76c-205.4 0-372 166.6-372 372s166.6 372 372 372 372-166.6 372-372-166.6-372-372-372zm128.01 198.83c.03 0 .05.01.09.06l45.02 45.01a.2.2 0 01.05.09.12.12 0 010 .07c0 .02-.01.04-.05.08L557.25 512l127.87 127.86a.27.27 0 01.05.06v.02a.12.12 0 010 .07c0 .03-.01.05-.05.09l-45.02 45.02a.2.2 0 01-.09.05.12.12 0 01-.07 0c-.02 0-.04-.01-.08-.05L512 557.25 384.14 685.12c-.04.04-.06.05-.08.05a.12.12 0 01-.07 0c-.03 0-.05-.01-.09-.05l-45.02-45.02a.2.2 0 01-.05-.09.12.12 0 010-.07c0-.02.01-.04.06-.08L466.75 512 338.88 384.14a.27.27 0 01-.05-.06l-.01-.02a.12.12 0 010-.07c0-.03.01-.05.05-.09l45.02-45.02a.2.2 0 01.09-.05.12.12 0 01.07 0c.02 0 .04.01.08.06L512 466.75l127.86-127.86c.04-.05.06-.06.08-.06a.12.12 0 01.07 0z',
  ]

  function fmtMoney(v: number): string {
    return `¥${(v || 0).toFixed(2)}`
  }

  onMount(() => {
    if (canTasks) {
      getTaskStats()
        .then((res) => {
          if (res.code === 0 && res.data) taskStats = res.data
        })
        .catch(() => {})
    }
    if (canFinance) {
      getReportSummary({ from: monthFrom, to: monthTo })
        .then((res) => {
          if (res.code === 0 && res.data) financeSummary = res.data
        })
        .catch(() => {})
    }
    if (canSettings) {
      healthLoading = true
      client
        .get<SystemHealth>('/system/health')
        .then((res) => {
          health = res.data
        })
        .catch(() => {
          health = null
        })
        .finally(() => {
          healthLoading = false
        })
    }
  })
</script>

{#snippet statusIcon(paths: string[], color: string)}
  <span class="anticon dash-status-icon" style="color:{color}">
    <svg viewBox="64 64 896 896" width="1em" height="1em" fill="currentColor" focusable="false" aria-hidden="true">
      {#each paths as p}<path d={p} />{/each}
    </svg>
  </span>
{/snippet}

<div>
  <Title level={4} style="margin-bottom:24px">
    {t('dashboard.welcome', { name: $authStore.user?.name || $authStore.user?.username || '' })}
  </Title>

  <Row gutter={16} style="margin-bottom:24px">
    {#if canTasks}
      <Col span={12}>
        <Card title={t('dashboard.taskCard')}>
          {#if taskStats}
            <Row gutter={16}>
              <Col span={8}>
                <div class="dash-stat" style="color:var(--ant-color-warning)">
                  <div class="dash-stat-value">{taskStats.todo}</div>
                  <div class="dash-stat-label">{t('dashboard.taskTodo')}</div>
                </div>
              </Col>
              <Col span={8}>
                <div class="dash-stat" style="color:var(--ant-color-success)">
                  <div class="dash-stat-value">{taskStats.done}</div>
                  <div class="dash-stat-label">{t('dashboard.taskDone')}</div>
                </div>
              </Col>
              <Col span={8}>
                <div class="dash-stat" style="color:{taskStats.overdue > 0 ? 'var(--ant-color-error)' : 'var(--ant-color-text)'}">
                  <div class="dash-stat-value">{taskStats.overdue}</div>
                  <div class="dash-stat-label">{t('dashboard.taskOverdue')}</div>
                </div>
              </Col>
            </Row>
            <div style="text-align:right;margin-top:12px">
              <Button type="link" size="small" onClick={() => goto('/tasks')}>
                {t('dashboard.taskGo')} →
              </Button>
            </div>
          {:else}
            <Spin />
          {/if}
        </Card>
      </Col>
    {/if}
    {#if canFinance}
      <Col span={12}>
        <Card title={t('dashboard.financeCard')}>
          {#if financeSummary}
            <Row gutter={16}>
              <Col span={8}>
                <div class="dash-stat" style="color:var(--ant-color-success)">
                  <div class="dash-stat-value">+{fmtMoney(financeSummary.income)}</div>
                  <div class="dash-stat-label">{t('dashboard.financeIncome')}</div>
                </div>
              </Col>
              <Col span={8}>
                <div class="dash-stat" style="color:var(--ant-color-error)">
                  <div class="dash-stat-value">-{fmtMoney(financeSummary.expense)}</div>
                  <div class="dash-stat-label">{t('dashboard.financeExpense')}</div>
                </div>
              </Col>
              <Col span={8}>
                <div class="dash-stat" style="color:var(--ant-color-warning)">
                  <div class="dash-stat-value">{fmtMoney(financeSummary.reimbursement_pending)}</div>
                  <div class="dash-stat-label">{t('dashboard.financePending')}</div>
                </div>
              </Col>
            </Row>
            <div style="text-align:right;margin-top:12px">
              <Button type="link" size="small" onClick={() => goto('/finance/reports')}>
                {t('dashboard.financeGo')} →
              </Button>
            </div>
          {:else}
            <Spin />
          {/if}
        </Card>
      </Col>
    {/if}
  </Row>

  {#if canSettings}
    <Card title={t('dashboard.systemStatus')} style="margin-bottom:24px">
      {#if healthLoading}
        <Spin />
      {:else if health}
        <Row gutter={16}>
          <Col span={8}>
            <Card bodyStyle="padding:12px">
              <div style="display:flex;align-items:center;gap:8px">
                {@render statusIcon(CLOUD_SERVER_PATHS, '#1677ff')}
                <div>
                  <div style="font-size:12px;color:#999">{t('dashboard.server')}</div>
                  <div style="font-weight:600">{t('dashboard.running', { version: health.version })}</div>
                </div>
              </div>
            </Card>
          </Col>
          <Col span={8}>
            <Card bodyStyle="padding:12px">
              <div style="display:flex;align-items:center;gap:8px">
                {#if health.database === 'connected'}
                  {@render statusIcon(CHECK_CIRCLE_PATHS, '#52c41a')}
                {:else}
                  {@render statusIcon(CLOSE_CIRCLE_PATHS, '#ff4d4f')}
                {/if}
                <div>
                  <div style="font-size:12px;color:#999">{t('dashboard.database')}</div>
                  <div style="font-weight:600">
                    {health.database === 'connected' ? t('dashboard.connected') : t('dashboard.disconnected')}
                  </div>
                </div>
              </div>
            </Card>
          </Col>
        </Row>
      {:else}
        <Text type="warning">{t('dashboard.statusFailed')}</Text>
      {/if}
    </Card>
  {/if}

  <Card title={t('dashboard.personalInfo')}>
    <table style="width:100%;border-collapse:collapse">
      <tbody>
        <tr>
          <td class="dash-desc-label">{t('dashboard.username')}</td>
          <td class="dash-desc-value">{$authStore.user?.username ?? ''}</td>
          <td class="dash-desc-label">{t('dashboard.name')}</td>
          <td class="dash-desc-value">{$authStore.user?.name ?? ''}</td>
        </tr>
        <tr>
          <td class="dash-desc-label">{t('dashboard.email')}</td>
          <td class="dash-desc-value">{$authStore.user?.email ?? ''}</td>
          <td class="dash-desc-label">{t('dashboard.title')}</td>
          <td class="dash-desc-value">{$authStore.user?.title || '-'}</td>
        </tr>
        <tr>
          <td class="dash-desc-label">{t('dashboard.phone')}</td>
          <td class="dash-desc-value">{$authStore.user?.phone || '-'}</td>
        </tr>
      </tbody>
    </table>
  </Card>
</div>

<style>
  .dash-status-icon {
    display: inline-flex;
    align-items: center;
    line-height: 0;
    font-size: 24px;
  }
  .dash-status-icon svg {
    width: 1em;
    height: 1em;
  }
  .dash-desc-label {
    padding: 16px 24px;
    color: var(--ant-color-text-secondary);
    font-size: 14px;
    line-height: 1.5715;
    text-align: right;
    white-space: nowrap;
    vertical-align: top;
  }
  .dash-desc-value {
    padding: 16px 24px;
    color: var(--ant-color-text);
    font-size: 14px;
    line-height: 1.5715;
    vertical-align: top;
  }
  .dash-stat {
    text-align: center;
    padding: 8px 0;
  }
  .dash-stat-value {
    font-size: 24px;
    font-weight: 600;
    line-height: 1.2;
  }
  .dash-stat-label {
    font-size: 13px;
    color: var(--ant-color-text-secondary);
    margin-top: 4px;
  }
</style>
