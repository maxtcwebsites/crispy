<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    subtitle?: Snippet;
    actions?: Snippet;
  }
  let { title, subtitle, actions }: Props = $props();
</script>

<header class="head">
  <div class="titles">
    <h1 class="serif">{title}</h1>
    {#if subtitle}<div class="sub">{@render subtitle()}</div>{/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
  .head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 26px;
  }
  .titles {
    min-width: 0;
  }
  h1 {
    font-size: var(--fs-hero);
    line-height: 1.02;
    font-style: italic;
    letter-spacing: -0.018em;
    color: var(--text-1);
  }
  .sub {
    margin-top: 9px;
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
    font-size: 13px;
    min-height: 20px;
  }
  @media (max-height: 680px) {
    .head {
      margin-bottom: 20px;
    }
    h1 {
      font-size: 34px;
    }
    .sub {
      margin-top: 6px;
    }
  }
  .actions {
    display: flex;
    gap: 8px;
    flex: none;
    padding-bottom: 2px;
  }
</style>
