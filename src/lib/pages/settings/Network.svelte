<script lang="ts">
  import Copy from '@lucide/svelte/icons/copy';
  import Plus from '@lucide/svelte/icons/plus';
  import X from '@lucide/svelte/icons/x';
  import RefreshCcw from '@lucide/svelte/icons/refresh-ccw';
  import { api } from '../../api';
  import { store } from '../../store.svelte';
  import type { Peer } from '../../types';
  import Button from '../../components/Button.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import OsGlyph from '../../components/OsGlyph.svelte';
  import SettingRow from '../../components/SettingRow.svelte';
  import SettingsGroup from '../../components/SettingsGroup.svelte';
  import Slider from '../../components/Slider.svelte';
  import StatusDot from '../../components/StatusDot.svelte';
  import TextField from '../../components/TextField.svelte';
  import Toggle from '../../components/Toggle.svelte';

  const s = $derived(store.settings!);
  const me = $derived(store.me);

  let port = $state(String(store.settings?.network.port ?? 24727));
  const portValid = $derived(/^\d+$/.test(port) && +port >= 1024 && +port <= 65535);
  function commitPort() {
    if (portValid) s.network.port = +port;
    else port = String(s.network.port);
  }

  let newPeer = $state('');
  const peerValid = $derived(/^(\[[0-9a-fA-F:]+\]|[\w.-]+)(:\d{1,5})?$/.test(newPeer.trim()));
  function addPeer(e: SubmitEvent) {
    e.preventDefault();
    const v = newPeer.trim();
    if (!peerValid || s.network.manualPeers.includes(v)) return;
    s.network.manualPeers = [...s.network.manualPeers, v];
    newPeer = '';
  }

  async function copy(text: string, what: string) {
    try {
      await navigator.clipboard.writeText(text);
      store.toast({ kind: 'success', title: `${what} copied`, body: text }, 2400);
    } catch {
      /* ignore */
    }
  }

  async function unpair(p: Peer) {
    const ok = await store.ask({
      title: `Unpair ${p.name}?`,
      body: 'It will no longer be trusted. Pair again with a new code to reconnect.',
      confirm: 'Unpair',
      danger: true,
    });
    if (ok) await store.run(() => api.unpair(p.id), 'Couldn’t unpair');
  }

  async function regenerate() {
    const ok = await store.ask({
      title: 'Create a new identity?',
      body: 'This computer gets a new fingerprint and every paired computer forgets it. You will need to pair them all again.',
      confirm: 'Create new identity',
      danger: true,
    });
    if (ok) await store.run(() => api.regenerateIdentity(), 'Couldn’t create a new identity');
  }
</script>

