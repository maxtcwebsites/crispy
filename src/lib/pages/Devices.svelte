<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus';
  import Radar from '@lucide/svelte/icons/radar';
  import { api } from '../api';
  import { store } from '../store.svelte';
  import type { Peer } from '../types';
  import { osName, plural, thisDeviceLabel } from '../util';
  import Button from '../components/Button.svelte';
  import ChipMark from '../components/ChipMark.svelte';
  import DeviceCard from '../components/DeviceCard.svelte';
  import OsGlyph from '../components/OsGlyph.svelte';
  import PageHeader from '../components/PageHeader.svelte';
  import StatusDot from '../components/StatusDot.svelte';

  const here = $derived(thisDeviceLabel(store.os).replace('This', 'this'));

  const live = $derived.by((): { text: string; tone: 'green' | 'amber' | 'grey' | 'accent'; pulse?: boolean } => {
    const s = store.state;
    if (!s) return { text: '', tone: 'grey' };
    if (store.controlledBy) return { text: `${store.controlledBy.name} is driving ${here}`, tone: 'accent', pulse: true };
    if (s.paused) return { text: 'Sharing is paused', tone: 'amber' };
    const fp = store.focusPeer;
    if (fp) return { text: `Keyboard & mouse on ${fp.name}${fp.rttMs != null ? ` · ${fp.rttMs} ms` : ''}`, tone: 'accent', pulse: true };
    if (s.locked) return { text: `Cursor locked to ${here}`, tone: 'amber' };
    const n = store.connected.length;
    if (n === 0) return { text: store.paired.length ? 'No computers connected right now' : 'Waiting for your first computer', tone: 'grey' };
    return { text: `Keyboard & mouse here · ${n === 1 ? `${store.connected[0].name} connected` : `${n} computers connected`}`, tone: 'green' };
  });

  let pairing = $state<string | null>(null);
  async function pair(p: Peer) {
    pairing = p.id;
    await store.run(() => api.pairStart(p.id), `Couldn’t start pairing with ${p.name}`);
    pairing = null;
  }

  const discovery = $derived(store.settings?.network.discovery ?? true);
</script>

