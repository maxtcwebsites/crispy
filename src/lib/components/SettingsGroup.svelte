<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title?: string;
    note?: string;
    children: Snippet;
  }
  let { title, note, children }: Props = $props();
</script>

<section class="group">
  {#if title}<h3 class="eyebrow">{title}</h3>{/if}
  <div class="card">{@render children()}</div>
  {#if note}<p class="note">{note}</p>{/if}
</section>

<style>
  h3 {
    margin: 0 0 9px 4px;
  }
  .card {
    border-radius: var(--r-lg);
    background: var(--surface-1);
    box-shadow:
      inset 0 0 0 1px var(--border),
      var(--highlight);
  }
  :global([data-theme='light']) .card {
    box-shadow:
      0 0 0 1px var(--border),
      0 1px 2px rgb(60 40 10 / 0.04);
  }
  .note {
    margin: 8px 4px 0;
    font-size: 12px;
    color: var(--text-3);
    line-height: 1.5;
  }
</style>
