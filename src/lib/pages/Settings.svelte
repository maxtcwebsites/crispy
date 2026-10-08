<script lang="ts">
  import type { Component } from 'svelte';
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal';
  import ArrowLeftRight from '@lucide/svelte/icons/arrow-left-right';
  import MousePointer2 from '@lucide/svelte/icons/mouse-pointer-2';
  import KeyboardIcon from '@lucide/svelte/icons/keyboard';
  import Command from '@lucide/svelte/icons/command';
  import ClipboardIcon from '@lucide/svelte/icons/clipboard';
  import Folder from '@lucide/svelte/icons/folder';
  import ShieldCheck from '@lucide/svelte/icons/shield-check';
  import Wrench from '@lucide/svelte/icons/wrench';
  import Info from '@lucide/svelte/icons/info';
  import Check from '@lucide/svelte/icons/check';
  import CircleAlert from '@lucide/svelte/icons/circle-alert';
  import { rise } from '../motion';
  import { store, type SectionId } from '../store.svelte';
  import PageHeader from '../components/PageHeader.svelte';
  import Spinner from '../components/Spinner.svelte';
  import General from './settings/General.svelte';
  import Switching from './settings/Switching.svelte';
  import Mouse from './settings/Mouse.svelte';
  import Keyboard from './settings/Keyboard.svelte';
  import Shortcuts from './settings/Shortcuts.svelte';
  import Clipboard from './settings/Clipboard.svelte';
  import Files from './settings/Files.svelte';
  import Network from './settings/Network.svelte';
  import Advanced from './settings/Advanced.svelte';
  import About from './settings/About.svelte';

  interface Section {
    id: SectionId;
    label: string;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    icon: Component<any>;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    view: Component<any>;
    title?: string;
    description: string;
  }

  const sections: Section[] = [
    { id: 'general', label: 'General', icon: SlidersHorizontal, view: General, description: 'Your computer’s name, how Crispy looks and how it starts.' },
    { id: 'switching', label: 'Switching', icon: ArrowLeftRight, view: Switching, description: 'When the cursor moves on to the next computer — and when it shouldn’t.' },
    { id: 'mouse', label: 'Mouse & Scrolling', icon: MousePointer2, view: Mouse, description: 'Pointer and scroll behaviour on the computers you control.' },
    { id: 'keyboard', label: 'Keyboard', icon: KeyboardIcon, view: Keyboard, description: 'Keep your muscle memory when typing between a Mac and a PC.' },
    { id: 'shortcuts', label: 'Shortcuts', icon: Command, view: Shortcuts, description: 'Global shortcuts that work from whichever keyboard you’re using.' },
    { id: 'clipboard', label: 'Clipboard', icon: ClipboardIcon, view: Clipboard, description: 'Copy on one computer, paste on another.' },
    { id: 'files', label: 'Files', icon: Folder, view: Files, description: 'Where received files go and how transfers behave.' },
    { id: 'network', label: 'Network & Security', icon: ShieldCheck, view: Network, description: 'How computers find each other, and which ones you trust.' },
    { id: 'advanced', label: 'Advanced', icon: Wrench, view: Advanced, description: 'Diagnostics for when something isn’t quite right.' },
    { id: 'about', label: 'About', icon: Info, view: About, title: 'About Crispy', description: '' },
  ];

  const current = $derived(sections.find((s) => s.id === store.section) ?? sections[0]);
  let buttons: HTMLButtonElement[] = $state([]);

  function keydown(e: KeyboardEvent) {
    const i = sections.findIndex((s) => s.id === current.id);
    let n = i;
    if (e.key === 'ArrowDown') n = Math.min(sections.length - 1, i + 1);
    else if (e.key === 'ArrowUp') n = Math.max(0, i - 1);
    else return;
    e.preventDefault();
    store.section = sections[n].id;
    buttons[n]?.focus();
  }
</script>

