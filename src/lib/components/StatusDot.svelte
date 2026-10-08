<script lang="ts">
  interface Props {
    tone: 'green' | 'amber' | 'red' | 'grey' | 'accent';
    pulse?: boolean;
    size?: number;
  }
  let { tone, pulse = false, size = 7 }: Props = $props();
</script>

<span class="dot {tone}" class:pulse style:width="{size}px" style:height="{size}px" aria-hidden="true"></span>

<style>
  .dot {
    --c: var(--text-4);
    position: relative;
    display: inline-block;
    flex: none;
    border-radius: 50%;
    background: var(--c);
  }
  .green {
    --c: var(--green);
    box-shadow: 0 0 8px rgb(var(--green-rgb) / 0.55);
  }
  .amber {
    --c: var(--amber);
  }
  .red {
    --c: var(--red);
  }
  .accent {
    --c: var(--accent);
    box-shadow: 0 0 10px rgb(var(--accent-rgb) / 0.7);
  }
  .pulse::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: var(--c);
    animation: ping 1.8s var(--ease) infinite;
  }
  @keyframes ping {
    0% {
      transform: scale(1);
      opacity: 0.55;
    }
    80%,
    100% {
      transform: scale(2.8);
      opacity: 0;
    }
  }
</style>
