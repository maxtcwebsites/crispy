<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';

  interface Props extends Omit<HTMLInputAttributes, 'value' | 'size'> {
    value: string | number;
    mono?: boolean;
    invalid?: boolean;
    size?: 'md' | 'lg';
    input?: HTMLInputElement;
  }
  let { value = $bindable(), mono = false, invalid = false, size = 'md', input = $bindable(), class: cls = '', ...rest }: Props = $props();
</script>

<input bind:this={input} class="field {size} {cls}" class:mono class:invalid bind:value spellcheck="false" autocomplete="off" {...rest} />

<style>
  .field {
    height: 32px;
    width: 100%;
    min-width: 0;
    padding: 0 11px;
    border: 0;
    border-radius: var(--r-sm);
    background: var(--surface-sunken);
    color: var(--text-1);
    font-size: 13px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
    transition:
      box-shadow var(--dur-1) var(--ease),
      background var(--dur-1) var(--ease);
  }
  :global([data-theme='light']) .field {
    background: #fff;
  }
  .lg {
    height: 44px;
    padding: 0 14px;
    font-size: 15px;
    border-radius: 11px;
  }
  .field::placeholder {
    color: var(--text-4);
  }
  .field:hover {
    box-shadow: inset 0 0 0 1px var(--border-heavy);
  }
  .field:focus,
  .field:focus-visible {
    border-radius: var(--r-sm);
    box-shadow:
      inset 0 0 0 1px rgb(var(--accent-rgb) / 0.7),
      0 0 0 3px rgb(var(--accent-rgb) / 0.18);
  }
  .lg:focus {
    border-radius: 11px;
  }
  .invalid,
  .invalid:focus {
    box-shadow:
      inset 0 0 0 1px rgb(var(--red-rgb) / 0.7),
      0 0 0 3px rgb(var(--red-rgb) / 0.15);
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 12.5px;
  }
  .field:disabled {
    opacity: 0.5;
  }
</style>
