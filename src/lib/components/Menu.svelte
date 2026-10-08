<script lang="ts" module>
  import type { Component } from 'svelte';
  export type MenuItem =
    | {
        kind?: 'action';
        label: string;
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        icon?: Component<any>;
        danger?: boolean;
        disabled?: boolean;
        onselect: () => void;
      }
    | { kind: 'separator' }
    | { kind: 'info'; label: string; value: string; mono?: boolean; onselect?: () => void };
</script>

<script lang="ts">
  import Ellipsis from '@lucide/svelte/icons/ellipsis';
  import { outside, portal, tooltip } from '../actions';
  import { rise } from '../motion';
  import { placeBelow } from '../popover';

  interface Props {
    items: MenuItem[];
    label: string;
    align?: 'start' | 'end';
    size?: number;
  }
  let { items, label, align = 'end', size = 28 }: Props = $props();

  let open = $state(false);
  let trigger: HTMLButtonElement | undefined = $state();
  let menu: HTMLDivElement | undefined = $state();
  let pos = $state({ left: 0, top: 0, up: false });

  function show() {
    if (!trigger) return;
    const r = trigger.getBoundingClientRect();
    const h = items.reduce((a, it) => a + (it.kind === 'separator' ? 9 : it.kind === 'info' ? 48 : 32), 10);
    pos = placeBelow(r, { width: 236, height: h }, align, 6);
    open = true;
    requestAnimationFrame(() => focusables()[0]?.focus());
  }
  function hide(refocus = false) {
    open = false;
    if (refocus) trigger?.focus();
  }
  function focusables(): HTMLElement[] {
    return menu ? Array.from(menu.querySelectorAll<HTMLElement>('[role="menuitem"]:not([aria-disabled="true"])')) : [];
  }
  function keydown(e: KeyboardEvent) {
    const list = focusables();
    const i = list.indexOf(document.activeElement as HTMLElement);
    if (e.key === 'ArrowDown') list[(i + 1) % list.length]?.focus();
    else if (e.key === 'ArrowUp') list[(i - 1 + list.length) % list.length]?.focus();
    else if (e.key === 'Home') list[0]?.focus();
    else if (e.key === 'End') list[list.length - 1]?.focus();
    else if (e.key === 'Escape') {
      e.stopPropagation();
      hide(true);
    } else if (e.key === 'Tab') hide();
    else return;
    e.preventDefault();
  }
  function select(it: MenuItem) {
    if (it.kind === 'separator') return;
    if (it.kind !== 'info' && it.disabled) return;
    hide(true);
    it.onselect?.();
  }
</script>

<button
  bind:this={trigger}
  type="button"
  class="trigger"
  class:open
  style:width="{size}px"
  style:height="{size}px"
  aria-label={label}
  aria-haspopup="menu"
  aria-expanded={open}
  use:tooltip={open ? null : label}
  onclick={() => (open ? hide() : show())}
>
  <Ellipsis size={16} strokeWidth={1.75} aria-hidden="true" />
</button>

{#if open}
  <div
    bind:this={menu}
    class="menu"
    role="menu"
    tabindex="-1"
    aria-label={label}
    style:left="{pos.left}px"
    style:top="{pos.top}px"
    use:portal
    use:outside={{ onoutside: () => hide(), ignore: trigger }}
    onkeydown={keydown}
    transition:rise={{ y: pos.up ? 4 : -4, scale: 0.97, duration: 180 }}
  >
    {#each items as it, i (i)}
      {#if it.kind === 'separator'}
        <div class="sep" role="separator"></div>
      {:else if it.kind === 'info'}
        <div class="info" role="menuitem" tabindex="-1" onclick={() => select(it)} onkeydown={(e) => e.key === 'Enter' && select(it)}>
          <span class="info-label">{it.label}</span>
          <span class="info-value" class:mono={it.mono}>{it.value}</span>
        </div>
      {:else}
        {@const Icon = it.icon}
        <div
          class="item"
          class:danger={it.danger}
          role="menuitem"
          tabindex="-1"
          aria-disabled={it.disabled || undefined}
          onclick={() => select(it)}
          onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), select(it))}
        >
          {#if Icon}<Icon size={15} strokeWidth={1.6} aria-hidden="true" />{/if}
          <span>{it.label}</span>
        </div>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .trigger {
    display: inline-grid;
    place-items: center;
    border-radius: var(--r-sm);
    color: var(--text-3);
    flex: none;
    transition:
      background var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease);
  }
  .trigger:hover,
  .trigger.open {
    background: var(--fill-2);
    color: var(--text-1);
  }
  .menu {
    position: fixed;
    z-index: 900;
    width: 236px;
    padding: 5px;
    border-radius: 12px;
    background: var(--surface-float);
    box-shadow: var(--shadow-float);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 10px;
    border-radius: 7px;
    font-size: 13px;
    color: var(--text-1);
  }
  .item :global(svg) {
    color: var(--text-2);
    flex: none;
  }
  .item:hover,
  .item:focus-visible {
    background: var(--fill-3);
    box-shadow: none;
  }
  .item.danger,
  .item.danger :global(svg) {
    color: var(--red);
  }
  .item.danger:hover,
  .item.danger:focus-visible {
    background: rgb(var(--red-rgb) / 0.12);
  }
  .item[aria-disabled='true'] {
    opacity: 0.4;
  }
  .item[aria-disabled='true']:hover {
    background: none;
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--border);
  }
  .info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 10px 7px;
    border-radius: 7px;
  }
  .info:hover,
  .info:focus-visible {
    background: var(--fill-2);
    box-shadow: none;
  }
  .info-label {
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .info-value {
    font-size: 12px;
    color: var(--text-2);
  }
</style>
