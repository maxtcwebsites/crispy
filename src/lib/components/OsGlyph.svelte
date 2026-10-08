<script lang="ts">
  // Abstract platform marks (not trademarked logos): ⌘ for macOS, a four-pane window for
  // Windows, a terminal prompt for Linux.
  import type { Os } from '../types';

  interface Props {
    os: Os;
    size?: number;
    stroke?: number;
  }
  let { os, size = 16, stroke = 1.5 }: Props = $props();
</script>

<svg
  viewBox="0 0 24 24"
  width={size}
  height={size}
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if os === 'macos'}
    <path d="M15 6.5v11a2.75 2.75 0 1 0 2.75-2.75H6.25A2.75 2.75 0 1 0 9 17.5v-11a2.75 2.75 0 1 0-2.75 2.75h11.5A2.75 2.75 0 1 0 15 6.5Z" />
  {:else if os === 'windows'}
    <rect x="4" y="4" width="7" height="7" rx="1.6" />
    <rect x="13" y="4" width="7" height="7" rx="1.6" />
    <rect x="4" y="13" width="7" height="7" rx="1.6" />
    <rect x="13" y="13" width="7" height="7" rx="1.6" />
  {:else}
    <rect x="3" y="4.5" width="18" height="15" rx="3.2" />
    <path d="m7.5 10 2.75 2.25L7.5 14.5M13 15h3.5" />
  {/if}
</svg>
