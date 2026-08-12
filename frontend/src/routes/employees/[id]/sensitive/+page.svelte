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
  // 员工敏感信息查看页（employee:view_sensitive）
  // 流程：列表操作入口（第一次确认）→ 本页（仍显示掩码 ***）→ 点击「查看完整信息」
  // （第二次确认）→ 后端解密 API（身份验证 + 强制写日志）→ 明文展示。
  import { onMount } from 'svelte'
  import { page } from '$app/stores'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { getApiError } from '$lib/api/client'
  import { t } from '$lib/i18n'
  import { getEmployee, viewSensitiveEmployee, viewSensitiveField } from '$lib/api/employees'
  import type { Employee, SensitiveEmployeeInfo } from '$lib/types'
  import Card from '$lib/components/Card.svelte'
  import Button from '$lib/components/Button.svelte'
  import Spin from '$lib/components/Spin.svelte'
  import Result from '$lib/components/Result.svelte'
  import Modal from '$lib/components/Modal.svelte'
  import { Icon } from '$lib/icons'
  import { modal } from '$lib/components/modal'
  import { message } from '$lib/components/message'

  const id = $page.params.id

  /** 敏感字段配置：key 与后端解密接口 /:field 一致 */
  const SENSITIVE_FIELDS = $derived.by(() => [
    { key: 'email', label: t('sensitive.email') },
    { key: 'phone', label: t('sensitive.phone') },
    { key: 'id_number', label: t('sensitive.idNumber') },
    { key: 'address', label: t('sensitive.address') },
  ] as const)

  function ensureId(): string {
    if (!id) {
      message.error(t('employee.form.missingId'))
      goto('/employees')
      throw new Error('missing employee id')
    }
    return id
  }

  let emp = $state<Employee | null>(null)
  let loading = $state(true)
  let revealing = $state(false)
  let sensitive = $state<SensitiveEmployeeInfo | null>(null)
  let fieldLoading = $state<string | null>(null)
  let fieldModal = $state<{ open: boolean; label: string; value: string }>({
    open: false,
    label: '',
    value: '',
  })

  onMount(() => {
    if (!$authStore.permissions.includes('employee:view_sensitive')) {
      loading = false
      return
    }
    const fetchData = async () => {
      loading = true
      try {
        const res = await getEmployee(ensureId())
        if (res.code !== 0) {
          message.error(res.message || t('employee.form.fetchFailed'))
          goto('/employees')
          return
        }
        emp = res.data
      } catch (err: unknown) {
        message.error(getApiError(err, t('employee.form.fetchFailed')))
        goto('/employees')
      } finally {
        loading = false
      }
    }
    fetchData()
  })

  // 第二次确认：解密查看前再次弹窗提示，确认后才调用后端解密接口（后端强制记录日志）。
  async function handleReveal() {
    const ok = await modal.confirm({
      title: t('sensitive.revealTitle'),
      content: t('sensitive.revealContent'),
      okText: t('sensitive.confirmView'),
    })
    if (!ok) return
    revealing = true
    try {
      const res = await viewSensitiveEmployee(ensureId())
      if (res.code !== 0) {
        message.error(res.message || t('sensitive.fetchFailed'))
        return
      }
      sensitive = res.data
      message.success(t('sensitive.logged'))
    } catch (err: unknown) {
      message.error(getApiError(err, t('sensitive.fetchFailed')))
    } finally {
      revealing = false
    }
  }

  // 逐字段解密查看：仅解密/显示单个字段，后端按字段细粒度记录日志（如「查看了邮箱」）。
  async function handleRevealField(field: (typeof SENSITIVE_FIELDS)[number]['key'], label: string) {
    const ok = await modal.confirm({
      title: t('sensitive.revealFieldTitle'),
      content: t('sensitive.revealFieldContent', { label }),
      okText: t('sensitive.confirmView'),
    })
    if (!ok) return
    fieldLoading = field
    try {
      const res = await viewSensitiveField(ensureId(), field)
      if (res.code !== 0) {
        message.error(res.message || t('sensitive.fetchFieldFailed'))
        return
      }
      fieldModal = { open: true, label, value: res.data.value ?? '' }
    } catch (err: unknown) {
      message.error(getApiError(err, t('sensitive.fetchFieldFailed')))
    } finally {
      fieldLoading = null
    }
  }
</script>

