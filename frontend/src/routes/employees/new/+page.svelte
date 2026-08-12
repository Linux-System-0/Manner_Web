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
  // 员工管理 - 新增（复刻 React 版 frontend/src/pages/employees/Form.tsx）
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { getApiError } from '$lib/api/client'
  import { t } from '$lib/i18n'
  import { createEmployee } from '$lib/api/employees'
  import Card from '$lib/components/Card.svelte'
  import Form from '$lib/components/Form.svelte'
  import FormItem from '$lib/components/FormItem.svelte'
  import Input from '$lib/components/Input.svelte'
  import DatePicker from '$lib/components/DatePicker.svelte'
  import Button from '$lib/components/Button.svelte'
  import Modal from '$lib/components/Modal.svelte'
  import Result from '$lib/components/Result.svelte'
  import { message } from '$lib/components/message'

  let username = $state('')
  let name = $state('')
  let title = $state('')
  let email = $state('')
  let phone = $state('')
  let idNumber = $state('')
  let address = $state('')
  let hireDate = $state('')
  let submitting = $state(false)

  // F-08: 创建成功后的初始密码展示弹窗（页面内弹窗展示，不用 toast 提示显示密码）
  let pwdModal = $state({ open: false, password: '' })

  let errors = $state<Record<string, string>>({})

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
      const res = await createEmployee({
        username: username.trim(),
        name: name.trim(),
        title: title.trim() || undefined,
        email: email.trim() || undefined,
        phone: phone.trim() || undefined,
        id_number: idNumber.trim() || undefined,
        address: address.trim() || undefined,
        hire_date: hireDate || undefined,
      })
      if (res.code !== 0) {
        message.error(res.message || t('common.createdFailed'))
        return
      }
      // F-02: 后端生成一次性初始密码，仅在此响应中返回一次
      const initialPassword = (res.data as unknown as { initial_password?: string })?.initial_password
      if (initialPassword) {
        // F-08: 初始密码改为页面内弹窗展示，关闭弹窗后再跳转列表
        pwdModal = { open: true, password: initialPassword }
        return
      }
      message.success(t('common.createdSuccess'))
      goto('/employees')
    } catch (err: unknown) {
      message.error(getApiError(err, t('common.createdFailed')))
    } finally {
      submitting = false
    }
  }

  function closePwdModal() {
    pwdModal = { open: false, password: '' }
    message.success(t('common.createdSuccess'))
    goto('/employees')
  }

  async function copyPwd() {
    try {
      await navigator.clipboard.writeText(pwdModal.password)
      message.success(t('common.copied'))
    } catch {
      message.error(t('common.copyFailed'))
    }
  }
</script>

{#if !$authStore.permissions.includes('employee:create')}
  <Result status="403" title="403" subTitle={t('common.noAccess')}>
    {#snippet extra()}
      <Button type="primary" tooltip={t('common.backList')} onClick={() => goto('/employees')}>{t('common.backList')}</Button>
    {/snippet}
  </Result>
{:else}
  <div style="height:100%;overflow:auto">
    <Card title={t('employee.form.title')} style="max-width:800px">
      <Form class="ant-form-vertical" onSubmit={(e) => { e.preventDefault(); handleSubmit() }}>
        <FormItem label={t('employee.form.username')} required={true} error={errors.username}>
          <Input
            placeholder={t('employee.form.placeholderUsername')}
            value={username}
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

        <FormItem label="">
          <div style="display:flex;gap:12px">
            <Button type="primary" htmlType="submit" loading={submitting} tooltip={t('employee.form.createTooltip')}>{t('employee.form.create')}</Button>
            <Button tooltip={t('employee.form.cancelTooltip')} onClick={() => goto('/employees')}>{t('common.cancel')}</Button>
          </div>
        </FormItem>
      </Form>
    </Card>
  </div>

  <!-- F-08: 初始密码展示弹窗（一次性密码，禁止遮罩点击误关） -->
  <Modal
    open={pwdModal.open}
    title={t('employee.form.createdTitle')}
    onclose={closePwdModal}
    onOk={closePwdModal}
    okText={t('employees.gotIt')}
    cancelText={t('common.closeBtn')}
    maskClosable={false}
  >
    <div style="display:flex;flex-direction:column;gap:12px">
      <span style="color:var(--ant-color-text-secondary)">
        {t('employee.form.createdNote')}
      </span>
      <div style="display:flex;align-items:center;gap:8px">
        <code
          style="flex:1;padding:8px 12px;border:1px solid var(--ant-color-border-secondary);border-radius:6px;background:var(--ant-color-fill-secondary);font-size:16px;letter-spacing:1px;user-select:all"
        >{pwdModal.password}</code>
        <Button size="small" tooltip={t('employees.copyInitialPwd')} onClick={copyPwd}>{t('common.copy')}</Button>
      </div>
      <span style="color:var(--ant-color-warning)">{t('employee.form.mustChangePassword')}</span>
    </div>
  </Modal>
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
