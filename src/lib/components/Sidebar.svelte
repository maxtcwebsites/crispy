<script lang="ts">
  import MonitorSmartphone from '@lucide/svelte/icons/monitor-smartphone';
  import PanelsTopLeft from '@lucide/svelte/icons/panels-top-left';
  import ArrowDownUp from '@lucide/svelte/icons/arrow-down-up';
  import Settings2 from '@lucide/svelte/icons/settings-2';
  import Lock from '@lucide/svelte/icons/lock';
  import LockOpen from '@lucide/svelte/icons/lock-open';
  import Pause from '@lucide/svelte/icons/pause';
  import Play from '@lucide/svelte/icons/play';
  import { api } from '../api';
  import { tooltip } from '../actions';
  import { store, type Page } from '../store.svelte';
  import { formatBytes, formatUptime, hotkeyParts } from '../util';
  import ChipMark from './ChipMark.svelte';
  import OsGlyph from './OsGlyph.svelte';
  import StatusDot from './StatusDot.svelte';

  const nav: { id: Page; label: string; icon: typeof MonitorSmartphone }[] = [
    { id: 'devices', label: 'Devices', icon: MonitorSmartphone },
    { id: 'arrangement', label: 'Arrangement', icon: PanelsTopLeft },
    { id: 'transfers', label: 'Transfers', icon: ArrowDownUp },
    { id: 'settings', label: 'Settings', icon: Settings2 },
  ];

  const mod = $derived(store.os === 'macos' ? '⌘' : 'Ctrl');
  const s = $derived(store.state);
  const me = $derived(store.me);

  const status = $derived.by(() => {
    if (!s) return { text: '', tone: 'grey' as const, pulse: false };
    if (store.controlledBy) return { text: `Controlled by ${store.controlledBy.name}`, tone: 'accent' as const, pulse: true };
    if (s.paused) return { text: 'Paused', tone: 'amber' as const, pulse: false };
    if (s.locked) return { text: 'Locked to this computer', tone: 'amber' as const, pulse: false };
    if (store.focusPeer) return { text: `On ${store.focusPeer.name}`, tone: 'accent' as const, pulse: true };
    if (!store.connected.length) return { text: 'Ready', tone: 'green' as const, pulse: false };
    return { text: 'Sharing', tone: 'green' as const, pulse: false };
  });

  const lockKeys = $derived(hotkeyParts(store.settings?.hotkeys.lockToScreen, store.keys, store.os));
  const pauseKeys = $derived(hotkeyParts(store.settings?.hotkeys.pauseSharing, store.keys, store.os));
</script>

