<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Spinner from './Spinner.svelte';

  interface Props extends HTMLButtonAttributes {
    variant?: 'primary' | 'secondary' | 'ghost' | 'quiet' | 'danger';
    size?: 'sm' | 'md' | 'lg';
    loading?: boolean;
    full?: boolean;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    icon?: Component<any>;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    trailing?: Component<any>;
    children?: Snippet;
  }

  let {
    variant = 'secondary',
    size = 'md',
    loading = false,
    full = false,
    icon,
    trailing,
    children,
    class: cls = '',
    type = 'button',
    disabled,
    ...rest
  }: Props = $props();

  const iconSize = $derived(size === 'lg' ? 17 : size === 'sm' ? 14 : 15);
</script>

<button {type} class="btn {variant} {size} {cls}" class:full class:icon-only={!children} disabled={disabled || loading} aria-busy={loading || undefined} {...rest}>
  {#if loading}
    <Spinner size={iconSize} />
  {:else if icon}
    {@const Icon = icon}
    <Icon size={iconSize} strokeWidth={1.75} aria-hidden="true" />
  {/if}
  {#if children}<span class="label">{@render children()}</span>{/if}
  {#if trailing}
    {@const Trail = trailing}
    <Trail size={iconSize - 1} strokeWidth={1.75} aria-hidden="true" class="trail" />
  {/if}
</button>

<style>
  .btn {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    height: 32px;
    padding: 0 13px;
    border-radius: var(--r-sm);
    font-size: 13px;
    font-weight: 560;
    letter-spacing: -0.005em;
    white-space: nowrap;
    flex: none;
    transition:
      background-color var(--dur-1) var(--ease),
      box-shadow var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease),
      transform 140ms var(--ease),
      filter var(--dur-1) var(--ease),
      opacity var(--dur-1) var(--ease);
  }
  .btn:active:not(:disabled) {
    transform: scale(0.965);
  }
  .btn:disabled {
    opacity: 0.45;
  }
  .full {
    width: 100%;
  }
  .label {
    display: inline-flex;
    align-items: center;
  }
  .sm {
    height: 28px;
    padding: 0 10px;
    font-size: 12.5px;
    border-radius: 7px;
    gap: 6px;
  }
  .lg {
    height: 40px;
    padding: 0 18px;
    font-size: 14px;
    border-radius: 11px;
    gap: 8px;
  }
  .icon-only {
    padding: 0;
    width: 32px;
  }
  .icon-only.sm {
    width: 28px;
  }
  .icon-only.lg {
    width: 40px;
  }

  .primary {
    color: var(--accent-contrast);
    background-color: var(--accent);
    background-image: linear-gradient(180deg, rgb(255 255 255 / 0.18), rgb(255 255 255 / 0) 60%);
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.38),
      inset 0 -1px 0 rgb(0 0 0 / 0.12),
      0 1px 2px rgb(0 0 0 / 0.28),
      0 6px 18px -8px rgb(var(--accent-rgb) / 0.6);
  }
  .primary:hover:not(:disabled) {
    filter: brightness(1.06) saturate(1.04);
  }

  .secondary {
    color: var(--text-1);
    background: var(--surface-3);
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      var(--highlight);
  }
  .secondary:hover:not(:disabled) {
    background: var(--surface-4);
  }
  :global([data-theme='light']) .secondary {
    background: var(--surface-1);
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      0 1px 2px rgb(60 40 10 / 0.06);
  }
  :global([data-theme='light']) .secondary:hover:not(:disabled) {
    background: var(--surface-2);
  }

  .ghost {
    color: var(--text-2);
  }
  .ghost:hover:not(:disabled) {
    color: var(--text-1);
    background: var(--fill-2);
  }

  .quiet {
    color: var(--text-1);
    background: var(--fill-2);
  }
  .quiet:hover:not(:disabled) {
    background: var(--fill-3);
  }

  .danger {
    color: var(--red);
    background: rgb(var(--red-rgb) / 0.1);
    box-shadow: inset 0 0 0 1px rgb(var(--red-rgb) / 0.22);
  }
  .danger:hover:not(:disabled) {
    background: rgb(var(--red-rgb) / 0.16);
  }

  .btn :global(.trail) {
    margin-right: -3px;
    opacity: 0.7;
  }
</style>
