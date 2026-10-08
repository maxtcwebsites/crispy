<script lang="ts">
  interface Props {
    label: string;
    size?: 'sm' | 'md' | 'lg';
    active?: boolean;
  }
  let { label, size = 'md', active = false }: Props = $props();
</script>

<kbd class="cap {size}" class:active class:wide={label.length > 2}>{label}</kbd>

<style>
  .cap {
    display: inline-grid;
    place-items: center;
    font-family: var(--font-sans);
    font-weight: 600;
    color: var(--text-1);
    background: linear-gradient(180deg, var(--surface-3), var(--surface-2));
    border-radius: 6px;
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      inset 0 -1.5px 0 rgb(0 0 0 / 0.28),
      var(--highlight-strong);
    line-height: 1;
    white-space: nowrap;
    transition:
      background var(--dur-1) var(--ease),
      color var(--dur-1) var(--ease),
      box-shadow var(--dur-1) var(--ease);
  }
  :global([data-theme='light']) .cap {
    background: linear-gradient(180deg, #fff, #f6f3ee);
    box-shadow:
      inset 0 0 0 1px var(--border-strong),
      inset 0 -1.5px 0 rgb(40 30 18 / 0.1),
      0 1px 1px rgb(40 30 18 / 0.05);
  }
  .sm {
    min-width: 18px;
    height: 18px;
    padding: 0 4px;
    font-size: 10.5px;
    border-radius: 5px;
  }
  .md {
    min-width: 24px;
    height: 24px;
    padding: 0 6px;
    font-size: 12px;
  }
  .lg {
    min-width: 34px;
    height: 34px;
    padding: 0 9px;
    font-size: 14px;
    border-radius: 8px;
  }
  .wide.md {
    padding: 0 8px;
  }
  .active {
    color: var(--accent-contrast);
    background: var(--accent);
    box-shadow:
      inset 0 -1.5px 0 rgb(0 0 0 / 0.18),
      inset 0 1px 0 rgb(255 255 255 / 0.35);
  }
</style>
