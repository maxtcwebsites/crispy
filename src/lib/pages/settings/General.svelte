<script lang="ts">
  import SunMoon from '@lucide/svelte/icons/sun-moon';
  import Sun from '@lucide/svelte/icons/sun';
  import Moon from '@lucide/svelte/icons/moon';
  import { flavorInfo } from '../../flavors';
  import { store } from '../../store.svelte';
  import FlavorPicker from '../../components/FlavorPicker.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import SettingRow from '../../components/SettingRow.svelte';
  import SettingsGroup from '../../components/SettingsGroup.svelte';
  import TextField from '../../components/TextField.svelte';
  import Toggle from '../../components/Toggle.svelte';

  const s = $derived(store.settings!);
  const tray = $derived(store.os === 'macos' ? 'menu bar' : 'notification area');

  // Edit the name locally; commit on blur / Enter so trimming never fights the caret.
  let name = $state(store.settings?.general.deviceName || store.me?.name || '');
  function commitName() {
    const v = name.trim();
    if (!v) name = store.me?.name ?? '';
    if (v && v !== s.general.deviceName) s.general.deviceName = v;
  }
</script>

<SettingsGroup title="This computer">
  <SettingRow label="Name" description="How this computer appears on your other computers." id="gen-name">
    <div class="name">
      <TextField id="gen-name" bind:value={name} maxlength={64} onchange={commitName} onkeydown={(e) => e.key === 'Enter' && (e.currentTarget as HTMLInputElement).blur()} />
    </div>
  </SettingRow>
</SettingsGroup>

<SettingsGroup title="Appearance">
  <SettingRow label="Theme" id="gen-theme">
    <Segmented
      bind:value={s.general.theme}
      label="Theme"
      options={[
        { value: 'system', label: 'Auto', icon: SunMoon },
        { value: 'light', label: 'Light', icon: Sun },
        { value: 'dark', label: 'Dark', icon: Moon },
      ]}
    />
  </SettingRow>
  <SettingRow label="Flavor" description={`${flavorInfo(s.general.flavor).name} — the accent colour across Crispy.`} id="gen-flavor">
    <FlavorPicker bind:value={s.general.flavor} id="gen-flavor" />
  </SettingRow>
  <SettingRow
    label="Translucent window"
    description={store.info?.windowEffects ? 'Let your desktop softly show through the sidebar.' : 'Not available on this system.'}
    id="gen-translucent"
    disabled={!store.info?.windowEffects}
  >
    <Toggle id="gen-translucent" bind:checked={s.general.translucentWindow} />
  </SettingRow>
  <SettingRow label="Reduce motion" description="Fewer animations throughout the app." id="gen-motion">
    <Toggle id="gen-motion" bind:checked={s.general.reduceMotion} />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title="Startup">
  <SettingRow label="Open at login" description="Start Crispy when you sign in, so sharing is ready right away." id="gen-login">
    <Toggle id="gen-login" bind:checked={s.general.launchAtLogin} />
  </SettingRow>
  <SettingRow label="Start in the background" description={`Open quietly in the ${tray} instead of showing this window.`} id="gen-min">
    <Toggle id="gen-min" bind:checked={s.general.startMinimized} />
  </SettingRow>
  <SettingRow label={`Keep running in the ${tray}`} description="Closing the window keeps sharing on. Quit from the menu to stop." id="gen-tray">
    <Toggle id="gen-tray" bind:checked={s.general.closeToTray} />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title="Notifications">
  <SettingRow label="Show notifications" description="For new devices, pairing requests and finished transfers." id="gen-notify">
    <Toggle id="gen-notify" bind:checked={s.general.notifications} />
  </SettingRow>
  <SettingRow label="Play sounds" description="A soft click when the cursor changes computers." id="gen-sounds">
    <Toggle id="gen-sounds" bind:checked={s.general.sounds} />
  </SettingRow>
</SettingsGroup>

<style>
  .name {
    width: 220px;
  }
</style>