<PageHeader title="Settings">
  {#snippet subtitle()}
    <span>Changes are saved as you make them.</span>
  {/snippet}
  {#snippet actions()}
    <div class="save" role="status" aria-live="polite">
      {#if store.saveStatus === 'saving'}
        <span class="pill" in:rise={{ y: 3 }}><Spinner size={12} /> Saving…</span>
      {:else if store.saveStatus === 'saved'}
        <span class="pill ok" in:rise={{ y: 3 }}><Check size={13} strokeWidth={2.4} aria-hidden="true" /> Saved</span>
      {:else if store.saveStatus === 'error'}
        <span class="pill err" in:rise={{ y: 3 }}><CircleAlert size={13} strokeWidth={2} aria-hidden="true" /> Not saved</span>
      {/if}
    </div>
  {/snippet}
</PageHeader>

<div class="layout">
  <nav class="subnav" aria-label="Settings sections">
    {#each sections as s, i (s.id)}
      {@const Icon = s.icon}
      <button
        bind:this={buttons[i]}
        type="button"
        class:active={current.id === s.id}
        class:about={s.id === 'about'}
        aria-current={current.id === s.id ? 'page' : undefined}
        onclick={() => (store.section = s.id)}
        onkeydown={keydown}
      >
        <Icon size={15} strokeWidth={1.6} aria-hidden="true" />
        <span class="truncate">{s.label}</span>
      </button>
    {/each}
  </nav>

  <div class="content">
    {#key current.id}
      <div in:rise={{ y: 6, duration: 240 }}>
        {#if current.id !== 'about'}
          <header class="section-head">
            <h2>{current.title ?? current.label}</h2>
            <p>{current.description}</p>
          </header>
        {/if}
        {#if store.settings}
          {@const View = current.view}
          <div class="groups"><View /></div>
        {/if}
      </div>
    {/key}
  </div>
</div>

<style>
  .layout {
    display: flex;
    align-items: flex-start;
    gap: 32px;
  }
  .subnav {
    position: sticky;
    top: 0;
    width: 188px;
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .subnav button {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 10px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 520;
    color: var(--text-2);
    text-align: left;
    transition:
      background var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease);
  }
  .subnav button :global(svg) {
    flex: none;
    color: var(--text-3);
  }
  .subnav button:hover {
    color: var(--text-1);
    background: var(--fill-1);
  }
  .subnav button.active {
    color: var(--text-1);
    background: var(--fill-2);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  :global([data-theme='light']) .subnav button.active {
    background: #fff;
    box-shadow:
      0 0 0 1px var(--border),
      0 1px 2px rgb(60 40 10 / 0.05);
  }
  .subnav button.active :global(svg) {
    color: var(--accent-ink);
  }
  .subnav .about {
    margin-top: 10px;
    position: relative;
  }
  .subnav .about::before {
    content: '';
    position: absolute;
    top: -6px;
    left: 10px;
    right: 10px;
    height: 1px;
    background: var(--border);
  }
  .content {
    flex: 1;
    min-width: 0;
    max-width: 680px;
  }
  .section-head {
    margin: 2px 0 22px 4px;
  }
  .section-head h2 {
    font-size: 19px;
    font-weight: 650;
    letter-spacing: -0.018em;
  }
  .section-head p {
    margin-top: 3px;
    color: var(--text-2);
  }
  .groups {
    display: flex;
    flex-direction: column;
    gap: 26px;
  }

  .save {
    min-width: 96px;
    display: flex;
    justify-content: flex-end;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 10px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 560;
    color: var(--text-2);
    background: var(--fill-2);
  }
  .pill.ok :global(svg) {
    color: var(--green);
  }
  .pill.err {
    color: var(--red);
    background: rgb(var(--red-rgb) / 0.1);
  }

  @media (max-width: 1000px) {
    .layout {
      gap: 22px;
    }
    .subnav {
      width: 156px;
    }
    .subnav button {
      font-size: 12.5px;
      gap: 8px;
      padding: 0 8px;
    }
  }
</style>
