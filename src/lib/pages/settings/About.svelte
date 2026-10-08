<script lang="ts">
  import { store } from '../../store.svelte';
  import { hotkeyParts, osName } from '../../util';
  import ChipMark from '../../components/ChipMark.svelte';
  import Keycap from '../../components/Keycap.svelte';
  import SettingsGroup from '../../components/SettingsGroup.svelte';

  const s = $derived(store.settings!);
  const os = $derived(store.os);
  const mod = $derived(os === 'macos' ? '⌘' : 'Ctrl');
  const keys = (h: Parameters<typeof hotkeyParts>[0]) => hotkeyParts(h, store.keys, os);

  const global = $derived([
    { label: 'Lock to this screen', keys: keys(s.hotkeys.lockToScreen) },
    { label: 'Bring cursor home', keys: keys(s.hotkeys.bringHome) },
    { label: 'Next computer', keys: keys(s.hotkeys.nextDevice) },
    { label: 'Pause sharing', keys: keys(s.hotkeys.pauseSharing) },
  ]);
  const inApp = $derived([
    { label: 'Devices', keys: [mod, '1'] },
    { label: 'Arrangement', keys: [mod, '2'] },
    { label: 'Transfers', keys: [mod, '3'] },
    { label: 'Settings', keys: [mod, ','] },
  ]);
</script>

<div class="hero">
  <div class="mark"><ChipMark size={104} label="Crispy" /></div>
  <h2 class="serif">Crispy</h2>
  <p class="tag">One keyboard. One mouse. Every computer.</p>
  <p class="version mono">Version {store.info?.version ?? '—'} · {osName(os)}</p>
  <p class="blurb">
    Crispy shares your keyboard, mouse, clipboard and files between computers on your network — peer to peer, end-to-end
    encrypted, no accounts and no servers. Push the cursor off the edge of one screen and keep going on the next.
  </p>
</div>

<SettingsGroup title="Shortcuts that work everywhere">
  {#each global as g (g.label)}
    <div class="sc">
      <span>{g.label}</span>
      <span class="caps">
        {#each g.keys as k, i (i)}<Keycap label={k} size="sm" />{:else}<span class="unset">Not set</span>{/each}
      </span>
    </div>
  {/each}
</SettingsGroup>

<SettingsGroup title="In this window">
  {#each inApp as g (g.label)}
    <div class="sc">
      <span>{g.label}</span>
      <span class="caps">{#each g.keys as k, i (i)}<Keycap label={k} size="sm" />{/each}</span>
    </div>
  {/each}
</SettingsGroup>

<SettingsGroup title="Moves">
  <div class="sc"><span>Switch computers</span><span class="move">Push the cursor across a glowing edge{s.switching.doubleTap ? ' twice' : ''}</span></div>
  <div class="sc"><span>Send files</span><span class="move">Drop them on a computer in Devices</span></div>
  <div class="sc"><span>Copy &amp; paste</span><span class="move">Just works, across every computer</span></div>
</SettingsGroup>

<p class="credit">Made with care, and a little salt.</p>

<style>
  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 12px 0 6px;
  }
  .mark {
    position: relative;
    animation: float 6s var(--ease-in-out) infinite;
  }
  .mark::before {
    content: '';
    position: absolute;
    inset: -20px;
    border-radius: 50%;
    background: radial-gradient(closest-side, rgb(243 181 68 / 0.28), transparent);
    z-index: -1;
  }
  h2 {
    font-size: 48px;
    line-height: 1;
    margin-top: 6px;
  }
  .tag {
    margin-top: 8px;
    font-size: 15px;
    color: var(--text-1);
  }
  .version {
    margin-top: 6px;
    font-size: 11.5px;
    color: var(--text-3);
  }
  .blurb {
    max-width: 52ch;
    margin-top: 14px;
    color: var(--text-2);
    line-height: 1.6;
  }
  .sc {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 16px;
    font-size: 13px;
  }
  .sc + .sc {
    border-top: 1px solid var(--border);
  }
  .caps {
    display: inline-flex;
    gap: 3px;
  }
  .move,
  .unset {
    color: var(--text-3);
    font-size: 12.5px;
    text-align: right;
  }
  .credit {
    text-align: center;
    color: var(--text-4);
    font-size: 12px;
    font-style: italic;
  }
  @keyframes float {
    0%,
    100% {
      transform: translateY(0) rotate(0);
    }
    50% {
      transform: translateY(-6px) rotate(-3deg);
    }
  }
</style>