{#if me}
  <SettingsGroup title="This computer" note="Compare fingerprints when pairing to be sure you’re connecting to the right computer.">
    <div class="identity">
      <div class="fp">
        <p class="lbl">Fingerprint</p>
        <p class="mono selectable fp-value">{me.fingerprint}</p>
      </div>
      <IconButton icon={Copy} label="Copy fingerprint" onclick={() => copy(me.fingerprint, 'Fingerprint')} />
    </div>
    <div class="addresses">
      <p class="lbl">Listening on</p>
      <ul>
        {#each me.addresses as a (a)}
          <li class="mono selectable">{a}</li>
        {:else}
          <li class="none">No network connection</li>
        {/each}
      </ul>
    </div>
  </SettingsGroup>
{/if}

<SettingsGroup title="Finding computers">
  <SettingRow label="Automatic discovery" description="Find other computers running Crispy on this network." id="n-disc">
    <Toggle id="n-disc" bind:checked={s.network.discovery} />
  </SettingRow>
  <SettingRow label="Port" description="Both computers must use the same port. Restart Crispy after changing it." id="n-port">
    <div class="port">
      <TextField id="n-port" bind:value={port} mono invalid={!portValid} inputmode="numeric" onchange={commitPort} aria-invalid={!portValid} />
    </div>
  </SettingRow>
  <SettingRow label="Connection timeout" description="How long a silent computer is still considered connected." stacked id="n-hb">
    <Slider id="n-hb" label="Connection timeout" bind:value={s.network.heartbeatTimeoutSecs} min={2} max={60} step={1} format={(v) => `${v} s`} />
  </SettingRow>
  <div class="manual">
    <p class="lbl">Always try these addresses</p>
    <p class="desc">For networks that block discovery. Crispy keeps trying to reach them.</p>
    {#if s.network.manualPeers.length}
      <ul class="chips">
        {#each s.network.manualPeers as addr (addr)}
          <li>
            <span class="mono">{addr}</span>
            <button type="button" aria-label="Remove {addr}" onclick={() => (s.network.manualPeers = s.network.manualPeers.filter((x) => x !== addr))}>
              <X size={12} strokeWidth={2} aria-hidden="true" />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
    <form class="add" onsubmit={addPeer}>
      <TextField bind:value={newPeer} mono placeholder="192.168.1.42 or nas.local:24727" aria-label="Address to add" />
      <Button type="submit" icon={Plus} disabled={!peerValid}>Add</Button>
    </form>
  </div>
</SettingsGroup>

<SettingsGroup title="Trusted computers">
  {#each store.paired as p (p.id)}
    <div class="trusted">
      <span class="glyph"><OsGlyph os={p.os} size={15} /></span>
      <div class="t-text">
        <p class="t-name">{p.name} <StatusDot tone={p.status === 'connected' ? 'green' : 'grey'} size={6} /></p>
        <p class="mono t-fp">{p.fingerprint}</p>
      </div>
      <Button size="sm" variant="ghost" onclick={() => unpair(p)}>Unpair</Button>
    </div>
  {:else}
    <p class="empty">No paired computers yet.</p>
  {/each}
</SettingsGroup>

<SettingsGroup title="Danger zone">
  <div class="danger">
    <div>
      <p class="lbl">Create a new identity</p>
      <p class="desc">Generates a new fingerprint and forgets every paired computer.</p>
    </div>
    <Button variant="danger" size="sm" icon={RefreshCcw} onclick={regenerate}>Regenerate…</Button>
  </div>
</SettingsGroup>

<style>
  .lbl {
    font-size: 13.5px;
    font-weight: 540;
  }
  .desc {
    margin-top: 2px;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .identity {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 12px 14px 16px;
    border-bottom: 1px solid var(--border);
  }
  .fp {
    flex: 1;
    min-width: 0;
  }
  .fp-value {
    margin-top: 4px;
    font-size: 17px;
    font-weight: 560;
    letter-spacing: 0.06em;
    color: var(--text-1);
  }
  .addresses {
    padding: 14px 16px;
  }
  .addresses ul {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .addresses li {
    font-size: 12px;
    color: var(--text-2);
  }
  .none {
    color: var(--text-3);
  }
  .port {
    width: 104px;
  }
  .manual {
    padding: 14px 16px 16px;
    border-top: 1px solid var(--border);
  }
  .chips {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chips li {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding: 0 4px 0 10px;
    border-radius: 7px;
    font-size: 12px;
    background: var(--fill-2);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .chips button {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    color: var(--text-3);
  }
  .chips button:hover {
    color: var(--text-1);
    background: var(--fill-3);
  }
  .add {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
  .trusted {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 12px 12px 16px;
  }
  .trusted + .trusted {
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
  }
  .t-text {
    flex: 1;
    min-width: 0;
  }
  .t-name {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 560;
  }
  .t-fp {
    font-size: 11.5px;
    color: var(--text-3);
  }
  .empty {
    padding: 16px;
    color: var(--text-3);
  }
  .danger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 16px;
    border-radius: inherit;
    background: linear-gradient(90deg, rgb(var(--red-rgb) / 0.05), transparent 60%);
  }
</style>
