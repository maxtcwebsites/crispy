<script lang="ts">
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import Check from '@lucide/svelte/icons/check';
  import ShieldCheck from '@lucide/svelte/icons/shield-check';
  import Accessibility from '@lucide/svelte/icons/accessibility';
  import { api, isMock } from '../api';
  import { rise } from '../motion';
  import { store } from '../store.svelte';
  import type { Peer } from '../types';
  import { osName, thisDeviceLabel } from '../util';
  import Button from '../components/Button.svelte';
  import ChipMark from '../components/ChipMark.svelte';
  import OsGlyph from '../components/OsGlyph.svelte';
  import StatusDot from '../components/StatusDot.svelte';
  import TextField from '../components/TextField.svelte';

  const STEPS = 4;
  const initial = isMock ? Number(new URLSearchParams(location.search).get('step') ?? 1) : 1;
  let step = $state(Math.min(STEPS, Math.max(1, initial || 1)));
  let dir = $state(1);

  const me = $derived(store.me!);
  const os = $derived(store.os);
  let name = $state(store.settings?.general.deviceName || store.me?.name || '');
  let checking = $state(false);
  let nameInput: HTMLInputElement | undefined = $state();
  $effect(() => {
    if (step === 2 && nameInput) requestAnimationFrame(() => nameInput?.focus());
  });
  let pairingId = $state<string | null>(null);

  const accessibility = $derived(store.state?.permissions.accessibility ?? 'notRequired');

  function go(n: number) {
    dir = n > step ? 1 : -1;
    step = n;
  }

  function next() {
    if (step === 2) {
      const v = name.trim();
      if (v && store.settings && v !== store.settings.general.deviceName && v !== me.name) store.settings.general.deviceName = v;
    }
    if (step < STEPS) go(step + 1);
  }

  async function finish() {
    if (!store.settings) return;
    store.settings.general.onboarded = true;
    await store.flushSettings();
  }

  async function check() {
    checking = true;
    await store.run(() => api.requestPermissions(), 'Couldn’t check permissions');
    checking = false;
  }

  async function pair(p: Peer) {
    pairingId = p.id;
    await store.run(() => api.pairStart(p.id), `Couldn’t start pairing with ${p.name}`);
    pairingId = null;
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !(e.target as HTMLElement).closest('button') && !document.querySelector('[aria-modal="true"]')) {
      e.preventDefault();
      if (step < STEPS) next();
      else void finish();
    }
  }
</script>

<svelte:window onkeydown={keydown} />

