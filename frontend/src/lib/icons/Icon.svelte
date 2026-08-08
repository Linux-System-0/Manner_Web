<script lang="ts">
  // 图标组件：name → antd outlined path（数据来自 @ant-design/icons-svg，见 icon-data.ts）
  import { iconData } from './icon-data'

  let {
    name,
    style = '',
    size = '1em',
    class: className = '',
  }: {
    name: string
    style?: string
    size?: string | number
    class?: string
  } = $props()

  let data = $derived(iconData[name])
  let sizeStr = $derived(typeof size === 'number' ? `${size}px` : size)
  let nodes = $derived(
    (data?.paths ?? []).map((p) => {
      if (p.startsWith('circle:')) {
        const attrs = JSON.parse(p.slice(7)) as Record<string, string | number>
        return { kind: 'circle' as const, attrs }
      }
      return { kind: 'path' as const, d: p }
    }),
  )
</script>

{#if data}
  <span
    class={`anticon ${name === 'loading' ? 'anticon-spin' : ''} ${className}`.trim()}
    style={`font-size:${sizeStr};${style}`}
    role="img"
    aria-label={name}
  >
    <svg
      viewBox={data.viewBox}
      width="1em"
      height="1em"
      fill="currentColor"
      focusable="false"
      aria-hidden="true"
    >
      {#each nodes as node}
        {#if node.kind === 'path'}
          <path d={node.d} />
        {:else}
          <circle {...node.attrs} />
        {/if}
      {/each}
    </svg>
  </span>
{/if}

<style>
  .anticon {
    display: inline-flex;
    align-items: center;
    color: inherit;
    line-height: 0;
    vertical-align: -0.125em;
  }
  .anticon-spin {
    animation: anticon-spin 1s linear infinite;
  }
  :global {
    @keyframes anticon-spin {
      to {
        transform: rotate(360deg);
      }
    }
  }
</style>
