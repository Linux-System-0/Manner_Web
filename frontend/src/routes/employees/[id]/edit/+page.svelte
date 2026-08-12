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
  import { onMount } from 'svelte'
  // 员工管理 - 编辑（复刻 React 版 frontend/src/pages/employees/Form.tsx）
  import { page } from '$app/stores'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { getApiError } from '$lib/api/client'
  import { t } from '$lib/i18n'
  import { getEmployee, updateEmployee } from '$lib/api/employees'
  import { getDepartments, updateEmployeeDepartments } from '$lib/api/departments'
  import Card from '$lib/components/Card.svelte'
  import Form from '$lib/components/Form.svelte'
  import FormItem from '$lib/components/FormItem.svelte'
  import Input from '$lib/components/Input.svelte'
  import DatePicker from '$lib/components/DatePicker.svelte'
  import Select from '$lib/components/Select.svelte'
  import Button from '$lib/components/Button.svelte'
  import Spin from '$lib/components/Spin.svelte'
  import Result from '$lib/components/Result.svelte'
  import { message } from '$lib/components/message'

  const id = $page.params.id

  function ensureId(): string {
    if (!id) {
      message.error(t('employee.form.missingId'))
      goto('/employees')
      throw new Error('missing employee id')
    }
    return id
  }

  let username = $state('')
  let name = $state('')
  let title = $state('')
  let email = $state('')
  let phone = $state('')
  let idNumber = $state('')
  let address = $state('')
  let hireDate = $state('')
  let departmentIds = $state<string[]>([])
  let deptOptions = $state<{ value: string; label: string }[]>([])
  // 敏感字段原始掩码（判断是否修改：未修改则跳过提交，后端保留原密文）
  let origEmail = ''
  let origPhone = ''
  let origIdNumber = ''
  let origAddress = ''
  let loading = $state(true)
  let submitting = $state(false)

  let errors = $state<Record<string, string>>({})

  onMount(() => {

    if (!$authStore.permissions.includes('employee:edit')) {
      loading = false
      return
    }

    const fetchEmployee = async () => {
      loading = true
      try {
        const res = await getEmployee(ensureId())
        if (res.code !== 0) {
          message.error(res.message || t('employee.form.fetchFailed'))
          goto('/employees')
          return
        }
        const emp = res.data
        username = emp.username
        name = emp.name || ''
        title = emp.title || ''
        origEmail = emp.email || ''
        origPhone = emp.phone || ''
        origIdNumber = emp.id_number || ''
        origAddress = emp.address || ''
        email = emp.email || ''
        phone = emp.phone || ''
        idNumber = emp.id_number || ''
        address = emp.address || ''
        hireDate = emp.hire_date || ''
        departmentIds = emp.department_ids || []
      } catch (err: unknown) {
        message.error(getApiError(err, t('employee.form.fetchFailed')))
        goto('/employees')
      } finally {
        loading = false
      }
    }
    fetchEmployee()

    getDepartments()
      .then((res) => {
        if (res.code === 0) {
          deptOptions = res.data.items.map((d) => ({ value: d.id, label: d.name }))
        }
      })
      .catch(() => {})
  })

  function validate(): boolean {
    const next: Record<string, string> = {}
    if (!username.trim()) next.username = t('employee.form.errUsername')
    else if (username.trim().length < 3) next.username = t('employee.form.errUsernameLen')
    if (!name.trim()) next.name = t('employee.form.errName')
    if (email && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) next.email = t('employee.form.errEmail')
    errors = next
    return Object.keys(next).length === 0
  }

  async function handleSubmit() {
    if (!validate()) return
    submitting = true
    try {
      const res = await updateEmployee(ensureId(), {
        name: name.trim(),
        title: title.trim() || undefined,
        // 敏感字段未修改（仍为原始掩码 ***）则不提交，后端保留原密文
        email: email.trim() === origEmail ? undefined : email.trim() || undefined,
        phone: phone.trim() === origPhone ? undefined : phone.trim() || undefined,
        id_number: idNumber.trim() === origIdNumber ? undefined : idNumber.trim() || undefined,
        address: address.trim() === origAddress ? undefined : address.trim() || undefined,
        hire_date: hireDate || undefined,
      })
      if (res.code !== 0) {
        message.error(res.message || t('employee.form.updateFailed'))
        return
      }
      // 归属部门独立更新（多对多整体替换）
      const deptRes = await updateEmployeeDepartments(ensureId(), departmentIds)
      if (deptRes.code !== 0) {
        message.error(deptRes.message || t('employee.form.deptUpdateFailed'))
        return
      }
      message.success(t('common.updatedSuccess'))
      goto('/employees')
    } catch (err: unknown) {
      message.error(getApiError(err, t('employee.form.updateFailed')))
    } finally {
      submitting = false
    }
  }