<aside class="sidebar" class:mac={store.os === 'macos'}>
  <div class="top" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <ChipMark size={30} />
      <span class="word serif" data-tauri-drag-region>Crispy</span>
    </div>
  </div>

  <nav aria-label="Main">
    {#each nav as item, i (item.id)}
      {@const Icon = item.icon}
      <button
        type="button"
        class="item"
        class:active={store.page === item.id}
        aria-current={store.page === item.id ? 'page' : undefined}
        onclick={() => store.go(item.id)}
        use:tooltip={{ text: item.label, keys: [mod, String(i + 1)], placement: 'right' }}
      >
        <Icon size={17} strokeWidth={1.6} aria-hidden="true" />
        <span class="label">{item.label}</span>
        {#if item.id === 'transfers' && store.activeTransferCount > 0}
          <span class="count tnum" aria-label="{store.activeTransferCount} active">{store.activeTransferCount}</span>
        {/if}
      </button>
    {/each}
  </nav>

  <div class="spacer" data-tauri-drag-region></div>

  {#if store.settings?.advanced.showStats && s}
    <div class="stats mono tnum" aria-label="Statistics">
      <span use:tooltip={'Input events per second'}>{s.stats.eventsPerSec} ev/s</span>
      <span use:tooltip={'Sent'}>↑ {formatBytes(s.stats.bytesSent)}</span>
      <span use:tooltip={'Received'}>↓ {formatBytes(s.stats.bytesReceived)}</span>
      <span use:tooltip={'Uptime'}>{formatUptime(s.stats.uptimeSecs)}</span>
    </div>
  {/if}

  {#if me && s}
    <div class="me" class:controlled={!!store.controlledBy}>
      <div class="me-head">
        <span class="glyph"><OsGlyph os={me.os} size={15} /></span>
        <div class="me-text">
          <p class="name truncate">{me.name}</p>
          <p class="status">
            <StatusDot tone={status.tone} pulse={status.pulse} size={6} />
            <span class="truncate">{status.text}</span>
          </p>
        </div>
      </div>
      <div class="quick">
        <button
          type="button"
          class="q"
          class:on={s.locked}
          aria-pressed={s.locked}
          onclick={() => store.run(() => api.toggleLock())}
          use:tooltip={{ text: s.locked ? 'Unlock the cursor' : 'Keep the cursor on this computer', keys: lockKeys }}
        >
          {#if s.locked}<Lock size={14} strokeWidth={1.75} aria-hidden="true" />{:else}<LockOpen size={14} strokeWidth={1.75} aria-hidden="true" />{/if}
          <span>{s.locked ? 'Locked' : 'Lock'}</span>
        </button>
        <button
          type="button"
          class="q"
          class:on={s.paused}
          aria-pressed={s.paused}
          onclick={() => store.run(() => api.togglePause())}
          use:tooltip={{ text: s.paused ? 'Resume sharing' : 'Pause sharing', keys: pauseKeys }}
        >
          {#if s.paused}<Play size={14} strokeWidth={1.75} aria-hidden="true" />{:else}<Pause size={14} strokeWidth={1.75} aria-hidden="true" />{/if}
          <span>{s.paused ? 'Resume' : 'Pause'}</span>
        </button>
      </div>
    </div>
  {/if}
</aside>

<style>
  .sidebar {
    position: relative;
    display: flex;
    flex-direction: column;
    width: var(--sidebar-w);
    height: 100%;
    padding: 0 12px 12px;
    flex: none;
  }
  .top {
    height: 64px;
    display: flex;
    align-items: flex-end;
    padding: 0 6px 12px;
  }
  .mac .top {
    height: 88px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
  }
  .word {
    font-size: 25px;
    line-height: 1;
    letter-spacing: -0.01em;
    color: var(--text-1);
    padding-top: 2px;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: 6px;
  }
  .item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 11px;
    height: 34px;
    padding: 0 10px;
    border-radius: 9px;
    font-size: 13.5px;
    font-weight: 520;
    color: var(--text-2);
    transition:
      background var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease);
  }
  .item :global(svg) {
    color: var(--text-3);
    transition: color var(--dur-1) var(--ease);
  }
  .item:hover {
    background: var(--fill-1);
    color: var(--text-1);
  }
  .item.active {
    color: var(--text-1);
    background: var(--fill-2);
    box-shadow:
      inset 0 0 0 1px var(--border),
      var(--highlight);
  }
  :global([data-theme='light']) .item.active {
    background: rgb(255 255 255 / 0.75);
    box-shadow:
      0 0 0 1px var(--border),
      0 1px 2px rgb(60 40 10 / 0.06);
  }
  .item.active :global(svg) {
    color: var(--accent-ink);
  }
  .item.active::before {
    content: '';
    position: absolute;
    left: -1px;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: 3px;
    background: var(--accent);
    box-shadow: 0 0 10px rgb(var(--accent-rgb) / 0.6);
  }
  .label {
    flex: 1;
    text-align: left;
  }
  .count {
    min-width: 20px;
    height: 18px;
    padding: 0 6px;
    display: inline-grid;
    place-items: center;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 650;
    color: var(--accent-ink);
    background: var(--accent-soft);
  }

  .spacer {
    flex: 1;
  }

  .stats {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 3px 10px;
    padding: 0 8px 10px;
    font-size: 10.5px;
    color: var(--text-3);
  }

  .me {
    padding: 10px;
    border-radius: 14px;
    background: var(--fill-1);
    box-shadow:
      inset 0 0 0 1px var(--border),
      var(--highlight);
    transition: box-shadow var(--dur-3) var(--ease);
  }
  :global([data-theme='light']) .me {
    background: rgb(255 255 255 / 0.6);
    box-shadow:
      0 0 0 1px var(--border),
      0 1px 2px rgb(60 40 10 / 0.05);
  }
  .me.controlled {
    box-shadow:
      inset 0 0 0 1px rgb(var(--accent-rgb) / 0.45),
      0 0 24px -8px rgb(var(--accent-rgb) / 0.5);
  }
  .me-head {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 2px 2px 10px;
  }
  .glyph {
    width: 30px;
    height: 30px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 9px;
    background: linear-gradient(180deg, var(--surface-3), var(--surface-2));
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      var(--highlight-strong);
  }
  :global([data-theme='light']) .glyph {
    background: linear-gradient(180deg, #fff, #f4f1eb);
  }
  .me-text {
    min-width: 0;
    flex: 1;
  }
  .name {
    font-size: 13px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    color: var(--text-2);
    min-width: 0;
  }
  .quick {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }
  .q {
    height: 30px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border-radius: 8px;
    font-size: 12px;
    font-weight: 560;
    color: var(--text-2);
    background: var(--fill-2);
    transition:
      background var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease),
      transform 140ms var(--ease);
  }
  :global([data-theme='light']) .q {
    background: rgb(40 30 18 / 0.05);
  }
  .q:hover {
    color: var(--text-1);
    background: var(--fill-3);
  }
  .q:active {
    transform: scale(0.96);
  }
  .q.on {
    color: var(--accent-ink);
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px rgb(var(--accent-rgb) / 0.3);
  }
</style>
