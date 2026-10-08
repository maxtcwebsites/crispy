<script lang="ts">
  import { store } from '../../store.svelte';
  import SettingRow from '../../components/SettingRow.svelte';
  import SettingsGroup from '../../components/SettingsGroup.svelte';
  import Slider from '../../components/Slider.svelte';
  import Toggle from '../../components/Toggle.svelte';

  const s = $derived(store.settings!);
  const times = (v: number) => `${Number(v.toFixed(2))}×`;
</script>

<SettingsGroup title="Pointer">
  <SettingRow label="Pointer speed" description="How fast the cursor moves on the computers you control." stacked id="m-speed">
    <Slider id="m-speed" label="Pointer speed" bind:value={s.mouse.pointerSpeed} min={0.25} max={3} step={0.05} format={times} ends={['Slower', 'Faster']} />
  </SettingRow>
  <SettingRow label="Raw movement" description="Send relative motion instead of positions. Turn on for games that capture the mouse." id="m-raw">
    <Toggle id="m-raw" bind:checked={s.mouse.relativeMovement} />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title="Scrolling">
  <SettingRow label="Scroll speed" stacked id="m-scroll">
    <Slider id="m-scroll" label="Scroll speed" bind:value={s.mouse.scrollSpeed} min={0.25} max={4} step={0.05} format={times} ends={['Slower', 'Faster']} />
  </SettingRow>
  <SettingRow label="Smooth scrolling" description="Pass trackpad-precise scrolling through instead of whole wheel notches." id="m-smooth">
    <Toggle id="m-smooth" bind:checked={s.mouse.smoothScrolling} />
  </SettingRow>
  <SettingRow label="Reverse vertical scrolling" description="Useful when one computer uses natural scrolling and the other doesn’t." id="m-inv-v">
    <Toggle id="m-inv-v" bind:checked={s.mouse.invertVertical} />
  </SettingRow>
  <SettingRow label="Reverse horizontal scrolling" id="m-inv-h">
    <Toggle id="m-inv-h" bind:checked={s.mouse.invertHorizontal} />
  </SettingRow>
</SettingsGroup>