<div class="onboarding" data-tauri-drag-region>
  <div class="glow" aria-hidden="true"></div>
  <div class="grid" aria-hidden="true"></div>

  <main class="stage" aria-live="polite">
    {#key step}
      <section class="step step-{step}" in:rise={{ x: 24 * dir, y: 0, duration: 380 }}>
        {#if step === 1}
          <div class="hero-mark" aria-hidden="true">
            <div class="halo"></div>
            <div class="chip"><ChipMark size={156} /></div>
            <span class="crumb c1"></span><span class="crumb c2"></span><span class="crumb c3"></span>
          </div>
          <h1 class="serif display">Meet <em>Crispy.</em></h1>
          <p class="lede">One keyboard. One mouse. Every computer.</p>
          <p class="sub">Move your cursor off the edge of one screen and keep going on the next — with your clipboard and files along for the ride.</p>
          <div class="actions center">
            <Button size="lg" variant="primary" trailing={ArrowRight} onclick={next}>Get started</Button>
          </div>
          <p class="fine">Takes about a minute.</p>
        {:else if step === 2}
          <p class="eyebrow">Step 1 of 3</p>
          <h1 class="serif title">Name this computer</h1>
          <p class="sub">It’s how this {os === 'macos' ? 'Mac' : 'computer'} will show up on your other computers.</p>
          <div class="name-card">
            <span class="name-glyph"><OsGlyph {os} size={20} /></span>
            <div class="name-field">
              <label class="sr-only" for="ob-name">Computer name</label>
              <TextField id="ob-name" size="lg" bind:value={name} bind:input={nameInput} maxlength={64} />
            </div>
          </div>
          <p class="hint">{osName(os)} · {me.screens.length === 1 ? '1 display' : `${me.screens.length} displays`}</p>
          <div class="actions">
            <Button variant="ghost" icon={ArrowLeft} onclick={() => go(1)}>Back</Button>
            <Button size="lg" variant="primary" trailing={ArrowRight} disabled={!name.trim()} onclick={next}>Continue</Button>
          </div>
        {:else if step === 3}
          <p class="eyebrow">Step 2 of 3</p>
          {#if os === 'macos'}
            <h1 class="serif title">Allow keyboard &amp; mouse control</h1>
            <p class="sub">
              To share this Mac’s keyboard and mouse — and to let your other computers control it — macOS asks you to turn on Crispy in
              <strong>Privacy &amp; Security → Accessibility</strong>.
            </p>
            <div class="perm" class:ok={accessibility !== 'denied'}>
              <span class="perm-icon"><Accessibility size={20} strokeWidth={1.6} aria-hidden="true" /></span>
              <div class="perm-text">
                <p class="perm-title">Accessibility</p>
                <p class="perm-state">
                  {#if accessibility === 'denied'}
                    <StatusDot tone="amber" size={6} /> Not allowed yet
                  {:else}
                    <StatusDot tone="green" size={6} /> Allowed
                  {/if}
                </p>
              </div>
              {#if accessibility === 'denied'}
                <Button size="sm" variant="secondary" loading={checking} onclick={check}>Check again</Button>
                <Button size="sm" variant="primary" onclick={() => store.run(() => api.openPermissionSettings())}>Open System Settings</Button>
              {:else}
                <span class="perm-check"><Check size={16} strokeWidth={2.5} aria-hidden="true" /></span>
              {/if}
            </div>
            <p class="fine left">Crispy never records what you type. Input only travels, encrypted, to computers you’ve paired.</p>
          {:else}
            <h1 class="serif title">One quick heads-up</h1>
            <p class="sub">
              {os === 'windows' ? 'Windows Firewall' : 'Your firewall'} may ask whether Crispy can talk to other computers. Choose
              <strong>Allow</strong> on <strong>private networks</strong> so your computers can find each other.
            </p>
            <div class="firewall" aria-hidden="true">
              <div class="fw-head"><ShieldCheck size={16} strokeWidth={1.75} /> Allow Crispy to communicate on these networks:</div>
              <div class="fw-row on"><span class="box"><Check size={11} strokeWidth={3} /></span> Private networks, such as your home or work network</div>
              <div class="fw-row"><span class="box"></span> Public networks, such as those in airports and cafés</div>
            </div>
            <p class="fine left">Crispy only listens on your local network, and everything it sends is end-to-end encrypted.</p>
          {/if}
          <div class="actions">
            <Button variant="ghost" icon={ArrowLeft} onclick={() => go(2)}>Back</Button>
            <Button size="lg" variant="primary" trailing={ArrowRight} onclick={next}>
              {os === 'macos' && accessibility === 'denied' ? 'I’ll do it later' : 'Continue'}
            </Button>
          </div>
        {:else}
          <p class="eyebrow">Step 3 of 3</p>
          <h1 class="serif title">Now, your other computer</h1>
          <p class="sub">Install Crispy on it and make sure it’s on the same network. It will show up here, ready to pair.</p>
          <div class="identity">
            <div class="id-head">
              <span class="name-glyph small"><OsGlyph {os} size={16} /></span>
              <div>
                <p class="id-name">{me.name}</p>
                <p class="id-sub">{thisDeviceLabel(os)}</p>
              </div>
            </div>
            <dl>
              <div><dt>Fingerprint</dt><dd class="mono selectable">{me.fingerprint}</dd></div>
              <div><dt>Address</dt><dd class="mono selectable">{me.addresses[0] ?? '—'}</dd></div>
            </dl>
            <div class="found">
              {#if store.nearby.length}
                {#each store.nearby.slice(0, 2) as p (p.id)}
                  <div class="found-row">
                    <span class="name-glyph small"><OsGlyph os={p.os} size={15} /></span>
                    <span class="found-name">{p.name}</span>
                    <Button size="sm" variant="primary" loading={pairingId === p.id} onclick={() => pair(p)}>Pair</Button>
                  </div>
                {/each}
              {:else}
                <p class="waiting"><StatusDot tone="green" pulse size={6} /> Looking for computers on your network…</p>
              {/if}
            </div>
          </div>
          <div class="actions">
            <Button variant="ghost" icon={ArrowLeft} onclick={() => go(3)}>Back</Button>
            <Button size="lg" variant="primary" trailing={ArrowRight} onclick={finish}>Start using Crispy</Button>
          </div>
        {/if}
      </section>
    {/key}
  </main>

  <nav class="dots" aria-label="Setup progress">
    {#each Array.from({ length: STEPS }, (_, i) => i + 1) as n (n)}
      <button
        type="button"
        class:active={n === step}
        class:done={n < step}
        aria-label="Step {n}"
        aria-current={n === step ? 'step' : undefined}
        disabled={n > step}
        onclick={() => go(n)}
      ></button>
    {/each}
  </nav>
</div>

<style>
  .onboarding {
    position: fixed;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    overflow: hidden auto;
    background: var(--bg-canvas);
    padding: 48px 32px 72px;
  }
  .glow {
    position: absolute;
    left: 50%;
    top: 34%;
    width: 900px;
    height: 640px;
    transform: translate(-50%, -50%);
    background: radial-gradient(closest-side, rgb(var(--accent-rgb) / 0.14), transparent 75%);
    pointer-events: none;
  }
  .grid {
    position: absolute;
    inset: 0;
    background: radial-gradient(circle at 1px 1px, var(--grid-dot) 1px, transparent 1.4px) 0 0 / 26px 26px;
    -webkit-mask-image: radial-gradient(60% 55% at 50% 40%, #000, transparent 75%);
    mask-image: radial-gradient(60% 55% at 50% 40%, #000, transparent 75%);
    opacity: 0.7;
    pointer-events: none;
  }
  .stage {
    position: relative;
    width: 100%;
    max-width: 560px;
  }
  .step {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .step-1 {
    align-items: center;
    text-align: center;
  }

  .hero-mark {
    position: relative;
    width: 220px;
    height: 180px;
    display: grid;
    place-items: center;
    margin-bottom: 6px;
  }
  .halo {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: radial-gradient(closest-side, rgb(243 181 68 / 0.32), transparent 72%);
    animation: pulse 5s var(--ease-in-out) infinite;
  }
  .chip {
    position: relative;
    animation: drift 6s var(--ease-in-out) infinite;
    filter: drop-shadow(0 26px 30px rgb(0 0 0 / 0.35));
  }
  .chip::after {
    /* a slow glint sweeping across the chip */
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(105deg, transparent 40%, rgb(255 255 255 / 0.35) 50%, transparent 60%);
    background-size: 300% 100%;
    -webkit-mask: url('/favicon.svg') center / contain no-repeat;
    mask: url('/favicon.svg') center / contain no-repeat;
    animation: glint 5.5s var(--ease-in-out) infinite;
  }
  .crumb {
    position: absolute;
    width: 6px;
    height: 6px;
    border-radius: 2px;
    background: #fff6dc;
    opacity: 0.7;
    box-shadow: 0 0 6px rgb(255 230 160 / 0.6);
  }
  .c1 {
    left: 22px;
    top: 52px;
    animation: fall 4.2s 0.3s var(--ease-in-out) infinite;
  }
  .c2 {
    right: 30px;
    top: 34px;
    width: 4px;
    height: 4px;
    animation: fall 5s 1.4s var(--ease-in-out) infinite;
  }
  .c3 {
    right: 52px;
    bottom: 28px;
    width: 5px;
    height: 5px;
    animation: fall 4.6s 2.2s var(--ease-in-out) infinite;
  }

  .display {
    font-size: 76px;
    line-height: 0.98;
    letter-spacing: -0.025em;
  }
  .display em {
    font-style: italic;
    color: var(--text-1);
  }
  .title {
    font-size: 44px;
    line-height: 1.04;
    font-style: italic;
    letter-spacing: -0.02em;
    margin-top: 10px;
  }
  .lede {
    margin-top: 14px;
    font-size: 18px;
    font-weight: 520;
    letter-spacing: -0.012em;
    color: var(--text-1);
  }
  .sub {
    margin-top: 12px;
    max-width: 46ch;
    font-size: 14.5px;
    line-height: 1.55;
    color: var(--text-2);
  }
  .step-1 .sub {
    max-width: 44ch;
  }
  .sub strong {
    color: var(--text-1);
    font-weight: 600;
  }
  .actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    gap: 10px;
    margin-top: 34px;
  }
  .actions.center {
    justify-content: center;
  }
  .fine {
    margin-top: 14px;
    font-size: 12px;
    color: var(--text-3);
  }
  .fine.left {
    margin-top: 14px;
    max-width: 52ch;
  }

  .name-card {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    margin-top: 26px;
    padding: 10px;
    border-radius: 18px;
    background: var(--surface-1);
    box-shadow:
      inset 0 0 0 1px var(--border),
      var(--highlight),
      0 20px 40px -24px rgb(0 0 0 / 0.5);
  }
  .name-glyph {
    width: 46px;
    height: 46px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 13px;
    color: var(--accent-ink);
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px rgb(var(--accent-rgb) / 0.25);
  }
  .name-glyph.small {
    width: 34px;
    height: 34px;
    border-radius: 10px;
  }
  .name-field {
    flex: 1;
  }
  .hint {
    margin: 10px 0 0 4px;
    font-size: 12px;
    color: var(--text-3);
  }

  .perm {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    margin-top: 26px;
    padding: 14px;
    border-radius: 16px;
    background: var(--surface-1);
    box-shadow:
      inset 0 0 0 1px var(--border),
      var(--highlight),
      0 20px 40px -24px rgb(0 0 0 / 0.5);
  }
  .perm-icon {
    width: 42px;
    height: 42px;
    display: grid;
    place-items: center;
    border-radius: 12px;
    color: var(--text-1);
    background: var(--fill-2);
    box-shadow: inset 0 0 0 1px var(--border);
    flex: none;
  }
  .perm.ok .perm-icon {
    color: var(--accent-ink);
    background: var(--accent-soft);
  }
  .perm-text {
    flex: 1;
    min-width: 0;
  }
  .perm-title {
    font-weight: 600;
  }
  .perm-state {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--text-2);
  }
  .perm-check {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: var(--accent-contrast);
    background: var(--accent);
    margin-right: 4px;
  }

  .firewall {
    width: 100%;
    margin-top: 24px;
    padding: 14px 16px;
    border-radius: 14px;
    background: var(--surface-1);
    box-shadow:
      inset 0 0 0 1px var(--border),
      var(--highlight),
      0 20px 40px -24px rgb(0 0 0 / 0.5);
    font-size: 12.5px;
    color: var(--text-2);
  }
  .fw-head {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    color: var(--text-1);
    margin-bottom: 10px;
  }
  .fw-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 5px 0;
  }
  .box {
    width: 16px;
    height: 16px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--border-heavy);
  }
  .fw-row.on {
    color: var(--text-1);
  }
  .fw-row.on .box {
    background: var(--accent);
    color: var(--accent-contrast);
    box-shadow: none;
  }

  .identity {
    width: 100%;
    margin-top: 24px;
    border-radius: 18px;
    background: var(--surface-1);
    box-shadow:
      inset 0 0 0 1px var(--border),
      var(--highlight),
      0 20px 40px -24px rgb(0 0 0 / 0.5);
    overflow: hidden;
  }
  .id-head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
  }
  .id-name {
    font-weight: 620;
  }
  .id-sub {
    font-size: 12px;
    color: var(--text-3);
  }
  dl {
    margin: 0;
    padding: 0 16px 12px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  dl div {
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--fill-1);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  dt {
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  dd {
    margin: 3px 0 0;
    font-size: 13px;
    color: var(--text-1);
  }
  .found {
    border-top: 1px solid var(--border);
    padding: 10px 16px;
  }
  .found-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 0;
  }
  .found-name {
    flex: 1;
    font-weight: 560;
  }
  .waiting {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 0;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .dots {
    position: absolute;
    bottom: 28px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    gap: 7px;
  }
  .dots button {
    width: 7px;
    height: 7px;
    border-radius: 999px;
    background: var(--fill-4);
    transition:
      width var(--dur-3) var(--ease),
      background var(--dur-3) var(--ease);
  }
  .dots button.done {
    background: rgb(var(--accent-rgb) / 0.45);
  }
  .dots button.active {
    width: 26px;
    background: var(--accent);
    box-shadow: 0 0 10px rgb(var(--accent-rgb) / 0.5);
  }

  @keyframes drift {
    0%,
    100% {
      transform: translateY(0) rotate(0deg);
    }
    50% {
      transform: translateY(-10px) rotate(-4deg);
    }
  }
  @keyframes pulse {
    0%,
    100% {
      transform: scale(0.95);
      opacity: 0.8;
    }
    50% {
      transform: scale(1.06);
      opacity: 1;
    }
  }
  @keyframes glint {
    0%,
    55% {
      background-position: 120% 0;
    }
    85%,
    100% {
      background-position: -30% 0;
    }
  }
  @keyframes fall {
    0% {
      transform: translateY(-6px) rotate(0deg);
      opacity: 0;
    }
    20% {
      opacity: 0.8;
    }
    100% {
      transform: translateY(26px) rotate(120deg);
      opacity: 0;
    }
  }

  @media (max-height: 680px) {
    .display {
      font-size: 62px;
    }
    .hero-mark {
      height: 150px;
    }
    .title {
      font-size: 38px;
    }
    .actions {
      margin-top: 26px;
    }
  }
</style>
