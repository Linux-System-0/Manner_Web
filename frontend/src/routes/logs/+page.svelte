<script lang="ts">
  // 应用日志页（复刻原 frontend/src/pages/Logs.tsx）
  // 权限守卫：system:settings（原版路由误用 system:config，重构后按后端权限码修正）
  import { onMount, onDestroy } from 'svelte'
  import { authStore } from '$lib/stores/auth'
  import { getGlobalPrefs } from '$lib/stores/preferences'
  import { t } from '$lib/i18n'
  import { getSystemLogs } from '$lib/api/system'
  import { Icon } from '$lib/icons'
  import Card from '$lib/components/Card.svelte'
  import Button from '$lib/components/Button.svelte'
  import Space from '$lib/components/Space.svelte'
  import Row from '$lib/components/Row.svelte'
  import Col from '$lib/components/Col.svelte'
  import Statistic from '$lib/components/Statistic.svelte'
  import Select from '$lib/components/Select.svelte'
  import Input from '$lib/components/Input.svelte'
  import Result from '$lib/components/Result.svelte'
  import Text from '$lib/components/Text.svelte'

  type LogLevel = 'INFO' | 'WARN' | 'ERROR' | 'DEBUG' | 'LOG'

  interface LogData {
    lines: string[]
    total: number
    file: string
  }

  function detectLevel(line: string): { level: LogLevel; color: string; icon: string } {
    if (/error|失败|异常|exception|fatal|panic/i.test(line)) {
      return { level: 'ERROR', color: '#ff4d4f', icon: 'close' }
    }
    if (/warn|警告|warning/i.test(line)) {
      return { level: 'WARN', color: '#faad14', icon: 'exclamation-circle' }
    }
    if (/info|登录|成功|started|completed/i.test(line)) {
      return { level: 'INFO', color: '#1677ff', icon: 'info-circle' }
    }
    if (/debug|调试/i.test(line)) {
      return { level: 'DEBUG', color: '#52c41a', icon: 'file-text' }
    }
    if (/应用|系统|用户|登录/i.test(line)) {
      return { level: 'INFO', color: '#1677ff', icon: 'info-circle' }
    }
    return { level: 'LOG', color: '#888', icon: 'info-circle' }
  }

  function extractTime(line: string): string {
    const match = line.match(/^\[([^\]]+)\]/)
    return match ? match[1] : ''
  }

  function convertTime(timeStr: string): string {
    if (!timeStr) return ''
    const prefs = getGlobalPrefs()
    const d = new Date(timeStr.replace(' ', 'T') + 'Z')
    if (isNaN(d.getTime())) return timeStr
    if (prefs.timezoneMode === 'manual') {
      const local = new Date(d.getTime() + prefs.timezoneOffset * 3600000)
      const y = local.getUTCFullYear()
      const m = String(local.getUTCMonth() + 1).padStart(2, '0')
      const day = String(local.getUTCDate()).padStart(2, '0')
      const h = String(local.getUTCHours()).padStart(2, '0')
      const min = String(local.getUTCMinutes()).padStart(2, '0')
      const s = String(local.getUTCSeconds()).padStart(2, '0')
      return `${y}-${m}-${day} ${h}:${min}:${s}`
    }
    const y = d.getFullYear()
    const m = String(d.getMonth() + 1).padStart(2, '0')
    const day = String(d.getDate()).padStart(2, '0')
    const h = String(d.getHours()).padStart(2, '0')
    const min = String(d.getMinutes()).padStart(2, '0')
    const s = String(d.getSeconds()).padStart(2, '0')
    return `${y}-${m}-${day} ${h}:${min}:${s}`
  }

  let LEVEL_OPTIONS: { value: LogLevel | 'ALL'; label: string; color: string }[] = $derived([
    { value: 'ALL', label: t('logs.all'), color: '#666' },
    { value: 'ERROR', label: t('logs.error'), color: '#ff4d4f' },
    { value: 'WARN', label: t('logs.warn'), color: '#faad14' },
    { value: 'INFO', label: t('logs.info'), color: '#1677ff' },
    { value: 'DEBUG', label: t('logs.debug'), color: '#52c41a' },
    { value: 'LOG', label: t('logs.other'), color: '#888' },
  ])

  let allowed = $derived($authStore.permissions.includes('system:settings'))

  let data = $state<LogData | null>(null)
  let loading = $state(false)
  let error = $state('')
  let levelFilter = $state<LogLevel | 'ALL'>('ALL')
  let keyword = $state('')
  let bottomEl = $state<HTMLDivElement | null>(null)

  const fetchLogs = async () => {
    loading = true
    error = ''
    try {
      const res = await getSystemLogs(200)
      data = res.data
    } catch {
      error = t('logs.fetchFailed')
    } finally {
      loading = false
    }
  }

  let timer: ReturnType<typeof setInterval> | undefined

  onMount(() => {
    if (!allowed) return
    fetchLogs()
    timer = setInterval(fetchLogs, 10000)
  })

  onDestroy(() => {
    if (timer) clearInterval(timer)
  })

  let filteredLines = $derived(
    data
      ? data.lines.filter((line) => {
          if (levelFilter !== 'ALL' && detectLevel(line).level !== levelFilter) return false
          if (keyword && !line.toLowerCase().includes(keyword.toLowerCase())) return false
          return true
        })
      : [],
  )

  let allLevels = $derived.by(() => {
    const counts: Record<string, number> = { ERROR: 0, WARN: 0, INFO: 0, DEBUG: 0, LOG: 0 }
    if (!data) return counts
    for (const line of data.lines) {
      const { level } = detectLevel(line)
      counts[level] = (counts[level] || 0) + 1
    }
    return counts
  })

  // 自动滚动到最新（原版：filteredLines.length 变化时滚动到底部）
  $effect(() => {
    const count = filteredLines.length
    if (bottomEl && count > 0) {
      bottomEl.scrollIntoView({ behavior: 'smooth' })
    }
  })

  const handleExport = () => {
    if (!data) return
    const blob = new Blob([filteredLines.join('\n')], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `app-logs-${new Date().toISOString().slice(0, 10)}.log`
    a.click()
    URL.revokeObjectURL(url)
  }
</script>

{#if !allowed}
  <Result status="403" title="403" subTitle={t('common.noAccess')}>
    {#snippet extra()}
      <Button type="primary" tooltip={t('common.backPrev')} onClick={() => window.history.back()}>{t('common.backPrev')}</Button>
    {/snippet}
  </Result>
{:else}
  <div>
    <Row gutter={[16, 16]} style="margin-bottom:16px">
      {#each LEVEL_OPTIONS.filter((o) => o.value !== 'ALL') as opt (opt.value)}
        <Col span={4}>
          <div
            role="button"
            tabindex={0}
            aria-label={t('logs.filterAria', { label: opt.label })}
            aria-pressed={levelFilter === opt.value}
            onclick={() => (levelFilter = levelFilter === opt.value ? 'ALL' : opt.value)}
            onkeydown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault()
                levelFilter = levelFilter === opt.value ? 'ALL' : opt.value
              }
            }}
            style="cursor:pointer"
          >
            <Card
              hoverable
              bodyStyle="padding:12px 16px"
              style={levelFilter === opt.value ? `border-color:${opt.color}` : ''}
            >
              <div class="stat-{opt.value.toLowerCase()}">
                <Statistic title={opt.label} value={allLevels[opt.value] || 0} />
              </div>
            </Card>
          </div>
        </Col>
      {/each}
      <Col span={4}>
        <Card bodyStyle="padding:12px 16px">
          <Statistic title={t('logs.total')} value={data?.total || 0} />
        </Card>
      </Col>
    </Row>

    <Card>
      {#snippet title()}
        <Space>
          <span style="font-weight:600">{t('logs.title')}</span>
          {#if data}
            <Text type="secondary" style="font-size:12px">
              {t('logs.summary', { total: data.total, filtered: filteredLines.length })}
            </Text>
          {/if}
        </Space>
      {/snippet}
      {#snippet extra()}
        <Space>
          <span class="logs-toolbar">
            <Select
              value={levelFilter}
              onChange={(v) => (levelFilter = String(v) as LogLevel | 'ALL')}
              width="100px"
              options={LEVEL_OPTIONS.map((o) => ({ value: o.value, label: o.label }))}
            />
          </span>
          <Input
            size="small"
            placeholder={t('logs.searchPlaceholder')}
            prefix="search"
            value={keyword}
            onInput={(v) => (keyword = v)}
            style="width:180px"
          />
          <Button size="small" tooltip={t('logs.exportTooltip')} onClick={handleExport}>{t('logs.export')}</Button>
          <Button size="small" tooltip={t('logs.refreshTooltip')} onClick={fetchLogs} loading={loading}>
            {#snippet icon()}<Icon name="reload" />{/snippet}
            {t('logs.refresh')}
          </Button>
        </Space>
      {/snippet}

      {#if error}
        <Text type="danger">{error}</Text>
      {:else if data}
        <div class="log-viewer">
          {#if filteredLines.length === 0}
            <div style="color:#666;text-align:center;padding:40px">{t('logs.noMatch')}</div>
          {:else}
            {#each filteredLines as line, i (i)}
              {@const level = detectLevel(line)}
              {@const time = extractTime(line)}
              {@const content = line.replace(/^\[[^\]]*\]\s*/, '')}
              <div class="log-line" class:log-line-even={i % 2 === 0}>
                <span class="log-no">{data.total - filteredLines.length + i + 1}</span>
                <span class="log-level" style="color:{level.color}">
                  <Icon name={level.icon} style="font-size:11px" />
                  {level.level}
                </span>
                <span class="log-time">{convertTime(time)}</span>
                <span class="log-content">{content}</span>
              </div>
            {/each}
          {/if}
          <div bind:this={bottomEl}></div>
        </div>
      {:else}
        <div style="text-align:center;padding:60px;color:#666">{t('logs.loadHint')}</div>
      {/if}
    </Card>
  </div>
{/if}

<style>
  .log-viewer {
    background: #1a1a2e;
    border-radius: 6px;
    padding: 4px;
    max-height: 540px;
    overflow: auto;
    font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
    font-size: 12px;
    line-height: 1.8;
  }
  .log-line {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 2px 12px;
    background: transparent;
    color: #e0e0e0;
  }
  .log-line-even {
    background: rgba(255, 255, 255, 0.02);
  }
  .log-no {
    color: #555;
    width: 30px;
    text-align: right;
    flex-shrink: 0;
    font-size: 11px;
  }
  .log-level {
    width: 56px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
  }
  .log-time {
    color: #888;
    width: 160px;
    flex-shrink: 0;
    font-size: 11px;
  }
  .log-content {
    flex: 1;
    word-break: break-all;
  }
  /* 统计卡片数值颜色（原版 Statistic valueStyle） */
  :global(.stat-error .ant-statistic-content) {
    color: #ff4d4f;
  }
  :global(.stat-warn .ant-statistic-content) {
    color: #faad14;
  }
  :global(.stat-info .ant-statistic-content) {
    color: #1677ff;
  }
  :global(.stat-debug .ant-statistic-content) {
    color: #52c41a;
  }
  :global(.stat-log .ant-statistic-content) {
    color: #888;
  }
  /* 工具栏小号 Select（原版 size="small"） */
  :global(.logs-toolbar .ant-select-selector) {
    height: 24px;
    font-size: 12px;
    padding: 0 7px;
  }
</style>
