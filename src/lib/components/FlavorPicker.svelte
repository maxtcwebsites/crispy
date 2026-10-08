<script lang="ts">
  import { tooltip } from '../actions';
  import { FLAVORS } from '../flavors';
  import type { Flavor } from '../types';

  interface Props {
    value: Flavor;
    id?: string;
  }
  let { value = $bindable(), id }: Props = $props();
  let buttons: HTMLButtonElement[] = $state([]);

  function keydown(e: KeyboardEvent) {
    const i = FLAVORS.findIndex((f) => f.id === value);
    let n = i;
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') n = (i + 1) % FLAVORS.length;
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') n = (i - 1 + FLAVORS.length) % FLAVORS.length;
    else return;
    e.preventDefault();
    value = FLAVORS[n].id;
    buttons[n]?.focus();
  }
</script>

<div class="flavors" role="radiogroup" aria-label="Flavor" {id} tabindex="-1" onkeydown={keydown}>
  {#each FLAVORS as f, i (f.id)}
    <button
      bind:this={buttons[i]}
      type="button"
      role="radio"
      aria-checked={value === f.id}
      aria-label={f.name}
      tabindex={value === f.id ? 0 : -1}
      class="swatch"
      class:selected={value === f.id}
      style="--c: {f.color}; --d: {f.dust}"
      onclick={() => (value = f.id)}
      use:tooltip={f.name}
    >
      <svg viewBox="0 0 32 32" width="30" height="30" aria-hidden="true">
        <defs>
          <radialGradient id="fl-{f.id}" cx="0.35" cy="0.3" r="0.85">
            <stop offset="0" stop-color="#fff" stop-opacity="0.55" />
            <stop offset="0.45" stop-color="#fff" stop-opacity="0" />
          </radialGradient>
        </defs>
        <path d="M4.6 16.2c-.3-5.2 4.7-9.4 11.1-9.9 6.9-.5 11.8 2.6 11.9 7.6.1 5.9-5.6 11.4-12.4 11.6-5.9.2-10.3-3.6-10.6-9.3Z" fill="var(--c)" />
        <path d="M4.6 16.2c-.3-5.2 4.7-9.4 11.1-9.9 6.9-.5 11.8 2.6 11.9 7.6.1 5.9-5.6 11.4-12.4 11.6-5.9.2-10.3-3.6-10.6-9.3Z" fill="url(#fl-{f.id})" />
        <path d="M8 13.4c4.6-2.3 10.6-2.9 16.8-1.6M7.6 17.6c4.9-2.1 10.7-2.4 16.5-1" fill="none" stroke="#fff" stroke-opacity="0.4" stroke-width="1.1" stroke-linecap="round" />
        <circle cx="12" cy="20.5" r="0.9" fill="var(--d)" opacity="0.85" />
        <circle cx="19.5" cy="9.6" r="0.75" fill="var(--d)" opacity="0.85" />
        <circle cx="22.4" cy="19" r="0.8" fill="var(--d)" opacity="0.85" />
      </svg>
    </button>
  {/each}
</div>

<style>
  .flavors {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .swatch {
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: 12px;
    background: var(--fill-1);
    box-shadow: inset 0 0 0 1px var(--border);
    transition:
      transform 180ms var(--ease-spring),
      box-shadow var(--dur-2) var(--ease),
      background var(--dur-2) var(--ease);
  }
  .swatch svg {
    filter: drop-shadow(0 2px 3px rgb(0 0 0 / 0.3));
    transition: transform 220ms var(--ease-spring);
  }
  .swatch:hover {
    background: var(--fill-2);
  }
  .swatch:hover svg {
    transform: rotate(-8deg) scale(1.06);
  }
  .swatch:active {
    transform: scale(0.94);
  }
  .swatch.selected {
    background: var(--fill-2);
    box-shadow:
      inset 0 0 0 1.5px var(--c),
      0 0 16px -6px var(--c);
  }
  .swatch:focus-visible {
    border-radius: 12px;
  }
</style>
