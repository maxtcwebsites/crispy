<script lang="ts">
  interface Props {
    value: number;
    label: string;
    min?: number;
    max?: number;
    step?: number;
    /** Discrete stops instead of a continuous range. */
    values?: number[];
    format?: (v: number) => string;
    id?: string;
    disabled?: boolean;
    /** Small captions under both ends of the track. */
    ends?: [string, string];
    onchange?: (v: number) => void;
  }
  let {
    value = $bindable(),
    label,
    min = 0,
    max = 100,
    step = 1,
    values,
    format = (v: number) => String(v),
    id,
    disabled = false,
    ends,
    onchange,
  }: Props = $props();

  const discrete = $derived(!!values && values.length > 1);

  function nearest(list: number[], v: number): number {
    let best = 0;
    for (let i = 1; i < list.length; i++) if (Math.abs(list[i] - v) < Math.abs(list[best] - v)) best = i;
    return best;
  }

  const index = $derived(discrete ? nearest(values!, value) : 0);
  const fraction = $derived.by(() => {
    if (discrete) return index / (values!.length - 1);
    return Math.min(1, Math.max(0, (value - min) / (max - min || 1)));
  });

  function input(e: Event & { currentTarget: HTMLInputElement }) {
    const raw = Number(e.currentTarget.value);
    value = discrete ? values![raw] : Math.round(raw / step) * step;
    // Avoid float noise like 1.2500000001.
    value = Number(value.toFixed(4));
    onchange?.(value);
  }
</script>

<div class="slider" class:disabled style="--f: {fraction}">
  <div class="track">
    <input
      {id}
      type="range"
      min={discrete ? 0 : min}
      max={discrete ? values!.length - 1 : max}
      step={discrete ? 1 : step}
      value={discrete ? index : value}
      aria-label={label}
      aria-valuetext={format(value)}
      {disabled}
      oninput={input}
    />
    {#if discrete}
      <div class="ticks" aria-hidden="true">
        {#each values! as _, i (i)}
          <span class:past={i <= index} style="left: calc(8px + (100% - 16px) * {i / (values!.length - 1)})"></span>
        {/each}
      </div>
    {/if}
    {#if ends}
      <div class="ends" aria-hidden="true"><span>{ends[0]}</span><span>{ends[1]}</span></div>
    {/if}
  </div>
  <output class="value tnum" for={id}>{format(value)}</output>
</div>

<style>
  .slider {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    width: 100%;
    min-width: 0;
  }
  .disabled {
    opacity: 0.45;
  }
  .track {
    position: relative;
    flex: 1;
    min-width: 0;
  }
  .value {
    flex: none;
    line-height: 22px;
    min-width: 66px;
    text-align: right;
    font-size: 12.5px;
    font-weight: 560;
    color: var(--text-1);
  }

  input {
    -webkit-appearance: none;
    appearance: none;
    display: block;
    width: 100%;
    height: 22px;
    margin: 0;
    background: transparent;
    position: relative;
    z-index: 1;
  }
  input:focus-visible {
    box-shadow: none;
  }
  input::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: 4px;
    background: linear-gradient(
      to right,
      var(--accent) 0,
      var(--accent) calc(8px + (100% - 16px) * var(--f)),
      var(--fill-3) calc(8px + (100% - 16px) * var(--f)),
      var(--fill-3) 100%
    );
  }
  :global([data-theme='light']) input::-webkit-slider-runnable-track {
    background: linear-gradient(
      to right,
      var(--accent) 0,
      var(--accent) calc(8px + (100% - 16px) * var(--f)),
      rgb(40 30 18 / 0.12) calc(8px + (100% - 16px) * var(--f)),
      rgb(40 30 18 / 0.12) 100%
    );
  }
  input::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    margin-top: -6px;
    border-radius: 50%;
    background: var(--knob);
    box-shadow:
      0 0 0 0.5px rgb(0 0 0 / 0.12),
      0 1px 3px rgb(0 0 0 / 0.35),
      0 3px 8px -2px rgb(0 0 0 / 0.25);
    transition:
      transform 160ms var(--ease),
      box-shadow 160ms var(--ease);
  }
  input:active::-webkit-slider-thumb {
    transform: scale(1.14);
  }
  input:focus-visible::-webkit-slider-thumb {
    box-shadow:
      0 0 0 3px var(--bg-panel),
      0 0 0 5px rgb(var(--accent-rgb) / 0.6);
  }
  input::-moz-range-track {
    height: 4px;
    border-radius: 4px;
    background: var(--fill-3);
  }
  input::-moz-range-progress {
    height: 4px;
    border-radius: 4px;
    background: var(--accent);
  }
  input::-moz-range-thumb {
    width: 16px;
    height: 16px;
    border: 0;
    border-radius: 50%;
    background: var(--knob);
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.35);
  }

  .ticks {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 0;
  }
  .ticks span {
    position: absolute;
    top: 50%;
    width: 2px;
    height: 8px;
    margin: -4px 0 0 -1px;
    border-radius: 2px;
    background: var(--fill-4);
  }
  .ticks span.past {
    background: rgb(var(--accent-rgb) / 0.5);
  }
  .ends {
    display: flex;
    justify-content: space-between;
    font-size: 10.5px;
    color: var(--text-3);
    margin-top: 1px;
    padding: 0 1px;
  }
</style>
