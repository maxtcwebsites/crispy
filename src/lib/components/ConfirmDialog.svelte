<script lang="ts">
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import { store } from '../store.svelte';
  import Button from './Button.svelte';
  import Modal from './Modal.svelte';

  const req = $derived(store.confirmReq);
</script>

<Modal open={!!req} width={400} labelledby="confirm-title" describedby="confirm-body" onclose={() => store.answer(false)} showClose={false} initialFocus=".confirm-cancel">
  {#if req}
    <div class="body">
      {#if req.danger}
        <div class="icon"><TriangleAlert size={20} strokeWidth={1.75} aria-hidden="true" /></div>
      {/if}
      <h2 id="confirm-title">{req.title}</h2>
      {#if req.body}<p id="confirm-body">{req.body}</p>{/if}
    </div>
    <div class="actions">
      <Button variant="secondary" class="confirm-cancel" onclick={() => store.answer(false)}>{req.cancel ?? 'Cancel'}</Button>
      <Button variant={req.danger ? 'danger' : 'primary'} onclick={() => store.answer(true)}>{req.confirm ?? 'Continue'}</Button>
    </div>
  {/if}
</Modal>

<style>
  .body {
    padding: 26px 26px 6px;
  }
  .icon {
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: 12px;
    margin-bottom: 14px;
    color: var(--red);
    background: rgb(var(--red-rgb) / 0.12);
    box-shadow: inset 0 0 0 1px rgb(var(--red-rgb) / 0.2);
  }
  h2 {
    font-size: 16px;
    font-weight: 620;
    letter-spacing: -0.012em;
  }
  p {
    margin-top: 6px;
    color: var(--text-2);
    line-height: 1.5;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 20px 26px 22px;
  }
</style>
