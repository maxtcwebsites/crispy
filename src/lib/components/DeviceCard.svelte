<script lang="ts">
  import Send from '@lucide/svelte/icons/send';
  import Power from '@lucide/svelte/icons/power';
  import Unlink from '@lucide/svelte/icons/unlink';
  import PencilLine from '@lucide/svelte/icons/pencil-line';
  import LayoutGrid from '@lucide/svelte/icons/layout-grid';
  import ArrowDownToLine from '@lucide/svelte/icons/arrow-down-to-line';
  import { api } from '../api';
  import { store } from '../store.svelte';
  import type { DeviceInfo, Peer } from '../types';
  import { displaysSummary, osName, statusLabel, thisDeviceLabel, timeAgo } from '../util';
  import Menu, { type MenuItem } from './Menu.svelte';
  import OsGlyph from './OsGlyph.svelte';
  import ScreensPreview from './ScreensPreview.svelte';
  import StatusDot from './StatusDot.svelte';

  interface Props {
    peer?: Peer;
    me?: DeviceInfo;
  }
  let { peer, me }: Props = $props();

  const id = $derived(peer?.id ?? me?.id ?? '');
  const name = $derived(peer?.name ?? me?.name ?? '');
  const os = $derived(peer?.os ?? me?.os ?? 'macos');
  const screens = $derived(peer?.screens ?? me?.screens ?? []);
  const isMe = $derived(!peer);
  const online = $derived(!!peer && peer.status === 'connected');
  const sendable = $derived(online && !!peer?.enabled);

  const cursorHere = $derived.by(() => {
    const s = store.state;
    if (!s) return false;
    if (isMe) return s.focus.kind === 'local' && store.connected.length > 0;
    return !!peer?.controlling;
  });
  const glowing = $derived(cursorHere || !!peer?.controllingMe);

  // Live cursor inside the miniature, while pointer events keep arriving.
  let now = $state(performance.now());
  $effect(() => {
    if (!cursorHere) return;
    const t = setInterval(() => (now = performance.now()), 500);
    return () => clearInterval(t);
  });
  const cursor = $derived.by(() => {
    const p = store.pointer;
    if (!p || p.device !== id || !cursorHere) return null;
    return now - p.t < 2500 || p.t > now ? { x: p.x, y: p.y } : null;
  });

  const tone = $derived.by((): 'green' | 'amber' | 'grey' | 'accent' => {
    if (isMe) return store.state?.paused ? 'amber' : 'green';
    if (!peer?.enabled) return 'grey';
    if (peer.status === 'connected') return 'green';
    if (peer.status === 'connecting') return 'amber';
    return 'grey';
  });

  const statusText = $derived.by(() => {
    if (isMe) {
      const s = store.state;
      if (store.controlledBy) return `Controlled by ${store.controlledBy.name}`;
      if (s?.paused) return 'Sharing paused';
      if (s?.locked) return 'Locked to this computer';
      return store.connected.length ? 'Sharing' : 'Ready';
    }
    if (!peer) return '';
    if (peer.status === 'offline' && peer.enabled) return `Offline · seen ${timeAgo(peer.lastSeen)}`;
    return statusLabel(peer);
  });

  let stageWidth = $state(260);

  const dropping = $derived(store.drag.active && sendable);
  const dropHover = $derived(dropping && store.drag.target === `peer:${id}`);

  async function copy(label: string, text: string) {
    try {
      await navigator.clipboard.writeText(text);
      store.toast({ kind: 'success', title: `${label} copied`, body: text }, 2600);
    } catch {
      /* clipboard unavailable */
    }
  }

  async function unpair() {
    if (!peer) return;
    const ok = await store.ask({
      title: `Unpair ${peer.name}?`,
      body: 'It will stop sharing with this computer right away. To connect again you will need to pair with a new code.',
      confirm: 'Unpair',
      danger: true,
    });
    if (ok) await store.run(() => api.unpair(peer.id), 'Couldn’t unpair');
  }

  const items = $derived.by((): MenuItem[] => {
    if (isMe) {
      const list: MenuItem[] = [
        { label: 'Rename…', icon: PencilLine, onselect: () => store.go('settings', 'general') },
        { label: 'Arrange displays', icon: LayoutGrid, onselect: () => store.go('arrangement') },
        { kind: 'separator' },
        { kind: 'info', label: 'Fingerprint', value: me?.fingerprint ?? '', mono: true, onselect: () => copy('Fingerprint', me?.fingerprint ?? '') },
      ];
      if (me?.addresses[0]) list.push({ kind: 'info', label: 'Address', value: me.addresses[0], mono: true, onselect: () => copy('Address', me!.addresses[0]) });
      return list;
    }
    const p = peer!;
    const list: MenuItem[] = [
      { label: 'Send files…', icon: Send, disabled: !sendable, onselect: () => store.chooseAndSend(p.id) },
      {
        label: p.enabled ? 'Pause sharing' : 'Resume sharing',
        icon: Power,
        onselect: () => store.run(() => api.setPeerEnabled(p.id, !p.enabled)),
      },
      { kind: 'separator' },
      { kind: 'info', label: 'Fingerprint', value: p.fingerprint, mono: true, onselect: () => copy('Fingerprint', p.fingerprint) },
    ];
    if (p.address) list.push({ kind: 'info', label: 'Address', value: p.address, mono: true, onselect: () => copy('Address', p.address!) });
    list.push({ kind: 'separator' }, { label: 'Unpair…', icon: Unlink, danger: true, onselect: unpair });
    return list;
  });
