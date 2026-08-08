<script lang="ts">
  // 个人资料（复刻 React 版 frontend/src/pages/profile/Index.tsx）
  import { authStore } from '$lib/stores/auth'
  import { getApiError } from '$lib/api/client'
  import { changePassword } from '$lib/api/auth'
  import { uploadImage } from '$lib/api/system'
  import { updateEmployee } from '$lib/api/employees'
  import type { UpdateEmployeeRequest } from '$lib/types'
  import Title from '$lib/components/Title.svelte'
  import Card from '$lib/components/Card.svelte'
  import Row from '$lib/components/Row.svelte'
  import Col from '$lib/components/Col.svelte'
  import Avatar from '$lib/components/Avatar.svelte'
  import Upload from '$lib/components/Upload.svelte'
  import Button from '$lib/components/Button.svelte'
  import Form from '$lib/components/Form.svelte'
  import FormItem from '$lib/components/FormItem.svelte'
  import Input from '$lib/components/Input.svelte'
  import Tag from '$lib/components/Tag.svelte'
  import { Icon } from '$lib/icons'
  import { message } from '$lib/components/message'

  let uploading = $state(false)
  let submitting = $state(false)

  // 修改密码表单
  let oldPassword = $state('')
  let newPassword = $state('')
  let confirmPassword = $state('')
  let errors = $state<Record<string, string>>({})

  async function handleAvatarUpload(file: File) {
    const me = $authStore.user
    if (!me) return
    uploading = true
    try {
      const avatarUrl = await uploadImage(file)
      // 头像仅允许更新自己（后端校验 only_avatar 分支）
      const res = await updateEmployee(me.id, { avatar: avatarUrl } as UpdateEmployeeRequest)
      if (res.code !== 0) {
        message.error(res.message || '头像上传失败')
        return
      }
      authStore.setUser({ ...me, avatar: avatarUrl })
      message.success('头像更新成功')
    } catch (err: unknown) {
      message.error(getApiError(err, '头像上传失败'))
    } finally {
      uploading = false
    }
  }

  function validate(): boolean {
    const next: Record<string, string> = {}
    if (!oldPassword) next.oldPassword = '请输入当前密码'
    else if (oldPassword.length < 8) next.oldPassword = '密码至少 8 位'
    if (!newPassword) next.newPassword = '请输入新密码'
    else if (newPassword.length < 8) next.newPassword = '密码至少 8 位'
    if (!confirmPassword) next.confirmPassword = '请再次输入新密码'
    else if (confirmPassword !== newPassword) next.confirmPassword = '两次输入的密码不一致'
    errors = next
    return Object.keys(next).length === 0
  }

  async function handleChangePassword() {
    if (!validate()) return
    submitting = true
    try {
      const res = await changePassword(oldPassword, newPassword)
      if (res.code !== 0) {
        message.error(res.message || '密码修改失败')
        return
      }
      // F-08: 改密后 pwd_version 递增，当前会话随之下一次请求失效，提示重新登录
      message.success('密码修改成功，请重新登录')
      oldPassword = ''
      newPassword = ''
      confirmPassword = ''
      errors = {}
    } catch (err: unknown) {
      message.error(getApiError(err, '密码修改失败'))
    } finally {
      submitting = false
    }
  }
</script>

<div style="height:100%;overflow:auto">
  <Title level={4} style="margin-bottom:24px">个人资料</Title>

  <Row gutter={[24, 24]}>
    <Col span={12}>
      <Card title="基本信息">
        <div style="text-align:center;margin-bottom:24px">
          <Avatar size={100} src={$authStore.user?.avatar}>
            {#if !$authStore.user?.avatar}
              <span style="display:inline-flex"><Icon name="user" style="font-size:44px" /></span>
            {/if}
          </Avatar>
          <div style="margin-top:12px">
            <Upload
              accept="image/*"
              beforeUpload={(file) => {
                handleAvatarUpload(file)
                return false
              }}
            >
              <Button loading={uploading}>
                <Icon name="upload" style="font-size:14px" />更换头像
              </Button>
            </Upload>
          </div>
        </div>

        <table style="width:100%;border-collapse:collapse">
          <tbody>
            <tr>
              <th style="padding:16px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top;width:110px">
                用户名
              </th>
              <td style="padding:16px 24px;color:var(--ant-color-text);border:1px solid var(--ant-color-border-secondary);vertical-align:top">
                {$authStore.user?.username || '-'}
              </td>
            </tr>
            <tr>
              <th style="padding:16px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top;width:110px">
                姓名
              </th>
              <td style="padding:16px 24px;color:var(--ant-color-text);border:1px solid var(--ant-color-border-secondary);vertical-align:top">
                {$authStore.user?.name || '-'}
              </td>
            </tr>
            <tr>
              <th style="padding:16px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top;width:110px">
                邮箱
              </th>
              <td style="padding:16px 24px;color:var(--ant-color-text);border:1px solid var(--ant-color-border-secondary);vertical-align:top">
                {$authStore.user?.email || '-'}
              </td>
            </tr>
            <tr>
              <th style="padding:16px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top;width:110px">
                职位
              </th>
              <td style="padding:16px 24px;color:var(--ant-color-text);border:1px solid var(--ant-color-border-secondary);vertical-align:top">
                {$authStore.user?.title || '-'}
              </td>
            </tr>
            <tr>
              <th style="padding:16px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top;width:110px">
                手机号
              </th>
              <td style="padding:16px 24px;color:var(--ant-color-text);border:1px solid var(--ant-color-border-secondary);vertical-align:top">
                {$authStore.user?.phone || '-'}
              </td>
            </tr>
          </tbody>
        </table>
      </Card>
    </Col>

    <Col span={12}>
      <Card title="修改密码">
        <Form class="ant-form-vertical" onSubmit={(e) => { e.preventDefault(); handleChangePassword() }}>
          <FormItem label="当前密码" required={true} error={errors.oldPassword}>
            <Input
              type="password"
              prefix="lock"
              placeholder="请输入当前密码"
              value={oldPassword}
              onInput={(v) => { oldPassword = v; errors = { ...errors, oldPassword: '' } }}
            />
          </FormItem>

          <FormItem label="新密码" required={true} error={errors.newPassword}>
            <Input
              type="password"
              prefix="lock"
              placeholder="请输入新密码"
              value={newPassword}
              onInput={(v) => { newPassword = v; errors = { ...errors, newPassword: '', confirmPassword: '' } }}
            />
          </FormItem>

          <FormItem label="确认新密码" required={true} error={errors.confirmPassword}>
            <Input
              type="password"
              prefix="lock"
              placeholder="请再次输入新密码"
              value={confirmPassword}
              onInput={(v) => { confirmPassword = v; errors = { ...errors, confirmPassword: '' } }}
            />
          </FormItem>

          <FormItem label="">
            <Button type="primary" htmlType="submit" loading={submitting}>修改密码</Button>
          </FormItem>
        </Form>
      </Card>
    </Col>
  </Row>
</div>

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
