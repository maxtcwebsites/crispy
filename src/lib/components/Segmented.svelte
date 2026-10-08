<script lang="ts" generics="T extends string">
  import type { Component } from 'svelte';

  interface Option {
    value: T;
    label: string;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    icon?: Component<any>;
  }
  interface Props {
    value: T;
    options: Option[];
    label: string;
    size?: 'sm' | 'md';
    full?: boolean;
    onchange?: (v: T) => void;
  }
  let { value = $bindable(), options, label, size = 'md', full = false, onchange }: Props = $props();

  const index = $derived(Math.max(0, options.findIndex((o) => o.value === value)));
  let buttons: HTMLButtonElement[] = $state([]);

  function pick(i: number) {
    value = options[i].value;
    onchange?.(value);
  }

  function keydown(e: KeyboardEvent) {
    let i = index;
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') i = (index + 1) % options.length;
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') i = (index - 1 + options.length) % options.length;
    else return;
    e.preventDefault();
    pick(i);
    buttons[i]?.focus();
  }
</script>

<div
  class="seg {size}"
  class:full
  role="radiogroup"
  aria-label={label}
  tabindex="-1"
  style="--n: {options.length}; --i: {index}"
  onkeydown={keydown}
>
  <span class="indicator" aria-hidden="true"></span>
  {#each options as o, i (o.value)}
    <button
      bind:this={buttons[i]}
      type="button"
      role="radio"
      aria-checked={i === index}
      tabindex={i === index ? 0 : -1}
      class:selected={i === index}
      onclick={() => pick(i)}
    >
      {#if o.icon}
        {@const Icon = o.icon}
        <Icon size={14} strokeWidth={1.75} aria-hidden="true" />
      {/if}
      <span>{o.label}</span>
    </button>
  {/each}
</div>

<style>
  .seg {
    position: relative;
    display: inline-grid;
    grid-template-columns: repeat(var(--n), minmax(0, 1fr));
    padding: 2px;
    border-radius: 9px;
    background: var(--fill-2);
    box-shadow: inset 0 0 0 1px var(--border);
    flex: none;
  }
  .full {
    display: grid;
    width: 100%;
  }
  .indicator {
    position: absolute;
    top: 2px;
    bottom: 2px;
    left: 2px;
    width: calc((100% - 4px) / var(--n));
    transform: translateX(calc(var(--i) * 100%));
    border-radius: 7px;
    background: var(--surface-4);
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      var(--highlight-strong),
      0 1px 3px rgb(0 0 0 / 0.3);
    transition: transform var(--dur-3) var(--ease);
  }
  :global([data-theme='light']) .indicator {
    background: #fff;
    box-shadow:
      0 0 0 0.5px rgb(40 30 18 / 0.12),
      0 1px 3px rgb(60 40 10 / 0.12);
  }
  button {
    position: relative;
    z-index: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 28px;
    padding: 0 12px;
    border-radius: 7px;
    font-size: 12.5px;
    font-weight: 540;
    color: var(--text-2);
    white-space: nowrap;
    transition: color var(--dur-1) var(--ease);
  }
  .sm button {
    height: 24px;
    padding: 0 9px;
    font-size: 12px;
  }
  button:hover {
    color: var(--text-1);
  }
  button.selected {
    color: var(--text-1);
  }
  button:focus-visible {
    border-radius: 7px;
  }
</style>
