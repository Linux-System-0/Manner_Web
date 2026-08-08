<script lang="ts">
  // ListItem：列表行（供 List children 内使用）
  import type { Snippet } from 'svelte'
  import Avatar from './Avatar.svelte'

  let {
    title = '',
    description = '',
    avatarSrc = null,
    avatarText = '',
    selected = false,
    onclick,
    children,
    extra,
  }: {
    title?: string
    description?: string
    avatarSrc?: string | null
    avatarText?: string
    selected?: boolean
    onclick?: () => void
    children?: Snippet
    extra?: Snippet
  } = $props()
</script>

<!-- 列表行可点击时按按钮语义暴露（li 自身无交互角色，用注释豁免 a11y 静态检查） -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_interactive_supports_focus, a11y_no_noninteractive_tabindex -->
<li
  class="ant-list-item"
  class:ant-list-item-selected={selected}
  style="display:flex;align-items:center;gap:12px;padding:12px 16px;cursor:{onclick ? 'pointer' : 'default'};transition:background 0.2s;border-bottom:1px solid var(--ant-list-item-border)"
  role={onclick ? 'button' : undefined}
  tabindex={onclick ? 0 : undefined}
  onclick={onclick}
  onkeydown={(e) => {
    if (!onclick) return
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      onclick()
    }
  }}
>
  {#if avatarSrc || avatarText}
    <Avatar src={avatarSrc} size={36}>
      {#if avatarText}<span style="font-size:16px">{avatarText}</span>{/if}
    </Avatar>
  {/if}
  <div style="flex:1;min-width:0">
    <div style="color:var(--ant-color-text);font-size:14px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{title}</div>
    {#if description}
      <div style="color:var(--ant-color-text-secondary);font-size:12px;margin-top:2px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">{description}</div>
    {/if}
    {#if children}
      <div style="margin-top:4px">{@render children()}</div>
    {/if}
  </div>
  {#if extra}
    <div style="flex-shrink:0">{@render extra()}</div>
  {/if}
</li>

<style>
  .ant-list-item:hover {
    background: var(--ant-color-fill-quaternary);
  }
  .ant-list-item-selected {
    background: var(--chat-selected-bg) !important;
  }
</style>
