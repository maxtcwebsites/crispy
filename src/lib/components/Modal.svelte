<script lang="ts">
  import type { Snippet } from 'svelte';
  import X from '@lucide/svelte/icons/x';
  import { portal, trapFocus } from '../actions';
  import { fadeQuick, rise } from '../motion';

  interface Props {
    open?: boolean;
    labelledby?: string;
    describedby?: string;
    width?: number;
    /** Esc / backdrop / close button allowed. */
    closable?: boolean;
    showClose?: boolean;
    onclose?: () => void;
    initialFocus?: string;
    children: Snippet;
  }
  let {
    open = true,
    labelledby,
    describedby,
    width = 440,
    closable = true,
    showClose = true,
    onclose,
    initialFocus,
    children,
  }: Props = $props();

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && closable) {
      e.stopPropagation();
      onclose?.();
    }
  }
</script>

{#if open}
  <div class="layer" use:portal>
    <div class="scrim" transition:fadeQuick={{ duration: 200 }} onclick={() => closable && onclose?.()} aria-hidden="true"></div>
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby={labelledby}
      aria-describedby={describedby}
      tabindex="-1"
      style:width="min({width}px, calc(100vw - 48px))"
      use:trapFocus={{ initial: initialFocus }}
      onkeydown={keydown}
      transition:rise={{ y: 12, scale: 0.965, duration: 260 }}
    >
      {#if closable && showClose}
        <button type="button" class="close" aria-label="Close" onclick={() => onclose?.()}>
          <X size={15} strokeWidth={1.75} aria-hidden="true" />
        </button>
      {/if}
      {@render children()}
    </div>
  </div>
{/if}

<style>
  .layer {
    position: fixed;
    inset: 0;
    z-index: 800;
    display: grid;
    place-items: center;
    padding: 24px;
  }
  .scrim {
    position: absolute;
    inset: 0;
    background: var(--scrim);
    -webkit-backdrop-filter: blur(6px) saturate(120%);
    backdrop-filter: blur(6px) saturate(120%);
  }
  .dialog {
    position: relative;
    max-height: calc(100vh - 48px);
    overflow: auto;
    border-radius: var(--r-2xl);
    background: var(--surface-float);
    box-shadow:
      var(--shadow-float),
      var(--highlight-strong);
  }
  :global([data-theme='dark']) .dialog {
    background: linear-gradient(180deg, #23211e, #1c1b19);
  }
  .dialog:focus,
  .dialog:focus-visible {
    border-radius: var(--r-2xl);
    box-shadow:
      var(--shadow-float),
      var(--highlight-strong);
  }
  .close {
    position: absolute;
    top: 14px;
    right: 14px;
    z-index: 2;
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    color: var(--text-3);
    transition:
      background var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease);
  }
  .close:hover {
    background: var(--fill-2);
    color: var(--text-1);
  }
</style>
