<script lang="ts" generics="T extends string">
  import ChevronsUpDown from '@lucide/svelte/icons/chevrons-up-down';
  import Check from '@lucide/svelte/icons/check';
  import { outside, portal } from '../actions';
  import { rise } from '../motion';
  import { placeBelow } from '../popover';

  interface Option {
    value: T;
    label: string;
    hint?: string;
  }
  interface Props {
    value: T;
    options: Option[];
    label: string;
    id?: string;
    width?: number;
    disabled?: boolean;
    onchange?: (v: T) => void;
  }
  let { value = $bindable(), options, label, id, width, disabled = false, onchange }: Props = $props();

  const uid = `sel-${Math.random().toString(36).slice(2, 8)}`;
  let open = $state(false);
  let active = $state(0);
  let trigger: HTMLButtonElement | undefined = $state();
  let pos = $state({ left: 0, top: 0, width: 0, up: false });

  const index = $derived(Math.max(0, options.findIndex((o) => o.value === value)));
  const current = $derived(options[index]);
  const hasHints = $derived(options.some((o) => o.hint));

  function show() {
    if (disabled || !trigger) return;
    active = index;
    const r = trigger.getBoundingClientRect();
    const h = options.length * (hasHints ? 46 : 32) + 10;
    const w = Math.max(r.width, hasHints ? 260 : 180);
    pos = { ...placeBelow(r, { width: w, height: h }, 'start', 6), width: w };
    open = true;
  }
  function hide() {
    open = false;
  }
  function choose(i: number) {
    value = options[i].value;
    onchange?.(value);
    hide();
    trigger?.focus();
  }
  function keydown(e: KeyboardEvent) {
    if (!open) {
      if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(e.key)) {
        e.preventDefault();
        show();
      }
      return;
    }
    if (e.key === 'ArrowDown') active = (active + 1) % options.length;
    else if (e.key === 'ArrowUp') active = (active - 1 + options.length) % options.length;
    else if (e.key === 'Home') active = 0;
    else if (e.key === 'End') active = options.length - 1;
    else if (e.key === 'Enter' || e.key === ' ') choose(active);
    else if (e.key === 'Escape' || e.key === 'Tab') {
      if (e.key === 'Escape') e.stopPropagation();
      hide();
      if (e.key === 'Tab') return;
    } else return;
    e.preventDefault();
  }
</script>

<button
  bind:this={trigger}
  {id}
  type="button"
  class="trigger"
  role="combobox"
  class:open
  style:width={width ? `${width}px` : undefined}
  aria-label={label}
  aria-haspopup="listbox"
  aria-expanded={open}
  aria-controls={open ? `${uid}-list` : undefined}
  aria-activedescendant={open ? `${uid}-${active}` : undefined}
  {disabled}
  onclick={() => (open ? hide() : show())}
  onkeydown={keydown}
>
  <span class="truncate">{current?.label}</span>
  <ChevronsUpDown size={14} strokeWidth={1.75} aria-hidden="true" />
</button>

{#if open}
  <div
    id="{uid}-list"
    class="pop"
    role="listbox"
    aria-label={label}
    style:left="{pos.left}px"
    style:top="{pos.top}px"
    style:min-width="{pos.width}px"
    use:portal
    use:outside={{ onoutside: hide, ignore: trigger }}
    transition:rise={{ y: pos.up ? 4 : -4, scale: 0.98, duration: 180 }}
  >
    {#each options as o, i (o.value)}
      <div
        id="{uid}-{i}"
        role="option"
        tabindex="-1"
        aria-selected={i === index}
        class="opt"
        class:active={i === active}
        onpointerenter={() => (active = i)}
        onclick={() => choose(i)}
        onkeydown={() => {}}
      >
        <span class="check">
          {#if i === index}<Check size={14} strokeWidth={2} aria-hidden="true" />{/if}
        </span>
        <span class="text">
          <span class="lbl">{o.label}</span>
          {#if o.hint}<span class="hint">{o.hint}</span>{/if}
        </span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .trigger {
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    height: 30px;
    min-width: 150px;
    padding: 0 9px 0 11px;
    border-radius: var(--r-sm);
    font-size: 13px;
    font-weight: 520;
    color: var(--text-1);
    background: var(--surface-3);
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      var(--highlight);
    transition:
      background var(--dur-1) var(--ease),
      box-shadow var(--dur-1) var(--ease);
  }
  .trigger :global(svg) {
    color: var(--text-3);
    flex: none;
  }
  .trigger:hover:not(:disabled),
  .trigger.open {
    background: var(--surface-4);
  }
  :global([data-theme='light']) .trigger {
    background: var(--surface-1);
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      0 1px 2px rgb(60 40 10 / 0.05);
  }
  :global([data-theme='light']) .trigger:hover:not(:disabled),
  :global([data-theme='light']) .trigger.open {
    background: var(--surface-2);
  }
  .trigger:disabled {
    opacity: 0.45;
  }

  .pop {
    position: fixed;
    z-index: 900;
    padding: 5px;
    border-radius: 12px;
    background: var(--surface-float);
    box-shadow: var(--shadow-float);
    max-height: 320px;
    overflow: auto;
  }
  .opt {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 6px 10px 6px 6px;
    border-radius: 7px;
    font-size: 13px;
    color: var(--text-1);
  }
  .opt.active {
    background: var(--fill-3);
  }
  .check {
    width: 18px;
    height: 20px;
    display: grid;
    place-items: center;
    color: var(--accent-ink);
    flex: none;
  }
  .text {
    display: flex;
    flex-direction: column;
    line-height: 20px;
  }
  .hint {
    font-size: 11.5px;
    line-height: 1.35;
    color: var(--text-3);
    margin-bottom: 2px;
  }
</style>
