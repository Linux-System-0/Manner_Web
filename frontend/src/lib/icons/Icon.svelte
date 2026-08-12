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