<PageHeader title="Your desk">
  {#snippet subtitle()}
    <StatusDot tone={live.tone} pulse={live.pulse} />
    <span>{live.text}</span>
  {/snippet}
  {#snippet actions()}
    <Button icon={Plus} onclick={() => (store.addByIpOpen = true)}>Add by IP</Button>
  {/snippet}
</PageHeader>

<section class="grid" aria-label="Your computers">
  {#if store.me}
    <DeviceCard me={store.me} />
  {/if}
  {#each store.paired as p (p.id)}
    <DeviceCard peer={p} />
  {/each}
  {#if store.paired.length === 0}
    <div class="empty">
      <div class="empty-art" aria-hidden="true">
        <span class="orbit"></span>
        <ChipMark size={64} />
      </div>
      <h2 class="serif">Just you, for now.</h2>
      <p>Install Crispy on your other computer and make sure both are on the same network. It will show up below, ready to pair.</p>
      <Button size="sm" variant="secondary" icon={Plus} onclick={() => (store.addByIpOpen = true)}>Add by IP address</Button>
    </div>
  {/if}
</section>

<section class="nearby" aria-labelledby="nearby-title">
  <header>
    <h2 id="nearby-title" class="eyebrow">Nearby</h2>
    <span class="scan" class:off={!discovery}>
      {#if discovery}
        <StatusDot tone="green" pulse size={6} /> Looking on your network
      {:else}
        Discovery is off
      {/if}
    </span>
  </header>

  {#if store.nearby.length}
    <ul class="list">
      {#each store.nearby as p (p.id)}
        <li class="row">
          <span class="glyph"><OsGlyph os={p.os} size={16} /></span>
          <div class="names">
            <p class="name truncate">{p.name}</p>
            <p class="sub truncate">{osName(p.os)}{#if p.version}<span class="dot">·</span>Crispy {p.version}{/if}</p>
          </div>
          {#if p.address}<span class="addr mono">{p.address}</span>{/if}
          <Button size="sm" variant="primary" loading={pairing === p.id} onclick={() => pair(p)}>Pair</Button>
        </li>
      {/each}
    </ul>
  {:else}
    <div class="none">
      <Radar size={18} strokeWidth={1.5} aria-hidden="true" />
      <p>
        {#if discovery}
          No other computers found yet. Open Crispy on another computer on this network — or
          <button type="button" class="link" onclick={() => (store.addByIpOpen = true)}>add one by its IP address</button>.
        {:else}
          Automatic discovery is turned off.
          <button type="button" class="link" onclick={() => store.go('settings', 'network')}>Turn it on</button> or
          <button type="button" class="link" onclick={() => (store.addByIpOpen = true)}>add a computer by IP address</button>.
        {/if}
      </p>
    </div>
  {/if}
  {#if store.paired.length > 0}
    <p class="hint">{plural(store.paired.length, 'paired computer')} · drop files on a computer to send them there.</p>
  {/if}
</section>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(226px, 1fr));
    align-items: start;
    gap: 14px;
  }

  .empty {
    grid-column: 2 / -1;
    min-height: 240px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 28px 28px 30px;
    border-radius: var(--r-lg);
    background:
      radial-gradient(60% 70% at 50% 30%, rgb(var(--accent-rgb) / 0.08), transparent 70%),
      var(--fill-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .empty-art {
    position: relative;
    width: 96px;
    height: 76px;
    display: grid;
    place-items: center;
    animation: float 5s var(--ease-in-out) infinite;
  }
  .orbit {
    position: absolute;
    inset: 4px -6px;
    border-radius: 50%;
    border: 1px dashed rgb(var(--accent-rgb) / 0.35);
    animation: spin 28s linear infinite;
  }
  .empty h2 {
    font-size: 26px;
    font-style: italic;
    margin-top: 10px;
  }
  .empty p {
    max-width: 40ch;
    margin: 6px 0 16px;
    color: var(--text-2);
    line-height: 1.5;
  }

  .nearby {
    margin-top: 40px;
  }
  .nearby header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: 0 4px 10px;
  }
  .scan {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 11.5px;
    color: var(--text-3);
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
  }
  :global([data-theme='light']) .list {
    box-shadow:
      0 0 0 1px var(--border),
      0 1px 2px rgb(60 40 10 / 0.04);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 12px 12px 14px;
  }
  .row + .row {
    border-top: 1px solid var(--border);
  }
  .glyph {
    width: 32px;
    height: 32px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 9px;
    background: var(--fill-2);
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text-2);
  }
  .names {
    flex: 1;
    min-width: 0;
  }
  .name {
    font-weight: 580;
    font-size: 13.5px;
  }
  .sub {
    font-size: 12px;
    color: var(--text-3);
  }
  .dot {
    margin: 0 5px;
  }
  .addr {
    font-size: 11.5px;
    color: var(--text-3);
  }
  .none {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    padding: 16px 18px;
    border-radius: var(--r-lg);
    background: var(--fill-1);
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--text-2);
    font-size: 12.5px;
    line-height: 1.55;
  }
  .none :global(svg) {
    flex: none;
    margin-top: 1px;
    color: var(--text-3);
  }
  .link {
    color: var(--accent-ink);
    font-weight: 560;
    text-decoration: underline;
    text-decoration-color: rgb(var(--accent-rgb) / 0.35);
    text-underline-offset: 2px;
    cursor: pointer;
  }
  .hint {
    margin: 12px 4px 0;
    font-size: 12px;
    color: var(--text-3);
  }

  @keyframes float {
    0%,
    100% {
      transform: translateY(0) rotate(0deg);
    }
    50% {
      transform: translateY(-5px) rotate(-2deg);
    }
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
