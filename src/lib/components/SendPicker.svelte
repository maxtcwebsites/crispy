<script lang="ts">
  // Asks which computer should receive a set of files (OS "Send To" hand-off, or a drop with
  // several computers connected).
  import Files from '@lucide/svelte/icons/files';
  import Check from '@lucide/svelte/icons/check';
  import { store } from '../store.svelte';
  import { basename, statusLabel } from '../util';
  import Button from './Button.svelte';
  import Modal from './Modal.svelte';
  import OsGlyph from './OsGlyph.svelte';
  import StatusDot from './StatusDot.svelte';

  const paths = $derived(store.sendPaths);
  const names = $derived((paths ?? []).map(basename));
  const peers = $derived(store.paired);

  let chosen = $state<string | null>(null);
  $effect(() => {
    if (!paths) return;
    const preferred = store.focusPeer && store.connected.some((p) => p.id === store.focusPeer!.id) ? store.focusPeer.id : store.connected[0]?.id;
    chosen = preferred ?? null;
  });

  function close() {
    store.sendPaths = null;
  }
  function send() {
    if (!paths || !chosen) return;
    const id = chosen;
    close();
    void store.sendTo(id, paths);
  }
  function keydown(e: KeyboardEvent) {
    const list = store.connected;
    if (!list.length) return;
    const i = list.findIndex((p) => p.id === chosen);
    if (e.key === 'ArrowDown') chosen = list[(i + 1) % list.length].id;
    else if (e.key === 'ArrowUp') chosen = list[(i - 1 + list.length) % list.length].id;
    else return;
    e.preventDefault();
  }
</script>

<Modal open={!!paths} width={460} labelledby="send-title" onclose={close} initialFocus=".choice.selected">
  <div class="head">
    <div class="stack" aria-hidden="true"><Files size={22} strokeWidth={1.5} /></div>
    <h2 id="send-title">Send {names.length === 1 ? names[0] : `${names.length} files`}</h2>
    {#if names.length > 1}
      <p class="names truncate">{names.slice(0, 3).join(', ')}{names.length > 3 ? ` and ${names.length - 3} more` : ''}</p>
    {/if}
  </div>

  <div class="choices" role="radiogroup" aria-label="Send to" tabindex="-1" onkeydown={keydown}>
    {#each peers as p (p.id)}
      {@const ok = p.status === 'connected' && p.enabled}
      <button
        type="button"
        role="radio"
        class="choice"
        class:selected={chosen === p.id}
        aria-checked={chosen === p.id}
        disabled={!ok}
        onclick={() => (chosen = p.id)}
        ondblclick={() => ok && ((chosen = p.id), send())}
      >
        <span class="glyph"><OsGlyph os={p.os} size={16} /></span>
        <span class="text">
          <span class="name">{p.name}</span>
          <span class="status"><StatusDot tone={ok ? 'green' : 'grey'} size={6} />{statusLabel(p)}</span>
        </span>
        <span class="tick" aria-hidden="true">{#if chosen === p.id}<Check size={14} strokeWidth={2.4} />{/if}</span>
      </button>
    {:else}
      <p class="none">Pair a computer first — then you can send it files.</p>
    {/each}
  </div>

  <footer>
    <Button variant="secondary" onclick={close}>Cancel</Button>
    <Button variant="primary" disabled={!chosen} onclick={send}>Send</Button>
  </footer>
</Modal>

<style>
  .head {
    padding: 30px 28px 6px;
    text-align: center;
  }
  .stack {
    width: 48px;
    height: 48px;
    margin: 0 auto 14px;
    display: grid;
    place-items: center;
    border-radius: 14px;
    color: var(--accent-ink);
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px rgb(var(--accent-rgb) / 0.28);
  }
  h2 {
    font-size: 17px;
    font-weight: 640;
    letter-spacing: -0.014em;
    overflow-wrap: anywhere;
  }
  .names {
    margin-top: 4px;
    color: var(--text-3);
    font-size: 12.5px;
  }
  .choices {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 18px 22px 4px;
  }
  .choice {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border-radius: 12px;
    text-align: left;
    background: var(--fill-1);
    box-shadow: inset 0 0 0 1px var(--border);
    transition:
      background var(--dur-1) var(--ease),
      box-shadow var(--dur-1) var(--ease);
  }
  .choice:hover:not(:disabled) {
    background: var(--fill-2);
  }
  .choice.selected {
    background: var(--accent-softer);
    box-shadow: inset 0 0 0 1.5px rgb(var(--accent-rgb) / 0.7);
  }
  .choice:disabled {
    opacity: 0.45;
  }
  .glyph {
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: 9px;
    background: var(--fill-2);
    flex: none;
  }
  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .name {
    font-weight: 580;
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-3);
  }
  .tick {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: var(--accent-contrast);
  }
  .selected .tick {
    background: var(--accent);
  }
  .none {
    text-align: center;
    color: var(--text-2);
    padding: 12px;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 20px 22px 22px;
  }
</style>
