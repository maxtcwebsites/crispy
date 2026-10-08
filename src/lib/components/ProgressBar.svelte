<script lang="ts">
  interface Props {
    value: number; // 0..1
    tone?: 'accent' | 'green' | 'red' | 'muted';
    active?: boolean;
    label?: string;
  }
  let { value, tone = 'accent', active = false, label = 'Progress' }: Props = $props();
  const pct = $derived(Math.round(Math.min(1, Math.max(0, value)) * 1000) / 10);
</script>

<div class="bar {tone}" class:active role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct}>
  <div class="fill" style:width="{pct}%"></div>
</div>

<style>
  .bar {
    position: relative;
    height: 4px;
    border-radius: 4px;
    overflow: hidden;
    background: var(--fill-3);
  }
  .fill {
    position: relative;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
    transition: width 420ms var(--ease);
    overflow: hidden;
  }
  .active .fill {
    box-shadow: 0 0 12px rgb(var(--accent-rgb) / 0.6);
  }
  .active .fill::after {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent, rgb(255 255 255 / 0.55), transparent);
    width: 60px;
    animation: sheen 1.6s var(--ease-in-out) infinite;
  }
  .green .fill {
    background: var(--green);
  }
  .red .fill {
    background: var(--red);
    opacity: 0.7;
  }
  .muted .fill {
    background: var(--text-4);
  }
  @keyframes sheen {
    from {
      transform: translateX(-60px);
    }
    to {
      transform: translateX(600px);
    }
  }
</style>
