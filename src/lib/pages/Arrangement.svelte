<script lang="ts">
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import MousePointer2 from '@lucide/svelte/icons/mouse-pointer-2';
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
  import { api } from '../api';
  import {
    crossingEdges,
    defaultArrangement,
    isolated,
    makeDevice,
    snapAndResolve,
    union,
    withMissing,
    type ArrangeDevice,
    type Guide,
  } from '../arrange';
  import { store } from '../store.svelte';
  import type { Pos } from '../types';
  import { thisDeviceLabel } from '../util';
  import Button from '../components/Button.svelte';
  import OsGlyph from '../components/OsGlyph.svelte';
  import PageHeader from '../components/PageHeader.svelte';

  // ---- devices & positions ----
  const devices: ArrangeDevice[] = $derived.by(() => {
    const me = store.me;
    if (!me) return [];
    const list = [makeDevice({ id: me.id, name: me.name, os: me.os, screens: me.screens, isMe: true, online: true })];
    for (const p of store.paired) {
      if (!p.screens.length) continue;
      list.push(makeDevice({ id: p.id, name: p.name, os: p.os, screens: p.screens, isMe: false, online: p.status === 'connected' && p.enabled }));
    }
    return list.filter((d) => d.screens.length);
  });

  let dragging = $state<string | null>(null);
  const layoutRev = $derived(store.state?.layout.rev ?? 0);
  let override = $state<Record<string, Pos>>({});
  let pendingRev = -1;
  $effect(() => {
    // A new layout from the engine replaces local, optimistic positions.
    if (layoutRev > pendingRev && pendingRev >= 0) {
      override = {};
      pendingRev = -1;
    }
  });

  const base = $derived(withMissing(devices, store.state?.layout.positions ?? {}));
  const positions: Record<string, Pos> = $derived({ ...base, ...override });

  const screens = $derived.by(() => {
    const out = [];
    for (const d of devices) {
      const p = positions[d.id];
      if (!p) continue;
      for (const s of d.screens) out.push({ device: d.id, screen: s, x: p.x + s.x - d.bounds.x, y: p.y + s.y - d.bounds.y, w: s.width, h: s.height });
    }
    return out;
  });
  const edges = $derived(crossingEdges(screens));
  const online = $derived(new Set(devices.filter((d) => d.online).map((d) => d.id)));
  const lonely = $derived(dragging ? [] : isolated(devices, screens));

  // ---- view (fit to canvas) ----
  let cw = $state(0);
  let ch = $state(0);
  interface View {
    s: number;
    tx: number;
    ty: number;
  }
  const fit: View = $derived.by(() => {
    if (!screens.length || !cw || !ch) return { s: 0.1, tx: 0, ty: 0 };
    const u = union(screens);
    const pad = { l: 56, r: 56, t: 64, b: 64 };
    const s = Math.min((cw - pad.l - pad.r) / u.w, (ch - pad.t - pad.b) / u.h, 0.2);
    return {
      s,
      tx: pad.l + (cw - pad.l - pad.r - u.w * s) / 2 - u.x * s,
      ty: pad.t + (ch - pad.t - pad.b - u.h * s) / 2 - u.y * s,
    };
  });
  let frozen = $state<View | null>(null);
  const view = $derived(frozen ?? fit);

  // ---- dragging ----
  let guides = $state<Guide[]>([]);
  let start = { px: 0, py: 0, pos: { x: 0, y: 0 } };
  let moved = false;
  let lastValid: Pos | null = null;

  function down(e: PointerEvent, d: ArrangeDevice) {
    if (e.button !== 0) return;
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    (e.currentTarget as HTMLElement).focus({ preventScroll: true });
    dragging = d.id;
    frozen = { ...view };
    start = { px: e.clientX, py: e.clientY, pos: { ...positions[d.id] } };
    lastValid = { ...positions[d.id] };
    moved = false;
  }

  function move(e: PointerEvent, d: ArrangeDevice) {
    if (dragging !== d.id || !frozen) return;
    const dx = (e.clientX - start.px) / frozen.s;
    const dy = (e.clientY - start.py) / frozen.s;
    if (!moved && Math.hypot(e.clientX - start.px, e.clientY - start.py) < 3) return;
    moved = true;
    place(d, { x: start.pos.x + dx, y: start.pos.y + dy }, 14 / frozen.s);
  }

  function place(d: ArrangeDevice, want: Pos, threshold: number) {
    const others = screens.filter((s) => s.device !== d.id);
    const res = snapAndResolve(d, want, others, threshold);
    if (res.valid) {
      lastValid = res.pos;
      override = { ...override, [d.id]: res.pos };
      guides = res.guides;
    } else if (lastValid) {
      override = { ...override, [d.id]: lastValid };
    }
  }

  async function up(e: PointerEvent, d: ArrangeDevice) {
    if (dragging !== d.id) return;
    (e.currentTarget as HTMLElement).releasePointerCapture?.(e.pointerId);
    dragging = null;
    guides = [];
    frozen = null;
    if (moved) await commit();
  }

  async function commit(next: Record<string, Pos> = positions) {
    const clean: Record<string, Pos> = {};
    for (const d of devices) if (next[d.id]) clean[d.id] = { x: Math.round(next[d.id].x), y: Math.round(next[d.id].y) };
    override = clean;
    pendingRev = layoutRev;
    const ok = await store.run(() => api.setLayout(clean), 'Couldn’t save the arrangement');
    if (!ok) {
      override = {};
      pendingRev = -1;
    }
  }

  // Keyboard: arrows nudge the focused computer (Shift = bigger steps), saved after a pause.
  let nudgeTimer: ReturnType<typeof setTimeout> | undefined;
  function keydown(e: KeyboardEvent, d: ArrangeDevice) {
    const step = (e.shiftKey ? 40 : 6) / view.s;
    const dir: Record<string, [number, number]> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };
    const v = dir[e.key];
    if (!v) return;
    e.preventDefault();
    const p = positions[d.id];
    lastValid = p;
    place(d, { x: p.x + v[0] * step, y: p.y + v[1] * step }, 4 / view.s);
    guides = [];
    clearTimeout(nudgeTimer);
    nudgeTimer = setTimeout(() => commit(), 600);
  }

  async function reset() {
    const ok = await store.ask({
      title: 'Reset the arrangement?',
      body: 'All computers will be placed side by side. You can drag them back where they belong at any time.',
      confirm: 'Reset',
    });
    if (ok) await commit(defaultArrangement(devices));
  }

  // ---- projection helpers ----
  const X = (x: number) => view.tx + x * view.s;
  const Y = (y: number) => view.ty + y * view.s;

  // ---- live cursor ----
  const focusId = $derived(store.state?.focus.kind === 'remote' ? store.state.focus.peerId : (store.me?.id ?? ''));
  interface TrailDot {
    id: number;
    x: number;
    y: number;
  }
  let trail = $state<TrailDot[]>([]);
  let trailSeq = 0;
  let lastAt = $state(0);
  let tick = $state(performance.now());
  const cursor = $derived.by(() => {
    const p = store.pointer;
    if (!p) return null;
    const d = devices.find((x) => x.id === p.device);
    const pos = d && positions[d.id];
    if (!d || !pos) return null;
    return { x: X(pos.x + p.x - d.bounds.x), y: Y(pos.y + p.y - d.bounds.y), t: p.t };
  });
  $effect(() => {
    const c = cursor;
    if (!c || c.t === lastAt) return;
    lastAt = c.t;
    trail = [...trail.slice(-9), { id: ++trailSeq, x: c.x, y: c.y }];
  });
  $effect(() => {
    const t = setInterval(() => (tick = performance.now()), 400);
    return () => clearInterval(t);
  });
  const cursorFresh = $derived(!!cursor && tick - cursor.t < 2500);

  const hint = $derived(
    devices.length < 2
      ? 'Pair another computer to arrange it here.'
      : 'Drag your computers to match where they sit on your desk.',
  );
