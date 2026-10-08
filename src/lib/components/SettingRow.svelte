<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    description?: string;
    /** Stack the control under the text (sliders, wide editors). */
    stacked?: boolean;
    id?: string;
    disabled?: boolean;
    children?: Snippet;
    extra?: Snippet;
  }
  let { label, description, stacked = false, id, disabled = false, children, extra }: Props = $props();
  const fallback = `row-${Math.random().toString(36).slice(2, 8)}`;
  const uid = $derived(id ?? fallback);
</script>

<div class="row" class:stacked class:disabled>
  <div class="text">
    <label class="label" id="{uid}-label" for={uid}>{label}</label>
    {#if description}<p class="desc" id="{uid}-desc">{description}</p>{/if}
    {#if extra}{@render extra()}{/if}
  </div>
  {#if children}<div class="control">{@render children()}</div>{/if}
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding: 14px 16px;
    min-height: 56px;
  }
  .row + :global(.row) {
    border-top: 1px solid var(--border);
  }
  .stacked {
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
  }
  .text {
    min-width: 0;
    flex: 1;
  }
  .label {
    display: block;
    font-size: 13.5px;
    font-weight: 540;
    color: var(--text-1);
  }
  .desc {
    margin-top: 2px;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--text-2);
    max-width: 52ch;
  }
  .control {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .stacked .control {
    flex: 1;
  }
  .disabled .text,
  .disabled .control {
    opacity: 0.45;
    pointer-events: none;
  }
</style>
