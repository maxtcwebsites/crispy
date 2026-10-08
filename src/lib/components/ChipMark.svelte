<script lang="ts">
  // The Crispy chip, inlined from branding/crispy-mark.svg (single source of truth). Every
  // instance gets its own id prefix so gradients/filters never collide or vanish.
  import raw from '../../../branding/crispy-mark.svg?raw';

  interface Props {
    size?: number;
    label?: string;
    class?: string;
  }
  let { size = 28, label = '', class: cls = '' }: Props = $props();

  const uid = `cm${Math.random().toString(36).slice(2, 8)}-`;
  const svg = raw
    .replace(/<title>[\s\S]*?<\/title>/, '')
    .replace(/id="([^"]+)"/g, `id="${uid}$1"`)
    .replace(/url\(#([^)]+)\)/g, `url(#${uid}$1)`)
    .replace(/href="#([^"]+)"/g, `href="#${uid}$1"`)
    .replace('<svg', '<svg width="100%" height="100%" aria-hidden="true" focusable="false"');
</script>

<span
  class="chip {cls}"
  style:width="{size}px"
  style:height="{size}px"
  role={label ? 'img' : undefined}
  aria-label={label || undefined}
  aria-hidden={label ? undefined : 'true'}
>
  {@html svg}
</span>

<style>
  .chip {
    display: inline-block;
    flex: none;
    line-height: 0;
  }
  .chip :global(svg) {
    overflow: visible;
  }
</style>
