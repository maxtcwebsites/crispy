<script lang="ts">
  import X from '@lucide/svelte/icons/x';
  import { store } from '../store.svelte';
  import type { Hotkey } from '../types';
  import { hotkeyParts, modLabel, MOD_ORDER } from '../util';
  import Keycap from './Keycap.svelte';

  interface Props {
    value: Hotkey;
    label: string;
    id?: string;
  }
  let { value = $bindable(), label, id }: Props = $props();

  let recording = $state(false);
  let live = $state({ ctrl: false, shift: false, alt: false, meta: false });
  let hint = $state<string | null>(null);
  let el: HTMLButtonElement | undefined = $state();

  const parts = $derived(hotkeyParts(value, store.keys, store.os));
  const liveParts = $derived(MOD_ORDER[store.os].filter((m) => live[m]).map((m) => modLabel(m, store.os)));

  const MODIFIER_CODES = new Set(['ControlLeft', 'ControlRight', 'ShiftLeft', 'ShiftRight', 'AltLeft', 'AltRight', 'MetaLeft', 'MetaRight', 'OSLeft', 'OSRight']);
  const STANDALONE = /^(F\d{1,2}|Pause|ScrollLock|PrintScreen|Insert|Media.*|AudioVolume.*)$/;

  function start() {
    recording = true;
    hint = null;
    live = { ctrl: false, shift: false, alt: false, meta: false };
  }
  function stop() {
    recording = false;
    hint = null;
  }

  function keydown(e: KeyboardEvent) {
    if (!recording) {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        start();
      } else if ((e.key === 'Backspace' || e.key === 'Delete') && value.key) {
        e.preventDefault();
        clear();
      }
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    live = { ctrl: e.ctrlKey, shift: e.shiftKey, alt: e.altKey, meta: e.metaKey };
    if (MODIFIER_CODES.has(e.code)) return;
    const anyMod = e.ctrlKey || e.shiftKey || e.altKey || e.metaKey;
    if (e.code === 'Escape' && !anyMod) return stop();
    if ((e.code === 'Backspace' || e.code === 'Delete') && !anyMod) {
      clear();
      return stop();
    }
    const info = store.keys.find((k) => k.code === e.code);
    if (!info) {
      hint = 'That key can’t be used';
      return;
    }
    if (!anyMod && !STANDALONE.test(e.code)) {
      hint = `Add a modifier, like ${modLabel('ctrl', store.os)} or ${modLabel('alt', store.os)}`;
      return;
    }
    value = { ctrl: e.ctrlKey, shift: e.shiftKey, alt: e.altKey, meta: e.metaKey, key: info.hid };
    stop();
  }

  function keyup(e: KeyboardEvent) {
    if (!recording) return;
    live = { ctrl: e.ctrlKey, shift: e.shiftKey, alt: e.altKey, meta: e.metaKey };
  }

  function clear() {
    value = { ctrl: false, shift: false, alt: false, meta: false, key: 0 };
  }
</script>

<div class="wrap">
  <button
    bind:this={el}
    {id}
    type="button"
    class="rec"
    class:recording
    class:empty={!parts.length}
    aria-label="{label}: {recording ? 'recording, press the new shortcut' : parts.length ? parts.join(' ') : 'not set'}"
    onclick={() => (recording ? stop() : start())}
    onkeydown={keydown}
    onkeyup={keyup}
    onblur={stop}
  >
    {#if recording}
      {#if liveParts.length}
        <span class="caps">{#each liveParts as p, i (i)}<Keycap label={p} size="sm" active />{/each}</span>
        <span class="dots" aria-hidden="true"><i></i><i></i><i></i></span>
      {:else}
        <span class="prompt">{hint ?? 'Press keys…'}</span>
      {/if}
    {:else if parts.length}
      <span class="caps">{#each parts as p, i (i)}<Keycap label={p} size="sm" />{/each}</span>
    {:else}
      <span class="prompt">Not set</span>
    {/if}
  </button>
  {#if value.key && !recording}
    <button type="button" class="clear" aria-label="Clear {label} shortcut" onclick={clear}>
      <X size={12} strokeWidth={2} aria-hidden="true" />
    </button>
  {/if}
  {#if recording && hint && liveParts.length}<span class="hint" role="status">{hint}</span>{/if}
</div>

<style>
  .wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .rec {
    min-width: 148px;
    height: 32px;
    padding: 0 8px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border-radius: var(--r-sm);
    background: var(--surface-sunken);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    transition:
      box-shadow var(--dur-2) var(--ease),
      background var(--dur-2) var(--ease);
  }
  :global([data-theme='light']) .rec {
    background: #fff;
  }
  .rec:hover {
    box-shadow: inset 0 0 0 1px var(--border-heavy);
  }
  .rec.recording {
    background: var(--accent-softer);
    box-shadow:
      inset 0 0 0 1.5px var(--accent),
      0 0 0 4px rgb(var(--accent-rgb) / 0.14);
    animation: breathe 1.6s var(--ease-in-out) infinite;
  }
  .rec:focus-visible {
    border-radius: var(--r-sm);
  }
  .caps {
    display: inline-flex;
    gap: 3px;
  }
  .prompt {
    font-size: 12.5px;
    color: var(--text-3);
  }
  .recording .prompt {
    color: var(--accent-ink);
    font-weight: 560;
  }
  .dots {
    display: inline-flex;
    gap: 3px;
  }
  .dots i {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--accent);
    animation: blink 1s infinite;
  }
  .dots i:nth-child(2) {
    animation-delay: 0.15s;
  }
  .dots i:nth-child(3) {
    animation-delay: 0.3s;
  }
  .clear {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border-radius: 6px;
    color: var(--text-3);
  }
  .clear:hover {
    color: var(--text-1);
    background: var(--fill-2);
  }
  .hint {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    font-size: 11.5px;
    color: var(--accent-ink);
    white-space: nowrap;
  }
  @keyframes blink {
    0%,
    100% {
      opacity: 0.25;
    }
    50% {
      opacity: 1;
    }
  }
  @keyframes breathe {
    0%,
    100% {
      box-shadow:
        inset 0 0 0 1.5px var(--accent),
        0 0 0 3px rgb(var(--accent-rgb) / 0.12);
    }
    50% {
      box-shadow:
        inset 0 0 0 1.5px var(--accent),
        0 0 0 5px rgb(var(--accent-rgb) / 0.2);
    }
  }
</style>
