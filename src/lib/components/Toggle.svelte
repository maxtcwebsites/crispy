<script lang="ts">
  interface Props {
    checked: boolean;
    label?: string;
    labelledby?: string;
    describedby?: string;
    disabled?: boolean;
    id?: string;
    onchange?: (v: boolean) => void;
  }
  let { checked = $bindable(), label, labelledby, describedby, disabled = false, id, onchange }: Props = $props();

  function flip() {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }
</script>

<button
  {id}
  type="button"
  role="switch"
  class="toggle"
  class:on={checked}
  aria-checked={checked}
  aria-label={label}
  aria-labelledby={labelledby}
  aria-describedby={describedby}
  {disabled}
  onclick={flip}
>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    width: 36px;
    height: 21px;
    border-radius: 999px;
    flex: none;
    background: var(--fill-4);
    box-shadow: inset 0 0 0 1px var(--border), inset 0 1px 2px rgb(0 0 0 / 0.18);
    transition:
      background-color var(--dur-2) var(--ease),
      box-shadow var(--dur-2) var(--ease);
  }
  :global([data-theme='light']) .toggle {
    background: rgb(40 30 18 / 0.14);
  }
  .toggle.on {
    background: var(--accent);
    box-shadow:
      inset 0 0 0 1px rgb(0 0 0 / 0.05),
      inset 0 1px 0 rgb(255 255 255 / 0.2),
      0 0 14px -4px rgb(var(--accent-rgb) / 0.55);
  }
  .toggle:disabled {
    opacity: 0.4;
  }
  .knob {
    position: absolute;
    top: 2.5px;
    left: 2.5px;
    width: 16px;
    height: 16px;
    border-radius: 999px;
    background: var(--knob);
    box-shadow:
      0 1px 2px rgb(0 0 0 / 0.3),
      0 2px 6px -1px rgb(0 0 0 / 0.25);
    transition:
      transform 260ms var(--ease-spring),
      width 160ms var(--ease);
  }
  .on .knob {
    transform: translateX(15px);
  }
  .toggle:active:not(:disabled) .knob {
    width: 19px;
  }
  .on:active:not(:disabled) .knob {
    transform: translateX(12px);
  }
</style>