{#if !$authStore.permissions.includes('employee:view_sensitive')}
  <Result status="403" title="403" subTitle={t('sensitive.noAccess')}>
    {#snippet extra()}
      <Button type="primary" tooltip={t('common.backList')} onClick={() => goto('/employees')}>{t('common.backList')}</Button>
    {/snippet}
  </Result>
{:else}
  <div style="height:100%;overflow:auto">
    <Spin spinning={loading}>
      <Card title={emp ? t('sensitive.titleWithName', { name: emp.name || emp.username }) : t('sensitive.title')} style="max-width:800px">
        {#if !emp}
          <span style="color:var(--ant-color-text-secondary)">{t('common.noData')}</span>
        {:else}
          {#if sensitive}
            <div
              style="margin-bottom:16px;padding:8px 12px;border:1px solid #b7eb8f;border-radius:6px;background:#f6ffed;color:#389e0d;font-size:13px;line-height:1.6"
            >
              {t('sensitive.decryptedNote')}
            </div>
          {/if}
          <table style="width:100%;border-collapse:collapse">
            <tbody>
              <tr>
                <th
                  style="padding:12px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top"
                >
                  {t('common.username')}
                </th>
                <td style="padding:12px 24px;border:1px solid var(--ant-color-border-secondary);vertical-align:top">{emp.username}</td>
              </tr>
              <tr>
                <th style="padding:12px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top">
                  {t('common.name')}
                </th>
                <td style="padding:12px 24px;border:1px solid var(--ant-color-border-secondary);vertical-align:top">{emp.name}</td>
              </tr>
              <tr>
                <th style="padding:12px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top">
                  {t('common.title')}
                </th>
                <td style="padding:12px 24px;border:1px solid var(--ant-color-border-secondary);vertical-align:top">{emp.title}</td>
              </tr>
              {#each SENSITIVE_FIELDS as f (f.key)}
                <tr>
                  <th style="padding:12px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top">
                    {f.label}
                  </th>
                  <td style="padding:12px 24px;border:1px solid var(--ant-color-border-secondary);vertical-align:top">
                    <span style="margin-right:12px">{sensitive ? sensitive[f.key] : emp[f.key]}</span>
                    <Button type="link" size="small" tooltip={t('sensitive.fieldTooltip', { label: f.label })} loading={fieldLoading === f.key} onClick={() => handleRevealField(f.key, f.label)}>
                      <Icon name="eye" style="font-size:13px" /> {t('sensitive.show')}
                    </Button>
                  </td>
                </tr>
              {/each}
              <tr>
                <th style="padding:12px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top">
                  {t('employee.form.hireDate')}
                </th>
                <td style="padding:12px 24px;border:1px solid var(--ant-color-border-secondary);vertical-align:top">{emp.hire_date}</td>
              </tr>
            </tbody>
          </table>
          <div style="margin-top:16px;display:flex;gap:12px;align-items:center">
            {#if !sensitive}
              <Button type="primary" loading={revealing} tooltip={t('sensitive.revealTooltip')} onClick={handleReveal}>
                {t('sensitive.viewFull')}
              </Button>
            {:else}
              <span style="color:var(--ant-color-text-secondary);font-size:13px">
                <Icon name="eye" style="font-size:14px" /> {t('sensitive.viewed')}
              </span>
            {/if}
            <Button tooltip={t('common.backList')} onClick={() => goto('/employees')}>{t('common.backList')}</Button>
          </div>
        {/if}
      </Card>
    </Spin>
  </div>

  <Modal
    open={fieldModal.open}
    title={`${fieldModal.label} - ${emp?.name || ''}`}
    onclose={() => (fieldModal = { open: false, label: '', value: '' })}
    onOk={() => (fieldModal = { open: false, label: '', value: '' })}
    okText={t('employees.gotIt')}
    cancelText={t('common.closeBtn')}
    width={420}
  >
    <div style="font-size:15px;line-height:1.8;word-break:break-all">
      {#if fieldModal.value}
        {fieldModal.value}
      {:else}
        <span style="color:var(--ant-color-text-secondary)">{t('sensitive.notProvided')}</span>
      {/if}
    </div>
    <div style="margin-top:12px;font-size:12px;color:var(--ant-color-text-secondary)">
      {t('sensitive.viewLogNote')}
    </div>
  </Modal>
{/if}
