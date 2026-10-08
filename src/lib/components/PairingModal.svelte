<script lang="ts">
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import ShieldCheck from '@lucide/svelte/icons/shield-check';
  import { api } from '../api';
  import { store } from '../store.svelte';
  import Button from './Button.svelte';
  import Modal from './Modal.svelte';
  import OsGlyph from './OsGlyph.svelte';
  import Spinner from './Spinner.svelte';

  const p = $derived(store.state?.pairing ?? null);
  const me = $derived(store.me);

  let digits = $state<string[]>(['', '', '', '', '', '']);
  let inputs: HTMLInputElement[] = $state([]);
  let submitted = $state(false);

  // Fresh boxes whenever we (re-)enter the code stage.
  let lastStage = '';
  $effect(() => {
    const stage = p?.stage ?? '';
    if (stage === 'enterCode' && lastStage !== 'enterCode') {
      digits = ['', '', '', '', '', ''];
      submitted = false;
      requestAnimationFrame(() => inputs[0]?.focus());
    }
    lastStage = stage;
  });

  function setFrom(i: number, text: string) {
    const clean = text.replace(/\D/g, '').slice(0, 6 - i);
    clean.split('').forEach((d, k) => (digits[i + k] = d));
    const next = Math.min(5, i + clean.length);
    inputs[next]?.focus();
    maybeSubmit();
  }

  function oninput(i: number, e: Event & { currentTarget: HTMLInputElement }) {
    const v = e.currentTarget.value.replace(/\D/g, '');
    if (v.length > 1) {
      setFrom(i, v);
      return;
    }
    digits[i] = v;
    e.currentTarget.value = v;
    if (v && i < 5) inputs[i + 1]?.focus();
    maybeSubmit();
  }

  function onkeydown(i: number, e: KeyboardEvent) {
    if (e.key === 'Backspace' && !digits[i] && i > 0) {
      e.preventDefault();
      digits[i - 1] = '';
      inputs[i - 1]?.focus();
    } else if (e.key === 'ArrowLeft' && i > 0) {
      e.preventDefault();
      inputs[i - 1]?.focus();
    } else if (e.key === 'ArrowRight' && i < 5) {
      e.preventDefault();
      inputs[i + 1]?.focus();
    }
  }

  function onpaste(i: number, e: ClipboardEvent) {
    const text = e.clipboardData?.getData('text') ?? '';
    if (!/\d/.test(text)) return;
    e.preventDefault();
    setFrom(i, text);
  }

  function maybeSubmit() {
    const code = digits.join('');
    if (code.length === 6 && !submitted) {
      submitted = true;
      void store.run(() => api.pairSubmit(code), 'Couldn’t check the code');
    }
  }

  const cancel = () => store.run(() => api.pairCancel());
  const retry = () => p && store.run(() => api.pairStart(p.peerId), 'Couldn’t start pairing');
  const formatted = $derived(p?.code ? `${p.code.slice(0, 3)} ${p.code.slice(3)}` : '');
  const busy = $derived(p?.stage === 'connecting' || p?.stage === 'verifying');
</script>

