<script lang="ts">
  import { onMount } from 'svelte';
  import { isMock } from './lib/api';
  import { store, type Page } from './lib/store.svelte';
  import { rise } from './lib/motion';
  import Sidebar from './lib/components/Sidebar.svelte';
  import WindowControls from './lib/components/WindowControls.svelte';
  import MockTrafficLights from './lib/components/MockTrafficLights.svelte';
  import Notices from './lib/components/Notices.svelte';
  import PairingModal from './lib/components/PairingModal.svelte';
  import SendPicker from './lib/components/SendPicker.svelte';
  import AddByIp from './lib/components/AddByIp.svelte';
  import ConfirmDialog from './lib/components/ConfirmDialog.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import ChipMark from './lib/components/ChipMark.svelte';
  import Devices from './lib/pages/Devices.svelte';
  import Arrangement from './lib/pages/Arrangement.svelte';
  import Transfers from './lib/pages/Transfers.svelte';
  import Settings from './lib/pages/Settings.svelte';
  import Onboarding from './lib/pages/Onboarding.svelte';

  const PAGES: Page[] = ['devices', 'arrangement', 'transfers', 'settings'];

  onMount(() => {
    void store.init();

    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const syncScheme = () => (store.systemDark = mq.matches);
    syncScheme();
    mq.addEventListener('change', syncScheme);

    const focus = () => (store.windowFocused = true);
    const blur = () => (store.windowFocused = false);
    window.addEventListener('focus', focus);
    window.addEventListener('blur', blur);

    // ⌘1–4 / Ctrl+1–4 switch pages, ⌘, opens settings.
    const keydown = (e: KeyboardEvent) => {
      const mod = store.os === 'macos' ? e.metaKey : e.ctrlKey;
      if (!mod || e.altKey || e.shiftKey || !store.settings?.general.onboarded) return;
      if (document.querySelector('[aria-modal="true"]')) return;
      const n = Number(e.key);
      if (n >= 1 && n <= PAGES.length) {
        e.preventDefault();
        store.go(PAGES[n - 1]);
      } else if (e.key === ',') {
        e.preventDefault();
        store.go('settings');
      }
    };
    window.addEventListener('keydown', keydown);

    // Desktop app: no browser context menu (except in text fields).
    const contextmenu = (e: MouseEvent) => {
      const t = e.target as HTMLElement;
      if (!isMock && !t.closest('input, textarea, .selectable')) e.preventDefault();
    };
    window.addEventListener('contextmenu', contextmenu);

    return () => {
      mq.removeEventListener('change', syncScheme);
      window.removeEventListener('focus', focus);
      window.removeEventListener('blur', blur);
      window.removeEventListener('keydown', keydown);
      window.removeEventListener('contextmenu', contextmenu);
    };
  });

  // Theme, flavor, motion and window material live on <html> so every token follows.
  $effect(() => {
    const root = document.documentElement;
    root.dataset.theme = store.theme;
    root.dataset.flavor = store.settings?.general.flavor ?? 'classic';
    root.dataset.reduceMotion = String(store.reduceMotion);
    root.dataset.translucent = String(store.translucent);
    root.dataset.os = store.os;
    // Browser preview of the native window material (?effects=1): a stand-in desktop wallpaper.
    if (isMock) root.classList.toggle('mock-wallpaper', store.translucent);
    // Remembered per machine so the next launch paints the right palette before settings load.
    if (store.settings) {
      try {
        localStorage.setItem('crispy.look', JSON.stringify({ theme: store.settings.general.theme, flavor: store.settings.general.flavor }));
      } catch {
        /* storage unavailable */
      }
    }
  });

  // Autosave: any change to the settings object is debounced and sent to the backend.
  $effect(() => {
    if (store.settings) store.settingsChanged(JSON.stringify(store.settings));
  });

  const ready = $derived(!!store.state && !!store.settings && !!store.info);
  const onboarding = $derived(ready && !store.settings!.general.onboarded);

  let scroller: HTMLDivElement | undefined = $state();
  $effect(() => {
    void store.page;
    scroller?.scrollTo({ top: 0 });
  });
</script>

{#if !ready}
  <div class="splash" data-tauri-drag-region>
    <div class="splash-mark"><ChipMark size={72} label="Crispy" /></div>
    {#if store.loadError}
      <p class="splash-error">Crispy couldn’t start its engine.<br /><span class="mono">{store.loadError}</span></p>
    {/if}
  </div>
{:else if onboarding}
  <Onboarding />
{:else}
  <div class="app">
    <Sidebar />
    <main class="panel" class:fill={store.page === 'arrangement'}>
      <div class="dragstrip" data-tauri-drag-region></div>
      <div class="scroll" bind:this={scroller}>
        {#key store.page}
          <div class="page page-{store.page}" in:rise={{ y: 8, duration: 260 }}>
            <Notices />
            {#if store.page === 'devices'}
              <Devices />
            {:else if store.page === 'arrangement'}
              <Arrangement />
            {:else if store.page === 'transfers'}
              <Transfers />
            {:else}
              <Settings />
            {/if}
          </div>
        {/key}
      </div>
    </main>
  </div>
  <SendPicker />
  <AddByIp />
{/if}

{#if ready}<PairingModal />{/if}

<ConfirmDialog />
<Toasts />

{#if store.info?.os === 'windows'}
  <div class="win-titlebar" data-tauri-drag-region></div>
  <WindowControls />
{:else if isMock && store.os === 'macos'}
  <MockTrafficLights />
{/if}

<style>
  .app {
    display: flex;
    height: 100%;
    min-width: 880px;
    min-height: 600px;
  }
  .panel {
    position: relative;
    flex: 1;
    min-width: 0;
    margin: 8px 8px 8px 0;
    border-radius: 14px;
    overflow: hidden;
    background:
      radial-gradient(760px 420px at 100% -6%, rgb(var(--accent-rgb) / var(--glow-strength)), transparent 72%),
      var(--bg-panel);
    box-shadow: var(--shadow-panel);
    isolation: isolate;
  }
  :global([data-os='windows']) .panel {
    margin-top: 32px;
  }
  .dragstrip {
    position: absolute;
    inset: 0 0 auto 0;
    height: 26px;
    z-index: 5;
  }
  .scroll {
    height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-gutter: stable;
  }
  .fill .scroll {
    overflow: hidden;
  }
  .page {
    max-width: 1120px;
    margin: 0 auto;
    padding: 40px 40px 48px;
  }
  .page-arrangement {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding-bottom: 28px;
  }
  .page-settings {
    padding-top: 40px;
  }
  @media (max-width: 1000px) {
    .page {
      padding: 34px 28px 40px;
    }
  }
  @media (max-height: 680px) {
    .page {
      padding-top: 28px;
    }
    .page-arrangement {
      padding-bottom: 20px;
    }
  }

  .win-titlebar {
    position: fixed;
    inset: 0 0 auto 0;
    height: 32px;
    z-index: 1;
  }

  .splash {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 22px;
    background: var(--bg-canvas);
  }
  .splash-mark {
    animation: breathe 2.4s var(--ease-in-out) infinite;
  }
  .splash-error {
    text-align: center;
    color: var(--text-2);
    line-height: 1.6;
  }
  .splash-error .mono {
    font-size: 12px;
    color: var(--text-3);
  }
  @keyframes breathe {
    0%,
    100% {
      transform: scale(1) rotate(0deg);
      opacity: 0.85;
    }
    50% {
      transform: scale(1.05) rotate(-3deg);
      opacity: 1;
    }
  }
</style>
