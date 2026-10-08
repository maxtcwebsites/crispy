<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  import Info from '@lucide/svelte/icons/info';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import CircleX from '@lucide/svelte/icons/circle-x';
  import X from '@lucide/svelte/icons/x';
  import { rise } from '../motion';

  interface Props {
    level?: 'info' | 'warn' | 'error' | 'accent';
    title: string;
    body?: string;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    icon?: Component<any>;
    ondismiss?: () => void;
    actions?: Snippet;
  }
  let { level = 'info', title, body, icon, ondismiss, actions }: Props = $props();
  const Icon = $derived(icon ?? (level === 'error' ? CircleX : level === 'warn' ? TriangleAlert : Info));
</script>

<div class="banner {level}" role={level === 'error' ? 'alert' : 'status'} transition:rise={{ y: -6, duration: 220 }}>
  <span class="icon"><Icon size={17} strokeWidth={1.75} aria-hidden="true" /></span>
  <div class="text">
    <p class="title">{title}</p>
    {#if body}<p class="body">{body}</p>{/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
  {#if ondismiss}
    <button type="button" class="close" aria-label="Dismiss" onclick={ondismiss}>
      <X size={14} strokeWidth={1.75} aria-hidden="true" />
    </button>
  {/if}
</div>

<style>
  .banner {
    --tone: var(--blue-rgb);
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    border-radius: var(--r-lg);
    background:
      linear-gradient(90deg, rgb(var(--tone) / 0.1), rgb(var(--tone) / 0.04)),
      var(--surface-1);
    box-shadow:
      inset 0 0 0 1px rgb(var(--tone) / 0.22),
      var(--highlight);
  }
  .warn {
    --tone: var(--amber-rgb);
  }
  .error {
    --tone: var(--red-rgb);
  }
  .accent {
    --tone: var(--accent-rgb);
  }
  .icon {
    flex: none;
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: 9px;
    color: rgb(var(--tone));
    background: rgb(var(--tone) / 0.14);
    align-self: flex-start;
  }
  :global([data-theme='light']) .accent .icon {
    color: var(--accent-deep);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-weight: 600;
    font-size: 13px;
  }
  .body {
    margin-top: 1px;
    color: var(--text-2);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .actions {
    display: flex;
    gap: 6px;
    flex: none;
  }
  .close {
    flex: none;
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 7px;
    color: var(--text-3);
  }
  .close:hover {
    background: var(--fill-2);
    color: var(--text-1);
  }
  @media (max-width: 1000px) {
    .banner {
      flex-wrap: wrap;
    }
    .actions {
      margin-left: 44px;
    }
  }
</style>
