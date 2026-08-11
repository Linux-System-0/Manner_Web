<script lang="ts">
  // 员工敏感信息查看页（employee:view_sensitive）
  // 流程：列表操作入口（第一次确认）→ 本页（仍显示掩码 ***）→ 点击「查看完整信息」
  // （第二次确认）→ 后端解密 API（身份验证 + 强制写日志）→ 明文展示。
  import { onMount } from 'svelte'
  import { page } from '$app/stores'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { getApiError } from '$lib/api/client'
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
  const SENSITIVE_FIELDS = [
    { key: 'email', label: '邮箱' },
    { key: 'phone', label: '手机号' },
    { key: 'id_number', label: '身份证号' },
    { key: 'address', label: '地址' },
  ] as const

  function ensureId(): string {
    if (!id) {
      message.error('缺少员工 ID')
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
          message.error(res.message || '获取员工信息失败')
          goto('/employees')
          return
        }
        emp = res.data
      } catch (err: unknown) {
        message.error(getApiError(err, '获取员工信息失败'))
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
      title: '解密查看敏感信息',
      content: '确认后将解密显示该员工的手机号 / 邮箱 / 身份证号 / 地址，本次查看会记录到系统日志。',
      okText: '确认查看',
    })
    if (!ok) return
    revealing = true
    try {
      const res = await viewSensitiveEmployee(ensureId())
      if (res.code !== 0) {
        message.error(res.message || '获取敏感信息失败')
        return
      }
      sensitive = res.data
      message.success('已记录到系统日志')
    } catch (err: unknown) {
      message.error(getApiError(err, '获取敏感信息失败'))
    } finally {
      revealing = false
    }
  }

  // 逐字段解密查看：仅解密/显示单个字段，后端按字段细粒度记录日志（如「查看了邮箱」）。
  async function handleRevealField(field: (typeof SENSITIVE_FIELDS)[number]['key'], label: string) {
    const ok = await modal.confirm({
      title: '解密查看',
      content: `确认后将解密显示该员工的${label}，本次查看会记录到系统日志。`,
      okText: '确认查看',
    })
    if (!ok) return
    fieldLoading = field
    try {
      const res = await viewSensitiveField(ensureId(), field)
      if (res.code !== 0) {
        message.error(res.message || '获取失败')
        return
      }
      fieldModal = { open: true, label, value: res.data.value ?? '' }
    } catch (err: unknown) {
      message.error(getApiError(err, '获取失败'))
    } finally {
      fieldLoading = null
    }
  }
</script>

{#if !$authStore.permissions.includes('employee:view_sensitive')}
  <Result status="403" title="403" subTitle="抱歉，你无权查看敏感信息">
    {#snippet extra()}
      <Button type="primary" tooltip="返回员工列表页" onClick={() => goto('/employees')}>返回列表</Button>
    {/snippet}
  </Result>
{:else}
  <div style="height:100%;overflow:auto">
    <Spin spinning={loading}>
      <Card title={emp ? `敏感信息 - ${emp.name || emp.username}` : '敏感信息'} style="max-width:800px">
        {#if !emp}
          <span style="color:var(--ant-color-text-secondary)">暂无数据</span>
        {:else}
          {#if sensitive}
            <div
              style="margin-bottom:16px;padding:8px 12px;border:1px solid #b7eb8f;border-radius:6px;background:#f6ffed;color:#389e0d;font-size:13px;line-height:1.6"
            >
              已解密显示以下完整信息，本次查看已记录到系统日志（可在「系统设置 → 日志」中查阅）。
            </div>
          {/if}
          <table style="width:100%;border-collapse:collapse">
            <tbody>
              <tr>
                <th
                  style="padding:12px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top"
                >
                  用户名
                </th>
                <td style="padding:12px 24px;border:1px solid var(--ant-color-border-secondary);vertical-align:top">{emp.username}</td>
              </tr>
              <tr>
                <th style="padding:12px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top">
                  姓名
                </th>
                <td style="padding:12px 24px;border:1px solid var(--ant-color-border-secondary);vertical-align:top">{emp.name}</td>
              </tr>
              <tr>
                <th style="padding:12px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top">
                  职位
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
                    <Button type="link" size="small" tooltip={`点击查看${f.label}明文`} loading={fieldLoading === f.key} onClick={() => handleRevealField(f.key, f.label)}>
                      <Icon name="eye" style="font-size:13px" /> 显示
                    </Button>
                  </td>
                </tr>
              {/each}
              <tr>
                <th style="padding:12px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top">
                  入职日期
                </th>
                <td style="padding:12px 24px;border:1px solid var(--ant-color-border-secondary);vertical-align:top">{emp.hire_date}</td>
              </tr>
            </tbody>
          </table>
          <div style="margin-top:16px;display:flex;gap:12px;align-items:center">
            {#if !sensitive}
              <Button type="primary" loading={revealing} tooltip="一键查看所有敏感字段明文" onClick={handleReveal}>
                查看完整信息
              </Button>
            {:else}
              <span style="color:var(--ant-color-text-secondary);font-size:13px">
                <Icon name="eye" style="font-size:14px" /> 已查看（已记录日志）
              </span>
            {/if}
            <Button tooltip="返回员工列表页" onClick={() => goto('/employees')}>返回列表</Button>
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
    okText="知道了"
    cancelText="关闭"
    width={420}
  >
    <div style="font-size:15px;line-height:1.8;word-break:break-all">
      {#if fieldModal.value}
        {fieldModal.value}
      {:else}
        <span style="color:var(--ant-color-text-secondary)">（该员工未填写此项）</span>
      {/if}
    </div>
    <div style="margin-top:12px;font-size:12px;color:var(--ant-color-text-secondary)">
      本次查看已记录到系统日志，可在「系统设置 → 日志」中查阅。
    </div>
  </Modal>
{/if}
