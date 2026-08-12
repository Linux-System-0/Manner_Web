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
  // 个人资料（复刻 React 版 frontend/src/pages/profile/Index.tsx）
  import { authStore } from '$lib/stores/auth'
  import { getApiError } from '$lib/api/client'
  import { t } from '$lib/i18n'
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
        message.error(res.message || t('profile.avatarFailed'))
        return
      }
      authStore.setUser({ ...me, avatar: avatarUrl })
      message.success(t('profile.avatarUpdated'))
    } catch (err: unknown) {
      message.error(getApiError(err, t('profile.avatarFailed')))
    } finally {
      uploading = false
    }
  }

  function validate(): boolean {
    const next: Record<string, string> = {}
    if (!oldPassword) next.oldPassword = t('profile.errCurrentPassword')
    else if (oldPassword.length < 8) next.oldPassword = t('profile.errPasswordLen')
    if (!newPassword) next.newPassword = t('profile.errNewPassword')
    else if (newPassword.length < 8) next.newPassword = t('profile.errPasswordLen')
    if (!confirmPassword) next.confirmPassword = t('profile.errConfirmPassword')
    else if (confirmPassword !== newPassword) next.confirmPassword = t('profile.errPasswordMismatch')
    errors = next
    return Object.keys(next).length === 0
  }

  async function handleChangePassword() {
    if (!validate()) return
    submitting = true
    try {
      const res = await changePassword(oldPassword, newPassword)
      if (res.code !== 0) {
        message.error(res.message || t('profile.changeFailed'))
        return
      }
      // F-08: 改密后 pwd_version 递增，当前会话随之下一次请求失效，提示重新登录
      message.success(t('profile.changedSuccess'))
      oldPassword = ''
      newPassword = ''
      confirmPassword = ''
      errors = {}
    } catch (err: unknown) {
      message.error(getApiError(err, t('profile.changeFailed')))
    } finally {
      submitting = false
    }
  }
</script>

<div style="height:100%;overflow:auto">
  <Title level={4} style="margin-bottom:24px">{t('profile.title')}</Title>

  <Row gutter={[24, 24]}>
    <Col span={12}>
      <Card title={t('profile.basicInfo')}>
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
              <Button loading={uploading} tooltip={t('profile.avatarTooltip')}>
                <Icon name="upload" style="font-size:14px" />{t('profile.changeAvatar')}
              </Button>
            </Upload>
          </div>
        </div>

        <table style="width:100%;border-collapse:collapse">
          <tbody>
            <tr>
              <th style="padding:16px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top;width:110px">
                {t('profile.username')}
              </th>
              <td style="padding:16px 24px;color:var(--ant-color-text);border:1px solid var(--ant-color-border-secondary);vertical-align:top">
                {$authStore.user?.username || '-'}
              </td>
            </tr>
            <tr>
              <th style="padding:16px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top;width:110px">
                {t('profile.name')}
              </th>
              <td style="padding:16px 24px;color:var(--ant-color-text);border:1px solid var(--ant-color-border-secondary);vertical-align:top">
                {$authStore.user?.name || '-'}
              </td>
            </tr>
            <tr>
              <th style="padding:16px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top;width:110px">
                {t('profile.email')}
              </th>
              <td style="padding:16px 24px;color:var(--ant-color-text);border:1px solid var(--ant-color-border-secondary);vertical-align:top">
                {$authStore.user?.email || '-'}
              </td>
            </tr>
            <tr>
              <th style="padding:16px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top;width:110px">
                {t('profile.titleField')}
              </th>
              <td style="padding:16px 24px;color:var(--ant-color-text);border:1px solid var(--ant-color-border-secondary);vertical-align:top">
                {$authStore.user?.title || '-'}
              </td>
            </tr>
            <tr>
              <th style="padding:16px 24px;font-weight:500;color:var(--ant-color-text-secondary);background:var(--ant-color-fill-quaternary);border:1px solid var(--ant-color-border-secondary);text-align:right;white-space:nowrap;vertical-align:top;width:110px">
                {t('profile.phone')}
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
      <Card title={t('profile.changePassword')}>
        <Form class="ant-form-vertical" onSubmit={(e) => { e.preventDefault(); handleChangePassword() }}>
          <FormItem label={t('profile.currentPassword')} required={true} error={errors.oldPassword}>
            <Input
              type="password"
              prefix="lock"
              placeholder={t('profile.currentPasswordPlaceholder')}
              value={oldPassword}
              onInput={(v) => { oldPassword = v; errors = { ...errors, oldPassword: '' } }}
            />
          </FormItem>

          <FormItem label={t('profile.newPassword')} required={true} error={errors.newPassword}>
            <Input
              type="password"
              prefix="lock"
              placeholder={t('profile.newPasswordPlaceholder')}
              value={newPassword}
              onInput={(v) => { newPassword = v; errors = { ...errors, newPassword: '', confirmPassword: '' } }}
            />
          </FormItem>

          <FormItem label={t('profile.confirmPassword')} required={true} error={errors.confirmPassword}>
            <Input
              type="password"
              prefix="lock"
              placeholder={t('profile.confirmPasswordPlaceholder')}
              value={confirmPassword}
              onInput={(v) => { confirmPassword = v; errors = { ...errors, confirmPassword: '' } }}
            />
          </FormItem>

          <FormItem label="">
            <Button type="primary" htmlType="submit" loading={submitting} tooltip={t('profile.changeTooltip')}>{t('profile.changePassword')}</Button>
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
