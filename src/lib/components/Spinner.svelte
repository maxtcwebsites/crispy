<script lang="ts">
  interface Props {
    size?: number;
    stroke?: number;
    label?: string;
  }
  let { size = 16, stroke = 2, label = 'Loading' }: Props = $props();
</script>

<svg class="spinner" viewBox="0 0 24 24" width={size} height={size} role="img" aria-label={label}>
  <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-opacity="0.18" stroke-width={stroke} />
  <path d="M21 12a9 9 0 0 0-9-9" fill="none" stroke="currentColor" stroke-width={stroke} stroke-linecap="round" />
</svg>

<style>
  .spinner {
    animation: spin 0.8s linear infinite;
    flex: none;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
