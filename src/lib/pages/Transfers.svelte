<script lang="ts">
  import { flip } from 'svelte/animate';
  import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
  import ArrowDownLeft from '@lucide/svelte/icons/arrow-down-left';
  import ArrowDownToLine from '@lucide/svelte/icons/arrow-down-to-line';
  import ClipboardIcon from '@lucide/svelte/icons/clipboard';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import X from '@lucide/svelte/icons/x';
  import Check from '@lucide/svelte/icons/check';
  import Sparkles from '@lucide/svelte/icons/sparkles';
  import { api } from '../api';
  import { reduced, rise } from '../motion';
  import { store } from '../store.svelte';
  import type { Transfer } from '../types';
  import { formatBytes, formatDuration, formatSpeed, isFinished, plural, prettyPath, timeAgo, transferEta, transferTitle } from '../util';
  import Button from '../components/Button.svelte';
  import IconButton from '../components/IconButton.svelte';
  import PageHeader from '../components/PageHeader.svelte';
  import ProgressBar from '../components/ProgressBar.svelte';

  const transfers = $derived(store.state?.transfers ?? []);
  const live = $derived(transfers.filter((t) => !isFinished(t)));
  const done = $derived(transfers.filter(isFinished).sort((a, b) => (b.finishedAt ?? 0) - (a.finishedAt ?? 0)));

  let now = $state(Date.now());
  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 20_000);
    return () => clearInterval(t);
  });

  const canSend = $derived(store.connected.length > 0);
  const dragOver = $derived(store.drag.active && store.drag.target === 'any');
  const saveDir = $derived(prettyPath(store.settings?.files.saveDir || store.info?.saveDir || ''));

  function detail(t: Transfer): string {
    const to = t.direction === 'send' ? `To ${t.peerName}` : `From ${t.peerName}`;
    const size = formatBytes(t.totalBytes);
    switch (t.state) {
      case 'pending':
        return `${to} · Waiting to start…`;
      case 'awaiting':
        return t.direction === 'receive'
          ? `${t.peerName} wants to send you ${plural(t.fileCount, 'file')} · ${size}`
          : `Waiting for ${t.peerName} to accept · ${size}`;
      case 'active': {
        const eta = transferEta(t);
        return [to, `${formatBytes(t.doneBytes)} of ${size}`, formatSpeed(t.bytesPerSec), eta != null ? `${formatDuration(eta)} left` : null]
          .filter(Boolean)
          .join(' · ');
      }
      case 'done':
        return `${t.direction === 'send' ? `Sent to ${t.peerName}` : `From ${t.peerName}`} · ${size} · ${timeAgo(t.finishedAt, now)}`;
      case 'failed':
        return t.error ?? 'The transfer failed';
      case 'cancelled':
        return `${to} · Cancelled ${timeAgo(t.finishedAt, now)}`;
      case 'declined':
        return `${to} · Declined`;
    }
  }

  let zw = $state(0);
  let zh = $state(0);

  async function clear() {
    await store.run(() => api.clearTransfers());
  }
</script>

