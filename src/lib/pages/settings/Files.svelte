<script lang="ts">
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import Folder from '@lucide/svelte/icons/folder';
  import { api, pickFolder } from '../../api';
  import { store } from '../../store.svelte';
  import { prettyPath } from '../../util';
  import Button from '../../components/Button.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import SettingRow from '../../components/SettingRow.svelte';
  import SettingsGroup from '../../components/SettingsGroup.svelte';
  import Slider from '../../components/Slider.svelte';
  import Toggle from '../../components/Toggle.svelte';

  const s = $derived(store.settings!);
  const effective = $derived(s.files.saveDir || store.info?.saveDir || '');
  const custom = $derived(!!s.files.saveDir);

  async function choose() {
    const dir = await pickFolder(effective || undefined);
    if (dir) s.files.saveDir = dir;
  }
  const speed = (v: number) => (v === 0 ? 'Unlimited' : `${v} MB/s`);
</script>

<SettingsGroup title="Received files">
  <div class="folder">
    <span class="icon" aria-hidden="true"><Folder size={18} strokeWidth={1.6} /></span>
    <div class="path">
      <p class="lbl">Save to</p>
      <p class="mono selectable truncate" title={effective}>{prettyPath(effective)}</p>
      {#if !custom}<p class="default">Default location</p>{/if}
    </div>
    <div class="btns">
      {#if custom}<Button size="sm" variant="ghost" onclick={() => (s.files.saveDir = '')}>Use default</Button>{/if}
      <Button size="sm" variant="ghost" icon={FolderOpen} onclick={() => store.run(() => api.openSaveDir())}>Open</Button>
      <Button size="sm" onclick={choose}>Change…</Button>
    </div>
  </div>
  <SettingRow label="Accept files automatically" description="From paired computers only. When off, you’ll be asked each time." id="f-auto">
    <Toggle id="f-auto" bind:checked={s.files.autoAccept} />
  </SettingRow>
  <SettingRow label="If a file already exists" id="f-conflict">
    <Segmented
      bind:value={s.files.conflict}
      label="If a file already exists"
      options={[
        { value: 'rename', label: 'Keep both' },
        { value: 'overwrite', label: 'Replace' },
        { value: 'skip', label: 'Skip' },
      ]}
    />
  </SettingRow>
  <SettingRow label="Show files when they arrive" description="Open the folder once a transfer finishes." id="f-reveal">
    <Toggle id="f-reveal" bind:checked={s.files.revealWhenDone} />
  </SettingRow>
  <SettingRow label="Keep original dates" description="Preserve when files were created and last modified." id="f-dates">
    <Toggle id="f-dates" bind:checked={s.files.preserveTimestamps} />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title="Speed">
  <SettingRow label="Bandwidth limit" description="Cap transfer speed so big files don’t crowd out video calls." stacked id="f-bw">
    <Slider id="f-bw" label="Bandwidth limit" bind:value={s.files.bandwidthLimitMbps} values={[0, 5, 10, 25, 50, 100, 250, 500, 1000]} format={speed} />
  </SettingRow>
</SettingsGroup>

<style>
  .folder {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border);
  }
  .icon {
    width: 38px;
    height: 38px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 11px;
    color: var(--accent-ink);
    background: var(--accent-soft);
  }
  .path {
    flex: 1;
    min-width: 0;
  }
  .lbl {
    font-size: 13.5px;
    font-weight: 540;
  }
  .path .mono {
    font-size: 12px;
    color: var(--text-2);
    margin-top: 1px;
  }
  .default {
    font-size: 11.5px;
    color: var(--text-3);
  }
  .btns {
    display: flex;
    gap: 4px;
    flex: none;
  }
</style>
