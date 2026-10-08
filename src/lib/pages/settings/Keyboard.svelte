<script lang="ts">
  import { store } from '../../store.svelte';
  import ModifierMapEditor from '../../components/ModifierMapEditor.svelte';
  import SettingRow from '../../components/SettingRow.svelte';
  import SettingsGroup from '../../components/SettingsGroup.svelte';
  import Toggle from '../../components/Toggle.svelte';

  const s = $derived(store.settings!);
</script>

<SettingsGroup title="Mac keyboard on Windows & Linux" note="Shortcut-friendly keeps ⌘C, ⌘V and friends working as Ctrl+C and Ctrl+V on the PC.">
  <ModifierMapEditor bind:value={s.keyboard.macToPc} from="mac" />
</SettingsGroup>

<SettingsGroup title="Windows & Linux keyboard on a Mac" note="Shortcut-friendly turns Ctrl+C into ⌘C on the Mac, so copy and paste feel the same everywhere.">
  <ModifierMapEditor bind:value={s.keyboard.pcToMac} from="pc" />
</SettingsGroup>

<SettingsGroup title="Keys">
  <SettingRow label="Media keys" description="Play, pause, volume and brightness go to the computer you’re controlling." id="kb-media">
    <Toggle id="kb-media" bind:checked={s.keyboard.forwardMediaKeys} />
  </SettingRow>
  <SettingRow label="Sync Caps Lock" description="Caps Lock on the other computer matches this one when the cursor arrives." id="kb-caps">
    <Toggle id="kb-caps" bind:checked={s.keyboard.syncCapsLock} />
  </SettingRow>
</SettingsGroup>