<PageHeader title="Transfers">
  {#snippet subtitle()}
    <span>
      {#if live.length}
        {plural(live.length, 'transfer')} in progress
      {:else}
        Drop files on a computer to send them — they arrive in its Crispy folder.
      {/if}
    </span>
  {/snippet}
  {#snippet actions()}
    <Button variant="ghost" icon={FolderOpen} onclick={() => store.run(() => api.openSaveDir())}>Open folder</Button>
    <Button disabled={!done.length} onclick={clear}>Clear finished</Button>
  {/snippet}
</PageHeader>

<div class="zone" bind:clientWidth={zw} bind:clientHeight={zh} class:over={dragOver} class:armed={store.drag.active} class:disabled={!canSend} data-drop={canSend ? 'any' : undefined}>
  <svg class="border" aria-hidden="true" preserveAspectRatio="none">
    <rect x="1" y="1" width={Math.max(0, zw - 2)} height={Math.max(0, zh - 2)} rx="17" ry="17" />
  </svg>
  <div class="zone-icon" aria-hidden="true"><ArrowDownToLine size={22} strokeWidth={1.6} /></div>
  <div class="zone-text">
    <p class="zone-title">
      {#if !canSend}Connect a computer to send files{:else if dragOver}Release to send{:else}Drop files to send{/if}
    </p>
    <p class="zone-sub">
      {#if canSend}
        {store.connected.length > 1 ? 'You’ll choose which computer gets them.' : `They go straight to ${store.connected[0].name}.`}
        <button type="button" class="link" onclick={() => store.chooseAndSend()}>Choose files…</button>
      {:else}
        Received files are saved to <span class="mono">{saveDir}</span>
      {/if}
    </p>
  </div>
</div>

{#if live.length}
  <section aria-labelledby="live-title">
    <h2 id="live-title" class="eyebrow">In progress</h2>
    <ul class="list">
      {#each live as t (t.id)}
        <li class="row state-{t.state}" animate:flip={{ duration: reduced() ? 1 : 260 }} in:rise={{ y: 6 }}>
          {@render row(t)}
        </li>
      {/each}
    </ul>
  </section>
{/if}

{#if done.length}
  <section aria-labelledby="done-title">
    <h2 id="done-title" class="eyebrow">Earlier</h2>
    <ul class="list">
      {#each done as t (t.id)}
        <li class="row state-{t.state}" animate:flip={{ duration: reduced() ? 1 : 260 }}>
          {@render row(t)}
        </li>
      {/each}
    </ul>
  </section>
{/if}

{#if !transfers.length}
  <div class="empty">
    <Sparkles size={18} strokeWidth={1.5} aria-hidden="true" />
    <p>Nothing sent yet. Copy and paste works across computers too — files and images included.</p>
  </div>
{/if}

{#snippet row(t: Transfer)}
  {@const Icon = t.purpose === 'clipboard' ? ClipboardIcon : t.direction === 'send' ? ArrowUpRight : ArrowDownLeft}
  <div class="tile" aria-hidden="true">
    <Icon size={17} strokeWidth={1.6} />
    {#if t.state === 'done'}<span class="badge ok"><Check size={9} strokeWidth={3.2} /></span>{/if}
    {#if t.state === 'failed'}<span class="badge bad"><X size={9} strokeWidth={3.2} /></span>{/if}
  </div>
  <div class="body">
    <div class="line">
      <p class="title truncate" title={t.files.map((f) => f.name).join('\n')}>
        {#if t.purpose === 'clipboard'}<span class="kind">Clipboard</span>{/if}{transferTitle(t)}
      </p>
      {#if t.state === 'active'}
        <span class="pct tnum">{Math.floor((t.doneBytes / Math.max(1, t.totalBytes)) * 100)}%</span>
      {/if}
    </div>
    <p class="detail tnum truncate" class:error={t.state === 'failed'}>{detail(t)}</p>
    {#if t.state === 'active' || t.state === 'pending'}
      <div class="progress">
        <ProgressBar value={t.doneBytes / Math.max(1, t.totalBytes)} active={t.state === 'active'} label="{transferTitle(t)} progress" />
      </div>
      {#if t.currentFile && t.fileCount > 1}
        <p class="current truncate">{t.direction === 'send' ? 'Sending' : 'Receiving'} {t.currentFile}</p>
      {/if}
    {/if}
  </div>
  <div class="actions">
    {#if t.state === 'awaiting' && t.direction === 'receive'}
      <Button size="sm" variant="secondary" onclick={() => store.run(() => api.answerTransfer(t.id, false))}>Decline</Button>
      <Button size="sm" variant="primary" onclick={() => store.run(() => api.answerTransfer(t.id, true))}>Accept</Button>
    {:else if !isFinished(t)}
      <IconButton icon={X} label="Cancel transfer" onclick={() => store.run(() => api.cancelTransfer(t.id))} />
    {:else if t.state === 'done' && t.savePath}
      <IconButton icon={FolderOpen} label="Show in folder" onclick={() => store.run(() => api.revealPath(t.savePath!))} />
    {/if}
  </div>
{/snippet}

<style>
  .zone {
    position: relative;
    display: flex;
    align-items: center;
    gap: 18px;
    min-height: 116px;
    padding: 24px 28px;
    border-radius: 18px;
    background:
      radial-gradient(80% 140% at 0% 50%, rgb(var(--accent-rgb) / 0.06), transparent 60%),
      var(--fill-1);
    transition:
      background var(--dur-2) var(--ease),
      transform var(--dur-2) var(--ease),
      box-shadow var(--dur-2) var(--ease);
  }
  .border {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
    pointer-events: none;
  }
  .border rect {
    fill: none;
    stroke: var(--border-heavy);
    stroke-width: 1.5;
    stroke-dasharray: 1 7;
    stroke-linecap: round;
    transition: stroke var(--dur-2) var(--ease);
  }
  .armed .border rect {
    stroke: rgb(var(--accent-rgb) / 0.55);
    animation: march 1.2s linear infinite;
  }
  .over {
    background:
      radial-gradient(80% 160% at 0% 50%, rgb(var(--accent-rgb) / 0.16), transparent 70%),
      var(--accent-softer);
    transform: scale(1.006);
    box-shadow: 0 0 40px -14px rgb(var(--accent-rgb) / 0.6);
  }
  .over .border rect {
    stroke: var(--accent);
    stroke-width: 2;
    stroke-dasharray: 6 6;
  }
  @keyframes march {
    to {
      stroke-dashoffset: -16;
    }
  }
  .zone-icon {
    width: 52px;
    height: 52px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 15px;
    color: var(--accent-ink);
    background: linear-gradient(180deg, var(--surface-3), var(--surface-2));
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      var(--highlight-strong),
      0 10px 24px -14px rgb(0 0 0 / 0.6);
    transition: transform var(--dur-3) var(--ease-spring);
  }
  :global([data-theme='light']) .zone-icon {
    background: linear-gradient(180deg, #fff, #f4f1eb);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 10px 24px -14px rgb(60 40 10 / 0.3);
  }
  .over .zone-icon {
    transform: translateY(3px) scale(1.06);
  }
  .disabled .zone-icon {
    color: var(--text-3);
  }
  .zone-title {
    font-size: 15px;
    font-weight: 620;
    letter-spacing: -0.012em;
  }
  .zone-sub {
    margin-top: 3px;
    color: var(--text-2);
    font-size: 12.5px;
  }
  .zone-sub .mono {
    font-size: 11.5px;
    color: var(--text-1);
  }
  .link {
    margin-left: 4px;
    color: var(--accent-ink);
    font-weight: 600;
    text-decoration: underline;
    text-decoration-color: rgb(var(--accent-rgb) / 0.35);
    text-underline-offset: 2px;
  }

  section {
    margin-top: 32px;
  }
  h2 {
    margin: 0 0 10px 4px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    border-radius: var(--r-lg);
    background: var(--surface-1);
    box-shadow:
      inset 0 0 0 1px var(--border),
      var(--highlight);
    overflow: hidden;
  }
  :global([data-theme='light']) .list {
    box-shadow:
      0 0 0 1px var(--border),
      0 1px 2px rgb(60 40 10 / 0.04);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 14px 14px 16px;
    background: var(--surface-1);
  }
  .row + .row {
    border-top: 1px solid var(--border);
  }
  .tile {
    position: relative;
    width: 38px;
    height: 38px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 11px;
    color: var(--text-2);
    background: var(--fill-2);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .state-active .tile,
  .state-awaiting .tile {
    color: var(--accent-ink);
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px rgb(var(--accent-rgb) / 0.25);
  }
  .badge {
    position: absolute;
    right: -3px;
    bottom: -3px;
    width: 15px;
    height: 15px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: #fff;
    box-shadow: 0 0 0 2px var(--surface-1);
  }
  .badge.ok {
    background: var(--green);
  }
  .badge.bad {
    background: var(--red);
  }
  .body {
    flex: 1;
    min-width: 0;
  }
  .line {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }
  .title {
    flex: 1;
    font-weight: 580;
    font-size: 13.5px;
  }
  .kind {
    margin-right: 7px;
    padding: 1px 6px;
    border-radius: 5px;
    font-size: 10.5px;
    font-weight: 650;
    letter-spacing: 0.03em;
    color: var(--text-2);
    background: var(--fill-2);
    vertical-align: 1px;
  }
  .pct {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent-ink);
  }
  .detail {
    margin-top: 1px;
    font-size: 12px;
    color: var(--text-3);
  }
  .detail.error {
    color: var(--red);
  }
  .state-awaiting .detail,
  .state-active .detail {
    color: var(--text-2);
  }
  .progress {
    margin-top: 9px;
  }
  .current {
    margin-top: 6px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }
  .state-cancelled .title,
  .state-declined .title {
    color: var(--text-2);
  }

  .empty {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-top: 28px;
    padding: 0 4px;
    color: var(--text-3);
    font-size: 12.5px;
  }
</style>