<Modal open={!!p} width={452} labelledby="pair-title" describedby="pair-desc" onclose={cancel} closable={p?.stage !== 'verifying'}>
  {#if p && me}
    <div class="pair stage-{p.stage}">
      {#if p.stage !== 'success' && p.stage !== 'failed'}
      <div class="duo" aria-hidden="true">
        <span class="tile"><OsGlyph os={me.os} size={20} /></span>
        <span class="link" class:busy>
          <i></i><i></i><i></i><i></i><i></i>
        </span>
        <span class="tile peer"><OsGlyph os={p.peerOs} size={20} /></span>
      </div>
      {/if}

      {#if p.stage === 'connecting'}
        <h2 id="pair-title">Connecting to {p.peerName}…</h2>
        <p id="pair-desc">Setting up an encrypted connection. This only takes a moment.</p>
        <div class="center"><Spinner size={22} /></div>
      {:else if p.stage === 'enterCode'}
        <h2 id="pair-title">Enter the pairing code</h2>
        <p id="pair-desc">Type the six digits shown on <strong>{p.peerName}</strong>.</p>
        <div class="otp" role="group" aria-label="Pairing code">
          {#each digits as d, i (i)}
            {#if i === 3}<span class="gap" aria-hidden="true"></span>{/if}
            <input
              bind:this={inputs[i]}
              class="box mono"
              class:filled={!!d}
              value={d}
              inputmode="numeric"
              autocomplete={i === 0 ? 'one-time-code' : 'off'}
              maxlength={6}
              aria-label="Digit {i + 1} of 6"
              oninput={(e) => oninput(i, e)}
              onkeydown={(e) => onkeydown(i, e)}
              onpaste={(e) => onpaste(i, e)}
              onfocus={(e) => e.currentTarget.select()}
            />
          {/each}
        </div>
      {:else if p.stage === 'showCode'}
        <h2 id="pair-title">Pair with {p.peerName}</h2>
        <p id="pair-desc">Enter this code on <strong>{p.peerName}</strong> to confirm it’s really you.</p>
        <div class="code mono selectable" aria-label="Pairing code {p.code?.split('').join(' ')}">{formatted}</div>
        <p class="fine"><ShieldCheck size={13} strokeWidth={1.75} aria-hidden="true" /> The code changes every time and works only once.</p>
      {:else if p.stage === 'verifying'}
        <h2 id="pair-title">Checking the code…</h2>
        <p id="pair-desc">Exchanging keys with {p.peerName}.</p>
        <div class="center"><Spinner size={22} /></div>
      {:else if p.stage === 'success'}
        <div class="check" aria-hidden="true">
          <svg viewBox="0 0 64 64" width="64" height="64">
            <circle class="ring" cx="32" cy="32" r="29" />
            <path class="tick" d="M20 33.5l8 8 16-17" />
          </svg>
        </div>
        <h2 id="pair-title">Paired with {p.peerName}</h2>
        <p id="pair-desc">You’re all set. Move your cursor past the edge of the screen to switch computers.</p>
      {:else if p.stage === 'failed'}
        <div class="fail-icon" aria-hidden="true"><TriangleAlert size={22} strokeWidth={1.75} /></div>
        <h2 id="pair-title">Pairing didn’t work</h2>
        <p id="pair-desc">{p.error ?? 'Something interrupted the connection. Please try again.'}</p>
      {/if}
    </div>

    <footer>
      {#if p.stage === 'success'}
        <Button variant="secondary" onclick={() => (cancel(), store.go('arrangement'))}>Arrange displays</Button>
        <Button variant="primary" onclick={cancel}>Done</Button>
      {:else if p.stage === 'failed'}
        <Button variant="secondary" onclick={cancel}>Cancel</Button>
        {#if p.role === 'initiator'}<Button variant="primary" onclick={retry}>Try again</Button>{/if}
      {:else}
        <Button variant="secondary" disabled={p.stage === 'verifying'} onclick={cancel}>Cancel</Button>
      {/if}
    </footer>
  {/if}
</Modal>

<style>
  .pair {
    padding: 34px 32px 6px;
    text-align: center;
  }
  h2 {
    font-size: 18px;
    font-weight: 640;
    letter-spacing: -0.016em;
  }
  p {
    margin: 6px auto 0;
    max-width: 34ch;
    color: var(--text-2);
    line-height: 1.5;
  }
  strong {
    color: var(--text-1);
    font-weight: 600;
  }
  .center {
    display: grid;
    place-items: center;
    padding: 26px 0 14px;
    color: var(--accent-ink);
  }

  .duo {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    margin-bottom: 22px;
  }
  .tile {
    width: 46px;
    height: 46px;
    display: grid;
    place-items: center;
    border-radius: 14px;
    color: var(--text-1);
    background: linear-gradient(180deg, var(--surface-3), var(--surface-2));
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      var(--highlight-strong),
      0 8px 20px -10px rgb(0 0 0 / 0.5);
  }
  :global([data-theme='light']) .tile {
    background: linear-gradient(180deg, #fff, #f3f0ea);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 8px 20px -12px rgb(60 40 10 / 0.3);
  }
  .link {
    display: flex;
    gap: 6px;
  }
  .link i {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--text-4);
  }
  .link.busy i {
    animation: travel 1.2s var(--ease-in-out) infinite;
  }
  .link.busy i:nth-child(2) {
    animation-delay: 0.12s;
  }
  .link.busy i:nth-child(3) {
    animation-delay: 0.24s;
  }
  .link.busy i:nth-child(4) {
    animation-delay: 0.36s;
  }
  .link.busy i:nth-child(5) {
    animation-delay: 0.48s;
  }
  @keyframes travel {
    0%,
    100% {
      background: var(--text-4);
      transform: scale(1);
    }
    40% {
      background: var(--accent);
      transform: scale(1.5);
    }
  }

  .otp {
    display: flex;
    justify-content: center;
    gap: 8px;
    margin: 24px 0 14px;
  }
  .gap {
    width: 8px;
  }
  .box {
    width: 46px;
    height: 56px;
    border: 0;
    border-radius: 12px;
    text-align: center;
    font-size: 24px;
    font-weight: 560;
    color: var(--text-1);
    background: var(--surface-sunken);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    caret-color: var(--accent);
    transition:
      box-shadow var(--dur-1) var(--ease),
      transform var(--dur-1) var(--ease);
  }
  :global([data-theme='light']) .box {
    background: #fff;
  }
  .box.filled {
    box-shadow: inset 0 0 0 1px var(--border-heavy);
  }
  .box:focus {
    border-radius: 12px;
    box-shadow:
      inset 0 0 0 1.5px var(--accent),
      0 0 0 4px rgb(var(--accent-rgb) / 0.16);
  }

  .code {
    margin: 22px auto 12px;
    width: fit-content;
    padding: 14px 26px;
    border-radius: 16px;
    font-size: 38px;
    font-weight: 600;
    letter-spacing: 0.14em;
    color: var(--text-1);
    background:
      radial-gradient(120% 120% at 50% 0%, rgb(var(--accent-rgb) / 0.1), transparent 70%),
      var(--surface-sunken);
    box-shadow:
      inset 0 0 0 1px rgb(var(--accent-rgb) / 0.28),
      0 0 32px -12px rgb(var(--accent-rgb) / 0.5);
  }
  .fine {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-3);
    max-width: none;
  }

  .check {
    display: grid;
    place-items: center;
    margin: -4px 0 14px;
  }
  .ring {
    fill: rgb(var(--accent-rgb) / 0.12);
    stroke: var(--accent);
    stroke-width: 2;
    stroke-dasharray: 183;
    stroke-dashoffset: 183;
    transform: rotate(-90deg);
    transform-origin: center;
    animation: draw 600ms var(--ease) forwards;
  }
  .tick {
    fill: none;
    stroke: var(--accent);
    stroke-width: 3.2;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-dasharray: 40;
    stroke-dashoffset: 40;
    animation: draw 420ms 380ms var(--ease) forwards;
  }
  .check svg {
    filter: drop-shadow(0 0 14px rgb(var(--accent-rgb) / 0.45));
    animation: pop 700ms var(--ease-spring);
  }
  @keyframes draw {
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes pop {
    0% {
      transform: scale(0.6);
      opacity: 0;
    }
    100% {
      transform: scale(1);
      opacity: 1;
    }
  }
  .fail-icon {
    width: 48px;
    height: 48px;
    margin: -4px auto 14px;
    display: grid;
    place-items: center;
    border-radius: 14px;
    color: var(--red);
    background: rgb(var(--red-rgb) / 0.12);
    box-shadow: inset 0 0 0 1px rgb(var(--red-rgb) / 0.22);
  }

  footer {
    display: flex;
    justify-content: center;
    gap: 8px;
    padding: 22px 32px 28px;
  }
</style>
