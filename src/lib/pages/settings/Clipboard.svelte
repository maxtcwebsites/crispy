<script lang="ts">
  import { store } from '../../store.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import SettingRow from '../../components/SettingRow.svelte';
  import SettingsGroup from '../../components/SettingsGroup.svelte';
  import Slider from '../../components/Slider.svelte';
  import Toggle from '../../components/Toggle.svelte';

  const s = $derived(store.settings!);
  const off = $derived(!s.clipboard.enabled);
  const mb = (v: number) => (v >= 1024 ? `${Number((v / 1024).toFixed(1))} GB` : `${v} MB`);
</script>

<SettingsGroup>
  <SettingRow label="Share the clipboard" description="Copy on one computer and paste on any other." id="cb-on">
    <Toggle id="cb-on" bind:checked={s.clipboard.enabled} />
  </SettingRow>
  <SettingRow
    label="When to share"
    description={s.clipboard.mode === 'instant'
      ? 'Right away — whatever you copy is ready to paste everywhere.'
      : 'Only when the cursor moves to another computer — nothing leaves until you go there.'}
    disabled={off}
    id="cb-mode"
  >
    <Segmented
      bind:value={s.clipboard.mode}
      label="When to share"
      options={[
        { value: 'instant', label: 'Instantly' },
        { value: 'onSwitch', label: 'On switch' },
      ]}
    />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title="What to share">
  <SettingRow label="Plain text" disabled={off} id="cb-text">
    <Toggle id="cb-text" bind:checked={s.clipboard.text} />
  </SettingRow>
  <SettingRow label="Rich text" description="Keep fonts, links and formatting when pasting." disabled={off} id="cb-rich">
    <Toggle id="cb-rich" bind:checked={s.clipboard.richText} />
  </SettingRow>
  <SettingRow label="Images" description="Screenshots and copied pictures." disabled={off} id="cb-img">
    <Toggle id="cb-img" bind:checked={s.clipboard.images} />
  </SettingRow>
  <SettingRow label="Files" description="Copy files in Finder or Explorer, paste them on the other computer." disabled={off} id="cb-files">
    <Toggle id="cb-files" bind:checked={s.clipboard.files} />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title="Limits" note="Anything larger stays on the computer where you copied it.">
  <SettingRow label="Largest image" stacked disabled={off || !s.clipboard.images} id="cb-max-img">
    <Slider id="cb-max-img" label="Largest image" bind:value={s.clipboard.maxImageMb} values={[4, 8, 16, 32, 64, 128, 256]} format={mb} />
  </SettingRow>
  <SettingRow label="Largest set of copied files" stacked disabled={off || !s.clipboard.files} id="cb-max-files">
    <Slider id="cb-max-files" label="Largest set of copied files" bind:value={s.clipboard.maxFilesMb} values={[64, 128, 256, 512, 1024, 2048, 4096]} format={mb} />
  </SettingRow>
</SettingsGroup>