</script>

<PageHeader title="Arrangement">
  {#snippet subtitle()}<span>{hint}</span>{/snippet}
  {#snippet actions()}
    <Button icon={RotateCcw} disabled={devices.length < 2} onclick={reset}>Reset</Button>
  {/snippet}
</PageHeader>

<div class="canvas" bind:clientWidth={cw} bind:clientHeight={ch} class:is-dragging={!!dragging} role="application" aria-label="Display arrangement">
  <!-- snapping guides -->
  {#each guides as g, i (i)}
    {#if g.axis === 'x'}
      <span class="guide v" style:left="{X(g.at)}px"></span>
    {:else}
      <span class="guide h" style:top="{Y(g.at)}px"></span>
    {/if}
  {/each}

  {#each devices as d (d.id)}
    {@const p = positions[d.id]}
    {@const focused = focusId === d.id && (d.isMe ? store.connected.length > 0 : true)}
    <div
      class="device"
      class:me={d.isMe}
      class:offline={!d.online}
      class:focus={focused}
      class:dragging={dragging === d.id}
      style:left="{X(p.x)}px"
      style:top="{Y(p.y)}px"
      style:width="{d.bounds.w * view.s}px"
      style:height="{d.bounds.h * view.s}px"
      tabindex="0"
      role="button"
      aria-roledescription="draggable computer"
      aria-label="{d.name}. Use arrow keys to move."
      onpointerdown={(e) => down(e, d)}
      onpointermove={(e) => move(e, d)}
      onpointerup={(e) => up(e, d)}
      onpointercancel={(e) => up(e, d)}
      onkeydown={(e) => keydown(e, d)}
    >
      {#each d.screens as s (s.id)}
        {@const w = s.width * view.s}
        {@const h = s.height * view.s}
        <div
          class="screen"
          class:primary={s.primary}
          style:left="{(s.x - d.bounds.x) * view.s + 1.5}px"
          style:top="{(s.y - d.bounds.y) * view.s + 1.5}px"
          style:width="{w - 3}px"
          style:height="{h - 3}px"
        >
          {#if s.primary}
            {#if d.os === 'windows'}<span class="taskbar"></span>{:else}<span class="menubar"></span>{/if}
            <div class="label" class:tiny={w < 118}>
              <span class="glyph"><OsGlyph os={d.os} size={w < 118 ? 13 : 15} /></span>
              <span class="name">{d.name}</span>
              {#if w >= 118 && h >= 74}
                <span class="res mono">{s.width} × {s.height}</span>
              {/if}
              {#if d.isMe && w >= 118 && h >= 96}<span class="tag">{thisDeviceLabel(d.os)}</span>{/if}
            </div>
          {:else if w >= 80 && h >= 40}
            <span class="res mono solo">{s.width} × {s.height}</span>
          {/if}
        </div>
      {/each}
    </div>
  {/each}

  <!-- where the cursor can cross -->
  <svg class="edges" width={cw} height={ch} aria-hidden="true">
    {#each edges as e, i (i)}
      {@const live = online.has(e.a) && online.has(e.b)}
      <g class:muted={!live}>
        <line x1={X(e.x1)} y1={Y(e.y1)} x2={X(e.x2)} y2={Y(e.y2)} class="edge-glow" />
        <line x1={X(e.x1)} y1={Y(e.y1)} x2={X(e.x2)} y2={Y(e.y2)} class="edge" />
      </g>
    {/each}
  </svg>

  {#if cursor && !dragging}
    <div class="trail" aria-hidden="true">
      {#each trail as t (t.id)}
        <span style:transform="translate({t.x}px, {t.y}px)"></span>
      {/each}
    </div>
    <span class="cursor" class:stale={!cursorFresh} style:transform="translate({cursor.x}px, {cursor.y}px)" aria-hidden="true"></span>
  {/if}

  <div class="legend" aria-hidden="true">
    <span><i class="dot"></i>Cursor</span>
    <span><i class="line"></i>Crossing edge</span>
  </div>
</div>

<p class="tip">
  {#if lonely.length}
    <TriangleAlert size={14} strokeWidth={1.75} aria-hidden="true" class="warn" />
    <span><strong>{lonely.map((d) => d.name).join(', ')}</strong> {lonely.length === 1 ? 'doesn’t' : 'don’t'} touch another screen, so the cursor can’t get there. Drag {lonely.length === 1 ? 'it' : 'them'} next to a neighbour.</span>
  {:else}
    <MousePointer2 size={14} strokeWidth={1.75} aria-hidden="true" />
    <span>Move your cursor across the glowing edges to switch computers. Screens snap together as you drag.</span>
  {/if}
</p>

<style>
  .canvas {
    position: relative;
    flex: 1;
    min-height: 260px;
    border-radius: var(--r-xl);
    overflow: hidden;
    background:
      radial-gradient(70% 60% at 50% 45%, rgb(var(--accent-rgb) / 0.045), transparent 70%),
      radial-gradient(circle at 1px 1px, var(--grid-dot) 1px, transparent 1.4px) 0 0 / 22px 22px,
      var(--surface-sunken);
    box-shadow:
      inset 0 0 0 1px var(--border),
      inset 0 1px 12px rgb(0 0 0 / 0.25);
    touch-action: none;
  }
  :global([data-theme='light']) .canvas {
    background:
      radial-gradient(70% 60% at 50% 45%, rgb(var(--accent-rgb) / 0.06), transparent 70%),
      radial-gradient(circle at 1px 1px, var(--grid-dot) 1px, transparent 1.4px) 0 0 / 22px 22px,
      #f1eee7;
    box-shadow:
      inset 0 0 0 1px var(--border),
      inset 0 1px 8px rgb(60 40 10 / 0.06);
  }
  .canvas:focus-visible {
    box-shadow: var(--accent-ring);
  }

  .device {
    position: absolute;
    border-radius: 8px;
    cursor: grab;
    transition:
      left 340ms var(--ease),
      top 340ms var(--ease),
      width 340ms var(--ease),
      height 340ms var(--ease),
      filter 200ms var(--ease);
    z-index: 1;
  }
  .device:focus-visible {
    box-shadow: none;
  }
  .device:focus-visible .screen {
    box-shadow:
      inset 0 0 0 1.5px rgb(var(--accent-rgb) / 0.8),
      0 0 0 3px rgb(var(--accent-rgb) / 0.2);
  }
  .device.dragging {
    cursor: grabbing;
    transition: none;
    z-index: 3;
    filter: drop-shadow(0 18px 28px rgb(0 0 0 / 0.45));
  }
  .is-dragging .device:not(.dragging) {
    filter: saturate(0.8);
  }
  .device.offline .screen {
    opacity: 0.55;
  }

  .screen {
    position: absolute;
    border-radius: 7px;
    overflow: hidden;
    background:
      linear-gradient(150deg, rgb(255 255 255 / 0.07) 0%, transparent 38%),
      linear-gradient(180deg, var(--glass-top), var(--glass-bottom));
    box-shadow:
      inset 0 0 0 1px var(--glass-border),
      inset 0 1px 0 rgb(255 255 255 / 0.07),
      0 10px 24px -14px rgb(0 0 0 / 0.7);
    transition:
      left 340ms var(--ease),
      top 340ms var(--ease),
      width 340ms var(--ease),
      height 340ms var(--ease),
      box-shadow 260ms var(--ease);
  }
  :global([data-theme='light']) .screen {
    box-shadow:
      inset 0 0 0 1px var(--glass-border),
      0 10px 20px -14px rgb(60 40 10 / 0.35);
  }
  .dragging .screen {
    transition:
      box-shadow 260ms var(--ease);
  }
  .device:hover .screen {
    box-shadow:
      inset 0 0 0 1px var(--border-heavy),
      inset 0 1px 0 rgb(255 255 255 / 0.08),
      0 10px 24px -14px rgb(0 0 0 / 0.7);
  }
  .focus .screen,
  .focus:hover .screen {
    background:
      radial-gradient(110% 80% at 50% 0%, rgb(var(--accent-rgb) / 0.17), transparent 70%),
      linear-gradient(180deg, var(--glass-top), var(--glass-bottom));
    box-shadow:
      inset 0 0 0 1px rgb(var(--accent-rgb) / 0.6),
      inset 0 0 22px -8px rgb(var(--accent-rgb) / 0.45),
      0 14px 30px -16px rgb(var(--accent-rgb) / 0.55);
  }
  .menubar {
    position: absolute;
    inset: 0 0 auto;
    height: 5px;
    background: rgb(255 255 255 / 0.09);
  }
  .taskbar {
    position: absolute;
    inset: auto 0 0;
    height: 6px;
    background: rgb(255 255 255 / 0.07);
  }
  :global([data-theme='light']) .menubar,
  :global([data-theme='light']) .taskbar {
    background: rgb(40 30 18 / 0.06);
  }

  .label {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 3px;
    padding: 6px;
    text-align: center;
    pointer-events: none;
  }
  .glyph {
    color: var(--text-2);
    margin-bottom: 2px;
  }
  .focus .glyph {
    color: var(--accent-ink);
  }
  .name {
    max-width: 100%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 12.5px;
    font-weight: 620;
    letter-spacing: -0.01em;
    color: var(--text-1);
  }
  .tiny .name {
    font-size: 10.5px;
  }
  .res {
    font-size: 10px;
    color: var(--text-3);
  }
  .res.solo {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
  }
  .tag {
    margin-top: 3px;
    padding: 1px 6px;
    border-radius: 5px;
    font-size: 9.5px;
    font-weight: 650;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-3);
    background: var(--fill-2);
  }

  .edges {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 2;
    overflow: visible;
  }
  .edge {
    stroke: var(--accent);
    stroke-width: 2.5;
    stroke-linecap: round;
  }
  .edge-glow {
    stroke: rgb(var(--accent-rgb) / 0.45);
    stroke-width: 9;
    stroke-linecap: round;
    filter: blur(4px);
  }
  .muted {
    opacity: 0.32;
  }
  .muted .edge-glow {
    display: none;
  }
  .is-dragging .edges {
    opacity: 0.5;
  }

  .guide {
    position: absolute;
    z-index: 2;
    pointer-events: none;
    background: rgb(var(--accent-rgb) / 0.55);
  }
  .guide.v {
    top: 0;
    bottom: 0;
    width: 1px;
  }
  .guide.h {
    left: 0;
    right: 0;
    height: 1px;
  }

  .cursor {
    position: absolute;
    left: -6px;
    top: -6px;
    z-index: 4;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow:
      0 0 0 3px rgb(var(--accent-rgb) / 0.22),
      0 0 18px 2px rgb(var(--accent-rgb) / 0.75);
    transition:
      transform 70ms linear,
      opacity 600ms var(--ease);
    pointer-events: none;
  }
  .cursor.stale {
    opacity: 0.55;
  }
  .trail span {
    position: absolute;
    left: -4px;
    top: -4px;
    z-index: 3;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: rgb(var(--accent-rgb) / 0.5);
    pointer-events: none;
    animation: fade 520ms var(--ease) forwards;
  }
  @keyframes fade {
    from {
      opacity: 0.55;
      scale: 1;
    }
    to {
      opacity: 0;
      scale: 0.3;
    }
  }

  .legend {
    position: absolute;
    left: 16px;
    bottom: 14px;
    z-index: 5;
    display: flex;
    gap: 16px;
    font-size: 11px;
    color: var(--text-3);
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 7px;
  }
  .legend .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 8px rgb(var(--accent-rgb) / 0.8);
  }
  .legend .line {
    width: 14px;
    height: 2.5px;
    border-radius: 2px;
    background: var(--accent);
    box-shadow: 0 0 6px rgb(var(--accent-rgb) / 0.8);
  }

  .tip {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin-top: 14px;
    padding: 0 4px;
    font-size: 12.5px;
    color: var(--text-2);
    line-height: 1.5;
  }
  .tip :global(svg) {
    flex: none;
    margin-top: 2px;
    color: var(--text-3);
  }
  .tip :global(svg.warn) {
    color: var(--amber);
  }
  strong {
    color: var(--text-1);
    font-weight: 600;
  }
</style>
