<script lang="ts">
  import Globe from '@lucide/svelte/icons/globe';
  import { api } from '../api';
  import { store } from '../store.svelte';
  import { thisDeviceLabel } from '../util';
  import Button from './Button.svelte';
  import Modal from './Modal.svelte';
  import TextField from './TextField.svelte';

  let address = $state('');
  let error = $state<string | null>(null);
  let busy = $state(false);

  $effect(() => {
    if (store.addByIpOpen) {
      address = '';
      error = null;
      busy = false;
    }
  });

  const valid = $derived(/^(\[[0-9a-fA-F:]+\]|[\w.-]+)(:\d{1,5})?$/.test(address.trim()));

  function close() {
    store.addByIpOpen = false;
  }

  async function connect(e: SubmitEvent) {
    e.preventDefault();
    if (!valid || busy) return;
    busy = true;
    error = null;
    try {
      await api.connectAddress(address.trim());
      close();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  const mine = $derived(store.me?.addresses.find((a) => !a.startsWith('[')) ?? store.me?.addresses[0]);
</script>

<Modal open={store.addByIpOpen} width={430} labelledby="ip-title" describedby="ip-desc" onclose={close} initialFocus="#ip-input">
  <form class="body" onsubmit={connect}>
    <div class="icon" aria-hidden="true"><Globe size={20} strokeWidth={1.6} /></div>
    <h2 id="ip-title">Add a computer by address</h2>
    <p id="ip-desc">For networks where computers can’t find each other on their own, like many office and guest Wi-Fi networks.</p>

    <label class="lbl" for="ip-input">IP address or host name</label>
    <TextField id="ip-input" bind:value={address} mono invalid={!!error} placeholder="192.168.1.42 or studio-pc.local" aria-invalid={!!error} aria-describedby="ip-err" />
    {#if error}
      <p class="err" id="ip-err" role="alert">{error}</p>
    {:else}
      <p class="help">The port is optional — Crispy uses <span class="mono">{store.settings?.network.port ?? 24727}</span> by default.</p>
    {/if}

    {#if mine}
      <div class="mine">
        <span>{thisDeviceLabel(store.os)} is at</span>
        <span class="mono selectable">{mine}</span>
      </div>
    {/if}

    <footer>
      <Button variant="secondary" onclick={close}>Cancel</Button>
      <Button variant="primary" type="submit" disabled={!valid} loading={busy}>Connect</Button>
    </footer>
  </form>
</Modal>

<style>
  .body {
    padding: 28px 28px 22px;
  }
  .icon {
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: 12px;
    margin-bottom: 14px;
    color: var(--text-1);
    background: var(--fill-2);
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }
  h2 {
    font-size: 17px;
    font-weight: 640;
    letter-spacing: -0.014em;
  }
  #ip-desc {
    margin: 5px 0 20px;
    color: var(--text-2);
    line-height: 1.5;
  }
  .lbl {
    display: block;
    font-size: 12px;
    font-weight: 560;
    color: var(--text-2);
    margin-bottom: 6px;
  }
  .help,
  .err {
    margin-top: 7px;
    font-size: 12px;
    color: var(--text-3);
  }
  .err {
    color: var(--red);
  }
  .mine {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    margin-top: 18px;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--fill-1);
    box-shadow: inset 0 0 0 1px var(--border);
    font-size: 12px;
    color: var(--text-2);
  }
  .mine .mono {
    color: var(--text-1);
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 24px;
  }
</style>
