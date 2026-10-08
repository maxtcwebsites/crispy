<script lang="ts">
  // "On a Mac keyboard typing on Windows, ⌘ acts as [Ctrl ▾]" — for Ctrl, Alt and Meta.
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import type { ModKey, ModifierMap } from '../types';
  import Keycap from './Keycap.svelte';
  import Segmented from './Segmented.svelte';
  import Select from './Select.svelte';

  interface Props {
    value: ModifierMap;
    /** Which keyboard is doing the typing. */
    from: 'mac' | 'pc';
  }
  let { value = $bindable(), from }: Props = $props();

  const MAC: Record<ModKey, { cap: string; name: string }> = {
    ctrl: { cap: '⌃', name: 'Control' },
    alt: { cap: '⌥', name: 'Option' },
    meta: { cap: '⌘', name: 'Command' },
    shift: { cap: '⇧', name: 'Shift' },
  };
  const PC: Record<ModKey, { cap: string; name: string }> = {
    ctrl: { cap: 'Ctrl', name: 'Control' },
    alt: { cap: 'Alt', name: 'Alt' },
    meta: { cap: 'Win', name: 'Windows / Super' },
    shift: { cap: 'Shift', name: 'Shift' },
  };
  const src = $derived(from === 'mac' ? MAC : PC);
  const dst = $derived(from === 'mac' ? PC : MAC);
  const rows: (keyof ModifierMap)[] = $derived(from === 'mac' ? ['ctrl', 'alt', 'meta'] : ['ctrl', 'alt', 'meta']);
  const targetOptions = $derived(
    (['ctrl', 'alt', 'meta', 'shift'] as ModKey[]).map((k) => ({
      value: k,
      label: from === 'mac' ? dst[k].cap : `${dst[k].cap} ${dst[k].name}`,
    })),
  );

  const SWAP: ModifierMap = { ctrl: 'meta', alt: 'alt', meta: 'ctrl' };
  const SAME: ModifierMap = { ctrl: 'ctrl', alt: 'alt', meta: 'meta' };
  const same = (a: ModifierMap, b: ModifierMap) => a.ctrl === b.ctrl && a.alt === b.alt && a.meta === b.meta;
  const preset = $derived(same(value, SWAP) ? 'swap' : same(value, SAME) ? 'same' : 'custom');

  function choosePreset(p: string) {
    if (p === 'swap') value = { ...SWAP };
    else if (p === 'same') value = { ...SAME };
  }
</script>

<div class="editor">
  <div class="presets">
    <Segmented
      value={preset}
      label="Preset"
      size="sm"
      full
      options={[
        { value: 'swap', label: from === 'mac' ? 'Shortcut-friendly (⌘ ⇄ Ctrl)' : 'Shortcut-friendly (Ctrl ⇄ ⌘)' },
        { value: 'same', label: 'Physical' },
        ...(preset === 'custom' ? [{ value: 'custom', label: 'Custom' }] : []),
      ]}
      onchange={choosePreset}
    />
  </div>
  <ul>
    {#each rows as k (k)}
      <li>
        <span class="src">
          <Keycap label={src[k].cap} size="lg" />
          <span class="src-name">{src[k].name}</span>
        </span>
        <span class="arrow" aria-hidden="true"><ArrowRight size={14} strokeWidth={1.75} /></span>
        <span class="acts">acts as</span>
        <Select value={value[k]} label="{src[k].name} acts as" options={targetOptions} width={from === 'mac' ? 110 : 150} onchange={(v) => (value = { ...value, [k]: v })} />
      </li>
    {/each}
  </ul>
</div>

<style>
  .editor {
    padding: 4px 16px 14px;
  }
  .presets {
    padding: 10px 0 12px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
  }
  li + li {
    border-top: 1px dashed var(--border);
  }
  .src {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    flex: 1;
    min-width: 0;
  }
  .src-name {
    font-size: 13px;
    font-weight: 540;
    color: var(--text-1);
  }
  .arrow {
    color: var(--text-4);
    display: none;
  }
  .acts {
    font-size: 12px;
    color: var(--text-3);
  }
</style>