</script>

{#if !$authStore.permissions.includes('employee:edit')}
  <Result status="403" title="403" subTitle={t('common.noAccess')}>
    {#snippet extra()}
      <Button type="primary" tooltip={t('common.backList')} onClick={() => goto('/employees')}>{t('common.backList')}</Button>
    {/snippet}
  </Result>
{:else}
  <div style="height:100%;overflow:auto">
    <Spin spinning={loading}>
      <Card title={t('employee.form.editTitle')} style="max-width:800px">
        <div
          style="margin-bottom:16px;padding:8px 12px;border:1px solid #ffd591;border-radius:6px;background:#fff7e6;color:#d46b08;font-size:13px;line-height:1.6"
        >
          {t('employee.form.sensitiveNote')}
        </div>
        <Form class="ant-form-vertical" onSubmit={(e) => { e.preventDefault(); handleSubmit() }}>
          <FormItem label={t('employee.form.username')} required={true} error={errors.username}>
            <Input
              placeholder={t('employee.form.placeholderUsername')}
              value={username}
              disabled={true}
              onInput={(v) => { username = v; errors = { ...errors, username: '' } }}
            />
          </FormItem>

          <FormItem label={t('employee.form.name')} required={true} error={errors.name}>
            <Input
              placeholder={t('employee.form.placeholderName')}
              value={name}
              onInput={(v) => { name = v; errors = { ...errors, name: '' } }}
            />
          </FormItem>

          <FormItem label={t('employee.form.titleField')} error={errors.title}>
            <Input
              placeholder={t('employee.form.placeholderTitle')}
              value={title}
              onInput={(v) => (title = v)}
            />
          </FormItem>

          <FormItem label={t('employee.form.email')} error={errors.email}>
            <Input
              placeholder={t('employee.form.placeholderEmail')}
              value={email}
              onInput={(v) => { email = v; errors = { ...errors, email: '' } }}
            />
          </FormItem>

          <FormItem label={t('employee.form.phone')} error={errors.phone}>
            <Input
              placeholder={t('employee.form.placeholderPhone')}
              value={phone}
              onInput={(v) => (phone = v)}
            />
          </FormItem>

          <FormItem label={t('employee.form.idNumber')} error={errors.id_number}>
            <Input
              placeholder={t('employee.form.placeholderIdNumber')}
              value={idNumber}
              onInput={(v) => (idNumber = v)}
            />
          </FormItem>

          <FormItem label={t('employee.form.address')} error={errors.address}>
            <Input
              type="textarea"
              rows={2}
              placeholder={t('employee.form.placeholderAddress')}
              value={address}
              onInput={(v) => (address = v)}
            />
          </FormItem>

          <FormItem label={t('employee.form.hireDate')} error={errors.hire_date}>
            <DatePicker
              value={hireDate}
              placeholder={t('employee.form.placeholderHireDate')}
              onChange={(v) => (hireDate = v)}
            />
          </FormItem>

          <FormItem label={t('employee.form.departments')} error={errors.department_ids}>
            <Select
              value={departmentIds as never[]}
              options={deptOptions}
              multiple={true}
              placeholder={t('employee.form.departmentsPlaceholder')}
              onChange={(v) => (departmentIds = (Array.isArray(v) ? v : []) as string[])}
            />
          </FormItem>

          <FormItem label="">
            <div style="display:flex;gap:12px">
              <Button type="primary" htmlType="submit" loading={submitting} tooltip={t('employee.form.saveTooltip')}>{t('employee.form.save')}</Button>
              <Button tooltip={t('employee.form.cancelTooltip')} onClick={() => goto('/employees')}>{t('common.cancel')}</Button>
            </div>
          </FormItem>
        </Form>
      </Card>
    </Spin>
  </div>
{/if}

<style>
  :global(.ant-form-vertical .ant-form-item) {
    flex-direction: column;
    align-items: stretch;
    row-gap: 4px;
  }
  :global(.ant-form-vertical .ant-form-item-label) {
    flex: none !important;
    width: 100% !important;
    padding-right: 0 !important;
    text-align: left !important;
    line-height: 1.5715 !important;
  }
</style>
