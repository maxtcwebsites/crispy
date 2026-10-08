<script lang="ts">
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import { store } from '../../store.svelte';
  import type { Settings } from '../../types';
  import Button from '../../components/Button.svelte';
  import HotkeyRecorder from '../../components/HotkeyRecorder.svelte';
  import SettingRow from '../../components/SettingRow.svelte';
  import SettingsGroup from '../../components/SettingsGroup.svelte';

  const s = $derived(store.settings!);
  const keyOf = (code: string) => store.keys.find((k) => k.code === code)?.hid ?? 0;

  function defaults(): Settings['hotkeys'] {
    const h = (code: string) => ({ ctrl: true, shift: true, alt: true, meta: false, key: keyOf(code) });
    return { lockToScreen: h('KeyL'), bringHome: h('KeyH'), nextDevice: h('KeyN'), pauseSharing: h('KeyP') };
  }
</script>

<SettingsGroup note="Click a shortcut, then press the new keys. Esc cancels, Backspace clears. Shortcuts work from every computer’s keyboard.">
  <SettingRow label="Lock to this screen" description="Keep the cursor on the current computer, e.g. while gaming." id="hk-lock">
    <HotkeyRecorder id="hk-lock" label="Lock to this screen" bind:value={s.hotkeys.lockToScreen} />
  </SettingRow>
  <SettingRow label="Bring cursor home" description="Jump straight back to this computer from anywhere." id="hk-home">
    <HotkeyRecorder id="hk-home" label="Bring cursor home" bind:value={s.hotkeys.bringHome} />
  </SettingRow>
  <SettingRow label="Next computer" description="Cycle the cursor through your connected computers." id="hk-next">
    <HotkeyRecorder id="hk-next" label="Next computer" bind:value={s.hotkeys.nextDevice} />
  </SettingRow>
  <SettingRow label="Pause sharing" description="Temporarily stop switching on every computer." id="hk-pause">
    <HotkeyRecorder id="hk-pause" label="Pause sharing" bind:value={s.hotkeys.pauseSharing} />
  </SettingRow>
</SettingsGroup>

<div class="reset">
  <Button size="sm" variant="ghost" icon={RotateCcw} onclick={() => (s.hotkeys = defaults())}>Restore defaults</Button>
</div>

<style>
  .reset {
    margin-top: -14px;
  }
</style>
