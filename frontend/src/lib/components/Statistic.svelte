<script lang="ts">
  // Statistic：统计数值
  let {
    title = '',
    value = 0,
    suffix = '',
    prefix = '',
    style = '',
  }: {
    title?: string
    value?: number | string
    suffix?: string
    prefix?: string
    style?: string
  } = $props()
</script>

<div class="ant-statistic" style="{style}">
  <div class="ant-statistic-title" style="font-size:14px;color:var(--ant-color-text-secondary);margin-bottom:4px">{title}</div>
  <div class="ant-statistic-content" style="font-size:24px;font-weight:600;color:var(--ant-color-text);line-height:1.2">
    {#if prefix}<span class="ant-statistic-content-prefix" style="margin-right:4px">{prefix}</span>{/if}
    {value}
    {#if suffix}<span class="ant-statistic-content-suffix" style="margin-left:4px;font-size:14px;font-weight:400">{suffix}</span>{/if}
  </div>
</div>
