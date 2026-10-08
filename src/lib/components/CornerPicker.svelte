<script lang="ts">
  // A little screen with four clickable corners: where the cursor should NOT switch computers.
  import type { Settings } from '../types';

  type Corners = Settings['switching']['corners'];
  interface Props {
    corners: Corners;
    size: number;
    disabled?: boolean;
  }
  let { corners = $bindable(), size, disabled = false }: Props = $props();

  const list: { key: keyof Corners; label: string }[] = [
    { key: 'topLeft', label: 'Top left' },
    { key: 'topRight', label: 'Top right' },
    { key: 'bottomLeft', label: 'Bottom left' },
    { key: 'bottomRight', label: 'Bottom right' },
  ];
  // 0–200 px on a real screen → 15–41 px in the diagram.
  const zone = $derived(size <= 0 ? 0 : 15 + Math.min(1, size / 200) * 26);
</script>

<div class="screen" class:disabled role="group" aria-label="Corners that never switch">
  <span class="menubar" aria-hidden="true"></span>
  {#each list as c (c.key)}
    <button
      type="button"
      class="corner {c.key}"
      class:on={corners[c.key]}
      aria-pressed={corners[c.key]}
      aria-label="{c.label} corner"
      {disabled}
      style="--z: {zone}px"
      onclick={() => (corners = { ...corners, [c.key]: !corners[c.key] })}
    >
      <span class="fill"></span>
    </button>
  {/each}
  <span class="caption" aria-hidden="true">{size > 0 ? `${size} px` : 'Off'}</span>
</div>

<style>
  .screen {
    position: relative;
    width: 176px;
    height: 110px;
    border-radius: 10px;
    background:
      linear-gradient(150deg, rgb(255 255 255 / 0.06), transparent 40%),
      linear-gradient(180deg, var(--glass-top), var(--glass-bottom));
    box-shadow:
      inset 0 0 0 1px var(--glass-border),
      0 10px 24px -14px rgb(0 0 0 / 0.6);
    flex: none;
  }
  .disabled {
    opacity: 0.45;
  }
  .menubar {
    position: absolute;
    inset: 0 0 auto;
    height: 6px;
    border-radius: 10px 10px 0 0;
    background: var(--fill-2);
  }
  .corner {
    position: absolute;
    width: 42px;
    height: 42px;
    display: block;
  }
  .corner:focus-visible {
    box-shadow: none;
  }
  .corner:focus-visible .fill {
    outline: 2px solid rgb(var(--accent-rgb) / 0.7);
    outline-offset: 2px;
  }
  .fill {
    position: absolute;
    width: max(var(--z), 12px);
    height: max(var(--z), 12px);
    border: 1.5px dashed var(--border-heavy);
    background: transparent;
    transition:
      width var(--dur-3) var(--ease),
      height var(--dur-3) var(--ease),
      background var(--dur-2) var(--ease),
      border-color var(--dur-2) var(--ease);
  }
  .corner:hover .fill {
    border-color: rgb(var(--accent-rgb) / 0.6);
  }
  .on .fill {
    border: 1.5px solid rgb(var(--accent-rgb) / 0.8);
    background: rgb(var(--accent-rgb) / 0.26);
    box-shadow: 0 0 14px -4px rgb(var(--accent-rgb) / 0.7);
  }
  .topLeft {
    top: 0;
    left: 0;
  }
  .topLeft .fill {
    top: 0;
    left: 0;
    border-radius: 10px 0 8px 0;
  }
  .topRight {
    top: 0;
    right: 0;
  }
  .topRight .fill {
    top: 0;
    right: 0;
    border-radius: 0 10px 0 8px;
  }
  .bottomLeft {
    bottom: 0;
    left: 0;
  }
  .bottomLeft .fill {
    bottom: 0;
    left: 0;
    border-radius: 0 8px 0 10px;
  }
  .bottomRight {
    bottom: 0;
    right: 0;
  }
  .bottomRight .fill {
    bottom: 0;
    right: 0;
    border-radius: 8px 0 10px 0;
  }
  .caption {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 11px;
    font-weight: 560;
    color: var(--text-3);
    pointer-events: none;
  }
</style>
