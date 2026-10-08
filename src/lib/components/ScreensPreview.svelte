<script lang="ts">
  // A miniature, to-scale rendering of a computer's displays, used on device cards.
  import type { Os, Screen } from '../types';
  import { screenBounds } from '../util';

  interface Props {
    screens: Screen[];
    os: Os;
    width: number;
    height: number;
    active?: boolean;
    dim?: boolean;
    /** Live cursor in this device's coordinates. */
    cursor?: { x: number; y: number } | null;
  }
  let { screens, os, width, height, active = false, dim = false, cursor = null }: Props = $props();

  const pad = 6;
  const layout = $derived.by(() => {
    const b = screenBounds(screens);
    if (!b.w || !b.h) return { scale: 0, ox: 0, oy: 0, b };
    // Leave room so a single laptop screen doesn't look gigantic next to multi-monitor rigs.
    const scale = Math.min((width - pad * 2) / b.w, (height - pad * 2) / b.h, (height * 0.92) / 1080);
    const ox = (width - b.w * scale) / 2;
    const oy = (height - b.h * scale) / 2;
    return { scale, ox, oy, b };
  });

  const dot = $derived.by(() => {
    if (!cursor || !layout.scale) return null;
    return {
      x: layout.ox + (cursor.x - layout.b.x) * layout.scale,
      y: layout.oy + (cursor.y - layout.b.y) * layout.scale,
    };
  });
</script>

<div class="stage" class:active class:dim style:width="{width}px" style:height="{height}px" aria-hidden="true">
  {#each screens as s (s.id)}
    {@const w = s.width * layout.scale}
    {@const h = s.height * layout.scale}
    <div
      class="screen"
      class:primary={s.primary}
      style:left="{layout.ox + (s.x - layout.b.x) * layout.scale + 1.5}px"
      style:top="{layout.oy + (s.y - layout.b.y) * layout.scale + 1.5}px"
      style:width="{Math.max(4, w - 3)}px"
      style:height="{Math.max(4, h - 3)}px"
    >
      {#if s.primary}
        {#if os === 'windows'}
          <span class="taskbar"><i></i><i></i><i></i><i></i></span>
        {:else}
          <span class="menubar"></span>
          {#if os === 'macos' && h > 34}<span class="dock"></span>{/if}
        {/if}
      {/if}
    </div>
  {/each}
  {#if dot}
    <span class="cursor" style:transform="translate({dot.x}px, {dot.y}px)"></span>
  {/if}
</div>

<style>
  .stage {
    position: relative;
    flex: none;
  }
  .screen {
    position: absolute;
    border-radius: 4px;
    background:
      linear-gradient(155deg, rgb(255 255 255 / 0.06), transparent 40%),
      linear-gradient(180deg, var(--glass-top), var(--glass-bottom));
    box-shadow:
      inset 0 0 0 1px var(--glass-border),
      inset 0 1px 0 rgb(255 255 255 / 0.06),
      0 6px 14px -8px rgb(0 0 0 / 0.6);
    overflow: hidden;
    transition:
      box-shadow var(--dur-3) var(--ease),
      background var(--dur-3) var(--ease);
  }
  :global([data-theme='light']) .screen {
    box-shadow:
      inset 0 0 0 1px var(--glass-border),
      0 4px 10px -6px rgb(60 40 10 / 0.25);
  }
  .active .screen {
    background:
      radial-gradient(120% 90% at 50% 0%, rgb(var(--accent-rgb) / 0.16), transparent 70%),
      linear-gradient(180deg, var(--glass-top), var(--glass-bottom));
    box-shadow:
      inset 0 0 0 1px rgb(var(--accent-rgb) / 0.55),
      0 0 18px -6px rgb(var(--accent-rgb) / 0.5);
  }
  .dim .screen {
    opacity: 0.45;
  }
  .menubar {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 3px;
    background: rgb(255 255 255 / 0.1);
  }
  :global([data-theme='light']) .menubar {
    background: rgb(40 30 18 / 0.08);
  }
  .dock {
    position: absolute;
    left: 50%;
    bottom: 3px;
    width: 34%;
    height: 3px;
    transform: translateX(-50%);
    border-radius: 2px;
    background: rgb(255 255 255 / 0.12);
  }
  :global([data-theme='light']) .dock {
    background: rgb(40 30 18 / 0.1);
  }
  .taskbar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 4px;
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 2px;
    background: rgb(255 255 255 / 0.08);
  }
  :global([data-theme='light']) .taskbar {
    background: rgb(40 30 18 / 0.07);
  }
  .taskbar i {
    width: 2px;
    height: 2px;
    border-radius: 1px;
    background: rgb(255 255 255 / 0.35);
  }
  :global([data-theme='light']) .taskbar i {
    background: rgb(40 30 18 / 0.3);
  }
  .cursor {
    position: absolute;
    left: -3.5px;
    top: -3.5px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow:
      0 0 0 2px rgb(var(--accent-rgb) / 0.25),
      0 0 10px rgb(var(--accent-rgb) / 0.9);
    transition: transform 90ms linear;
  }
</style>
