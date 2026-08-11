<script lang="ts">
  import { onMount } from 'svelte'
  // 员工管理 - 编辑（复刻 React 版 frontend/src/pages/employees/Form.tsx）
  import { page } from '$app/stores'
  import { goto } from '$app/navigation'
  import { authStore } from '$lib/stores/auth'
  import { getApiError } from '$lib/api/client'
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
      message.error('缺少员工 ID')
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
          message.error(res.message || '获取员工信息失败')
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
        message.error(getApiError(err, '获取员工信息失败'))
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
    if (!username.trim()) next.username = '请输入用户名'
    else if (username.trim().length < 3) next.username = '用户名至少3个字符'
    if (!name.trim()) next.name = '请输入姓名'
    if (email && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) next.email = '请输入有效的邮箱地址'
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
        message.error(res.message || '更新失败')
        return
      }
      // 归属部门独立更新（多对多整体替换）
      const deptRes = await updateEmployeeDepartments(ensureId(), departmentIds)
      if (deptRes.code !== 0) {
        message.error(deptRes.message || '归属部门更新失败')
        return
      }
      message.success('更新成功')
      goto('/employees')
    } catch (err: unknown) {
      message.error(getApiError(err, '更新失败'))
    } finally {
      submitting = false
    }
  }
</script>

{#if !$authStore.permissions.includes('employee:edit')}
  <Result status="403" title="403" subTitle="抱歉，你无权访问该页面">
    {#snippet extra()}
      <Button type="primary" tooltip="返回员工列表页" onClick={() => goto('/employees')}>返回列表</Button>
    {/snippet}
  </Result>
{:else}
  <div style="height:100%;overflow:auto">
    <Spin spinning={loading}>
      <Card title="编辑员工" style="max-width:800px">
        <div
          style="margin-bottom:16px;padding:8px 12px;border:1px solid #ffd591;border-radius:6px;background:#fff7e6;color:#d46b08;font-size:13px;line-height:1.6"
        >
          手机号 / 邮箱 / 身份证号 / 地址已加密存储，此处仅显示掩码（***）。如需修改请重新输入；
          未改动的字段将保留原值，不会被覆盖。
        </div>
        <Form class="ant-form-vertical" onSubmit={(e) => { e.preventDefault(); handleSubmit() }}>
          <FormItem label="用户名" required={true} error={errors.username}>
            <Input
              placeholder="请输入用户名"
              value={username}
              disabled={true}
              onInput={(v) => { username = v; errors = { ...errors, username: '' } }}
            />
          </FormItem>

          <FormItem label="姓名" required={true} error={errors.name}>
            <Input
              placeholder="请输入姓名"
              value={name}
              onInput={(v) => { name = v; errors = { ...errors, name: '' } }}
            />
          </FormItem>

          <FormItem label="职位" error={errors.title}>
            <Input
              placeholder="请输入职位"
              value={title}
              onInput={(v) => (title = v)}
            />
          </FormItem>

          <FormItem label="邮箱" error={errors.email}>
            <Input
              placeholder="请输入邮箱"
              value={email}
              onInput={(v) => { email = v; errors = { ...errors, email: '' } }}
            />
          </FormItem>

          <FormItem label="手机号" error={errors.phone}>
            <Input
              placeholder="请输入手机号"
              value={phone}
              onInput={(v) => (phone = v)}
            />
          </FormItem>

          <FormItem label="身份证号" error={errors.id_number}>
            <Input
              placeholder="请输入身份证号"
              value={idNumber}
              onInput={(v) => (idNumber = v)}
            />
          </FormItem>

          <FormItem label="地址" error={errors.address}>
            <Input
              type="textarea"
              rows={2}
              placeholder="请输入地址"
              value={address}
              onInput={(v) => (address = v)}
            />
          </FormItem>

          <FormItem label="入职日期" error={errors.hire_date}>
            <DatePicker
              value={hireDate}
              placeholder="请选择日期"
              onChange={(v) => (hireDate = v)}
            />
          </FormItem>

          <FormItem label="归属部门" error={errors.department_ids}>
            <Select
              value={departmentIds as never[]}
              options={deptOptions}
              multiple={true}
              placeholder="可选择多个部门"
              onChange={(v) => (departmentIds = (Array.isArray(v) ? v : []) as string[])}
            />
          </FormItem>

          <FormItem label="">
            <div style="display:flex;gap:12px">
              <Button type="primary" htmlType="submit" loading={submitting} tooltip="保存对员工信息的修改">保存</Button>
              <Button tooltip="放弃修改，返回员工列表" onClick={() => goto('/employees')}>取消</Button>
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
