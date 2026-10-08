<script lang="ts">
  import { flip } from 'svelte/animate';
  import CircleCheck from '@lucide/svelte/icons/circle-check';
  import Info from '@lucide/svelte/icons/info';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import CircleX from '@lucide/svelte/icons/circle-x';
  import X from '@lucide/svelte/icons/x';
  import { store } from '../store.svelte';
  import { reduced, rise } from '../motion';

  const icons = { success: CircleCheck, info: Info, warn: TriangleAlert, error: CircleX };
</script>

<section class="stack" aria-live="polite" aria-label="Notifications">
  {#each store.toasts as t (t.id)}
    {@const Icon = icons[t.kind]}
    <div
      class="toast {t.kind}"
      role={t.kind === 'error' ? 'alert' : 'status'}
      animate:flip={{ duration: reduced() ? 1 : 240 }}
      in:rise={{ y: 14, scale: 0.97, duration: 280 }}
      out:rise={{ x: 24, duration: 200 }}
    >
      <span class="icon"><Icon size={17} strokeWidth={1.75} aria-hidden="true" /></span>
      <div class="text">
        <p class="title">{t.title}</p>
        {#if t.body}<p class="body">{t.body}</p>{/if}
      </div>
      <button type="button" class="close" aria-label="Dismiss" onclick={() => store.dismissToast(t.id)}>
        <X size={13} strokeWidth={2} aria-hidden="true" />
      </button>
    </div>
  {/each}
</section>

<style>
  .stack {
    position: fixed;
    right: 20px;
    bottom: 20px;
    z-index: 950;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 8px;
    pointer-events: none;
    width: 340px;
    max-width: calc(100vw - 40px);
  }
  .toast {
    pointer-events: auto;
    width: 100%;
    display: flex;
    align-items: flex-start;
    gap: 11px;
    padding: 12px 12px 12px 13px;
    border-radius: 14px;
    background: var(--surface-float);
    box-shadow:
      var(--shadow-float),
      var(--highlight-strong);
    -webkit-backdrop-filter: blur(16px);
    backdrop-filter: blur(16px);
  }
  :global([data-theme='dark']) .toast {
    background: rgb(36 34 31 / 0.94);
  }
  .icon {
    flex: none;
    margin-top: 1px;
    color: var(--text-2);
  }
  .success .icon {
    color: var(--green);
  }
  .warn .icon {
    color: var(--amber);
  }
  .error .icon {
    color: var(--red);
  }
  .info .icon {
    color: var(--accent-ink);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-size: 13px;
    font-weight: 600;
    letter-spacing: -0.006em;
  }
  .body {
    margin-top: 2px;
    font-size: 12.5px;
    color: var(--text-2);
    overflow-wrap: anywhere;
  }
  .close {
    flex: none;
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border-radius: 6px;
    color: var(--text-3);
    margin: -2px -2px 0 0;
    opacity: 0;
    transition:
      opacity var(--dur-1) var(--ease),
      background var(--dur-1) var(--ease);
  }
  .toast:hover .close,
  .close:focus-visible {
    opacity: 1;
  }
  .close:hover {
    background: var(--fill-2);
    color: var(--text-1);
  }
</style>
