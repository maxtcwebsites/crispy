<script lang="ts">
  import { store } from '../../store.svelte';
  import { modLabel } from '../../util';
  import CornerPicker from '../../components/CornerPicker.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import Select from '../../components/Select.svelte';
  import SettingRow from '../../components/SettingRow.svelte';
  import SettingsGroup from '../../components/SettingsGroup.svelte';
  import Slider from '../../components/Slider.svelte';
  import Toggle from '../../components/Toggle.svelte';

  const s = $derived(store.settings!);
  const off = $derived(!s.switching.enabled);
  const os = $derived(store.os);
  const ms = (v: number) => (v === 0 ? 'Instant' : `${v} ms`);
</script>

<SettingsGroup>
  <SettingRow label="Switch at the screen edge" description="Move the cursor past the edge of the screen to continue on the computer next to it." id="sw-on">
    <Toggle id="sw-on" bind:checked={s.switching.enabled} />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title="Timing">
  <SettingRow label="Edge delay" description="How long the cursor rests against the edge before switching." stacked disabled={off} id="sw-delay">
    <Slider id="sw-delay" label="Edge delay" bind:value={s.switching.delayMs} min={0} max={1000} step={25} format={ms} ends={['Instant', '1 s']} />
  </SettingRow>
  <SettingRow label="Double-tap to switch" description="Push against the edge twice in a row. Great for screens you often brush past." disabled={off} id="sw-dbl">
    <Toggle id="sw-dbl" bind:checked={s.switching.doubleTap} />
  </SettingRow>
  {#if s.switching.doubleTap}
    <SettingRow label="Double-tap window" description="Time allowed between the two taps." stacked disabled={off} id="sw-dblw">
      <Slider id="sw-dblw" label="Double-tap window" bind:value={s.switching.doubleTapWindowMs} min={150} max={1000} step={25} format={(v) => `${v} ms`} />
    </SettingRow>
  {/if}
  <SettingRow label="Hold a key to switch" description="Only switch while this key is held down." disabled={off} id="sw-mod">
    <Select
      id="sw-mod"
      bind:value={s.switching.requiredModifier}
      label="Required key"
      width={150}
      options={[
        { value: 'none', label: 'No key needed' },
        { value: 'shift', label: `${modLabel('shift', os)}${os === 'macos' ? ' Shift' : ''}` },
        { value: 'ctrl', label: `${modLabel('ctrl', os)}${os === 'macos' ? ' Control' : ''}` },
        { value: 'alt', label: `${modLabel('alt', os)}${os === 'macos' ? ' Option' : ''}` },
        { value: 'meta', label: `${modLabel('meta', os)}${os === 'macos' ? ' Command' : ''}` },
      ]}
    />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title="Corners" note="Hot corners, the Dock and Start menu live near corners — dead zones keep them from flinging the cursor to another computer.">
  <div class="corners" class:off>
    <CornerPicker bind:corners={s.switching.corners} size={s.switching.cornerSize} disabled={off || s.switching.cornerSize === 0} />
    <div class="corner-text">
      <label class="lbl" for="sw-corner">Dead zone size</label>
      <p class="desc">Click a corner to turn it on or off.</p>
      <Slider id="sw-corner" label="Corner dead zone size" bind:value={s.switching.cornerSize} min={0} max={200} step={4} format={(v) => (v === 0 ? 'Off' : `${v} px`)} disabled={off} />
    </div>
  </div>
</SettingsGroup>

<SettingsGroup title="Edges">
  <SettingRow
    label="Where the cursor lands"
    description={s.switching.edgeMapping === 'direct'
      ? 'Directly across — just like the screens are physically lined up.'
      : 'Proportionally — the top of one edge maps to the top of the other, even if the screens differ in size.'}
    id="sw-map"
  >
    <Segmented
      bind:value={s.switching.edgeMapping}
      label="Edge mapping"
      options={[
        { value: 'direct', label: 'Direct' },
        { value: 'proportional', label: 'Proportional' },
      ]}
    />
  </SettingRow>
</SettingsGroup>

<SettingsGroup title="Safeguards">
  <SettingRow label="Not while dragging" description="Holding a mouse button (moving a window, selecting text) never switches." disabled={off} id="sw-drag">
    <Toggle id="sw-drag" bind:checked={s.switching.blockWhileDragging} />
  </SettingRow>
  <SettingRow label="Not in full-screen apps" description="Games and full-screen video keep the cursor to themselves." disabled={off} id="sw-full">
    <Toggle id="sw-full" bind:checked={s.switching.blockFullscreen} />
  </SettingRow>
  <SettingRow label="Local input takes over" description="Touching the mouse or keyboard of a computer that’s being controlled hands it back right away." id="sw-local">
    <Toggle id="sw-local" bind:checked={s.switching.localInputTakesOver} />
  </SettingRow>
</SettingsGroup>

<style>
  .corners {
    display: flex;
    align-items: center;
    gap: 22px;
    padding: 16px;
  }
  .corners.off {
    opacity: 0.5;
  }
  .corner-text {
    flex: 1;
    min-width: 0;
  }
  .lbl {
    display: block;
    font-size: 13.5px;
    font-weight: 540;
  }
  .desc {
    margin: 2px 0 10px;
    font-size: 12.5px;
    color: var(--text-2);
  }
</style>