</script>

<article
  class="card"
  class:glowing
  class:offline={!isMe && !online}
  class:dropping
  class:drop-hover={dropHover}
  data-drop={sendable ? `peer:${id}` : undefined}
  aria-label="{name}, {statusText}"
>
  <div class="stage" bind:clientWidth={stageWidth}>
    <ScreensPreview {screens} {os} width={stageWidth} height={98} active={cursorHere} dim={!isMe && !online} {cursor} />
    <div class="badges">
      {#if isMe}
        <span class="badge">{thisDeviceLabel(os)}</span>
      {/if}
      {#if peer?.controllingMe}
        <span class="badge accent">Controlling {thisDeviceLabel(store.os).replace('This', 'this')}</span>
      {:else if cursorHere && !isMe}
        <span class="badge accent">Keyboard &amp; mouse</span>
      {/if}
      {#if peer && !peer.enabled}
        <span class="badge">Paused</span>
      {/if}
    </div>
    {#if dropping}
      <div class="drop">
        <ArrowDownToLine size={18} strokeWidth={1.75} aria-hidden="true" />
        <span>{dropHover ? `Send to ${name}` : 'Drop to send'}</span>
      </div>
    {/if}
  </div>

  <div class="info">
    <div class="glyph"><OsGlyph {os} size={17} /></div>
    <div class="names">
      <h3 class="truncate">{name}</h3>
      <p class="sub truncate">
        {osName(os)}{#if (peer?.version ?? me?.version)}<span class="sep">·</span>Crispy {peer?.version ?? me?.version}{/if}
      </p>
    </div>
    <Menu {items} label="{name} options" />
  </div>

  <div class="foot">
    <span class="status">
      <StatusDot {tone} pulse={tone === 'amber' && !isMe} />
      <span class="truncate">{statusText}</span>
    </span>
    <span class="meta tnum">
      {#if online && peer?.rttMs != null}
        <span class="rtt">{peer.rttMs} ms</span><span class="sep">·</span>
      {/if}
      <span>{displaysSummary(screens).split(' · ')[0]}</span>
    </span>
  </div>
</article>

<style>
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    border-radius: var(--r-lg);
    background: var(--surface-1);
    box-shadow:
      inset 0 0 0 1px var(--border),
      var(--highlight);
    transition:
      box-shadow var(--dur-3) var(--ease),
      transform var(--dur-2) var(--ease),
      background var(--dur-2) var(--ease);
  }
  :global([data-theme='light']) .card {
    box-shadow:
      0 0 0 1px var(--border),
      0 1px 2px rgb(60 40 10 / 0.04),
      0 8px 24px -16px rgb(60 40 10 / 0.12);
  }
  .card:hover {
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      var(--highlight);
  }
  .card.glowing {
    box-shadow:
      inset 0 0 0 1px rgb(var(--accent-rgb) / 0.5),
      var(--accent-glow),
      var(--highlight);
  }
  :global([data-theme='light']) .card.glowing {
    box-shadow:
      0 0 0 1px rgb(var(--accent-rgb) / 0.8),
      0 0 0 4px rgb(var(--accent-rgb) / 0.14),
      0 12px 32px -14px rgb(var(--accent-rgb) / 0.5);
  }
  .card.dropping {
    box-shadow:
      inset 0 0 0 1px rgb(var(--accent-rgb) / 0.45),
      var(--highlight);
  }
  .card.drop-hover {
    transform: translateY(-2px) scale(1.01);
    box-shadow:
      inset 0 0 0 1.5px var(--accent),
      var(--accent-glow);
  }

  .stage {
    position: relative;
    margin: 6px 6px 0;
    padding: 30px 0 6px;
    border-radius: 10px;
    background:
      radial-gradient(120% 100% at 50% 0%, var(--fill-1), transparent 70%),
      var(--surface-sunken);
    box-shadow: inset 0 0 0 1px var(--border);
    overflow: hidden;
  }
  :global([data-theme='light']) .stage {
    background:
      radial-gradient(120% 100% at 50% 0%, #fff, transparent 70%),
      #f3f0ea;
  }
  .glowing .stage {
    background:
      radial-gradient(120% 110% at 50% 100%, rgb(var(--accent-rgb) / 0.12), transparent 70%),
      var(--surface-sunken);
  }
  .badges {
    position: absolute;
    top: 8px;
    left: 8px;
    display: flex;
    gap: 5px;
  }
  .badge {
    height: 20px;
    padding: 0 7px;
    display: inline-flex;
    align-items: center;
    border-radius: 6px;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--text-2);
    background: var(--fill-3);
    -webkit-backdrop-filter: blur(8px);
    backdrop-filter: blur(8px);
  }
  .badge.accent {
    color: var(--accent-ink);
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px rgb(var(--accent-rgb) / 0.25);
  }
  .drop {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--accent-ink);
    background: rgb(var(--accent-rgb) / 0.08);
    -webkit-backdrop-filter: blur(3px);
    backdrop-filter: blur(3px);
    border-radius: inherit;
    box-shadow: inset 0 0 0 1.5px rgb(var(--accent-rgb) / 0.4);
  }
  .drop-hover .drop {
    background: rgb(var(--accent-rgb) / 0.16);
  }

  .info {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 14px 10px 0 14px;
  }
  .glyph {
    width: 34px;
    height: 34px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 10px;
    color: var(--text-1);
    background: linear-gradient(180deg, var(--surface-3), var(--surface-2));
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      var(--highlight-strong);
  }
  :global([data-theme='light']) .glyph {
    background: linear-gradient(180deg, #fff, #f4f1eb);
  }
  .glowing .glyph {
    color: var(--accent-ink);
  }
  .names {
    flex: 1;
    min-width: 0;
  }
  h3 {
    font-size: 14px;
    font-weight: 620;
    letter-spacing: -0.012em;
  }
  .sub {
    font-size: 12px;
    color: var(--text-3);
  }
  .sep {
    margin: 0 5px;
    color: var(--text-4);
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 12px 14px 14px;
    font-size: 12px;
    color: var(--text-2);
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }
  .meta {
    flex: none;
    color: var(--text-3);
  }
  .rtt {
    color: var(--text-2);
  }
</style>
