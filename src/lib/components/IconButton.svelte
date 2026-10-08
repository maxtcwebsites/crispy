<script lang="ts">
  import type { Component } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import { tooltip, type TooltipOptions } from '../actions';

  interface Props extends HTMLButtonAttributes {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    icon: Component<any>;
    label: string;
    tip?: TooltipOptions | string | null;
    size?: number;
    active?: boolean;
    subtle?: boolean;
  }
  let { icon: Icon, label, tip, size = 30, active = false, subtle = false, class: cls = '', ...rest }: Props = $props();
</script>

<button
  type="button"
  class="ib {cls}"
  class:active
  class:subtle
  style:width="{size}px"
  style:height="{size}px"
  aria-label={label}
  use:tooltip={tip === null ? null : (tip ?? label)}
  {...rest}
>
  <Icon size={Math.round(size * 0.53)} strokeWidth={1.6} aria-hidden="true" />
</button>

<style>
  .ib {
    display: inline-grid;
    place-items: center;
    border-radius: var(--r-sm);
    color: var(--text-2);
    flex: none;
    transition:
      background var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease),
      transform 140ms var(--ease);
  }
  .ib:hover:not(:disabled) {
    background: var(--fill-2);
    color: var(--text-1);
  }
  .ib:active:not(:disabled) {
    transform: scale(0.94);
  }
  .ib:disabled {
    opacity: 0.4;
  }
  .subtle {
    color: var(--text-3);
  }
  .active {
    color: var(--accent-ink);
    background: var(--accent-soft);
  }
  .active:hover:not(:disabled) {
    color: var(--accent-ink);
    background: rgb(var(--accent-rgb) / 0.2);
  }
</style>
