<script lang="ts">
  // Windows 11 style caption buttons for the undecorated window.
  import { onMount } from 'svelte';
  import { appWindow } from '../api';

  let maximized = $state(false);

  onMount(() => {
    let off: (() => void) | undefined;
    const sync = async () => (maximized = await appWindow.isMaximized());
    void sync();
    appWindow.onResized(sync).then((u) => (off = u));
    return () => off?.();
  });
</script>

<div class="controls" role="group" aria-label="Window">
  <button type="button" aria-label="Minimize" onclick={() => appWindow.minimize()}>
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M0 5.5h10" stroke="currentColor" stroke-width="1" /></svg>
  </button>
  <button type="button" aria-label={maximized ? 'Restore' : 'Maximize'} onclick={() => appWindow.toggleMaximize()}>
    {#if maximized}
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
        <rect x="0.5" y="2.5" width="7" height="7" rx="1.2" stroke="currentColor" />
        <path d="M2.5 2.5V1.7A1.2 1.2 0 0 1 3.7.5h4.6a1.2 1.2 0 0 1 1.2 1.2v4.6a1.2 1.2 0 0 1-1.2 1.2H7.5" stroke="currentColor" />
      </svg>
    {:else}
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true"><rect x="0.5" y="0.5" width="9" height="9" rx="1.4" stroke="currentColor" /></svg>
    {/if}
  </button>
  <button type="button" class="close" aria-label="Close" onclick={() => appWindow.close()}>
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M.6.6l8.8 8.8M9.4.6L.6 9.4" stroke="currentColor" stroke-width="1.05" /></svg>
  </button>
</div>

<style>
  .controls {
    position: fixed;
    top: 0;
    right: 0;
    z-index: 700;
    display: flex;
  }
  button {
    width: 46px;
    height: 32px;
    display: grid;
    place-items: center;
    color: var(--text-1);
    opacity: 0.85;
    transition:
      background-color 120ms linear,
      color 120ms linear;
  }
  button:hover {
    background: var(--fill-3);
    opacity: 1;
  }
  button:active {
    background: var(--fill-2);
  }
  .close:hover {
    background: #c42b1c;
    color: #fff;
  }
  .close:active {
    background: #b52a1c;
    color: rgb(255 255 255 / 0.8);
  }
  button:focus-visible {
    border-radius: 0;
    box-shadow: inset 0 0 0 2px rgb(var(--accent-rgb) / 0.7);
  }
</style>
