<script lang="ts">
  // Persistent notices at the top of every page: missing macOS permission, a failed input hook,
  // and engine warnings (dismissible).
  import ShieldAlert from '@lucide/svelte/icons/shield-alert';
  import { api } from '../api';
  import { store } from '../store.svelte';
  import Banner from './Banner.svelte';
  import Button from './Button.svelte';

  const perms = $derived(store.state?.permissions);
  const needsAccessibility = $derived(perms?.accessibility === 'denied');
  const captureFailed = $derived(!needsAccessibility && perms?.capture === 'failed');
  let checking = $state(false);

  async function checkAgain() {
    checking = true;
    await store.run(() => api.requestPermissions(), 'Couldn’t check permissions');
    checking = false;
  }
</script>

{#if needsAccessibility || captureFailed || store.warnings.length}
  <div class="notices">
    {#if needsAccessibility}
      <Banner
        level="accent"
        icon={ShieldAlert}
        title="Allow Crispy to use Accessibility"
        body="macOS needs your permission before Crispy can share this Mac’s keyboard and mouse. Turn on Crispy in Privacy & Security → Accessibility."
      >
        {#snippet actions()}
          <Button size="sm" variant="secondary" loading={checking} onclick={checkAgain}>Check again</Button>
          <Button size="sm" variant="primary" onclick={() => store.run(() => api.openPermissionSettings())}>Open System Settings</Button>
        {/snippet}
      </Banner>
    {:else if captureFailed}
      <Banner level="error" title="Keyboard and mouse sharing stopped" body={perms?.detail ?? 'The input hook could not start. Restarting Crispy usually fixes this.'}>
        {#snippet actions()}
          <Button size="sm" variant="secondary" onclick={() => store.run(() => api.openLogs())}>Open logs</Button>
        {/snippet}
      </Banner>
    {/if}
    {#each store.warnings as w (w.id)}
      <Banner level={w.level} title={w.title} body={w.body} ondismiss={() => store.dismissedWarnings.push(w.id)} />
    {/each}
  </div>
{/if}

<style>
  .notices {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: -8px 0 28px;
  }
</style>
