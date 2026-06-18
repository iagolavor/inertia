<script lang="ts">
  interface Props {
    online: boolean;
    loading?: boolean;
    size?: number;
    showLabel?: boolean;
  }

  let { online, loading = false, size = 10, showLabel = false }: Props = $props();

  const title = $derived(
    loading ? 'Checking connection…' : online ? 'API online' : 'API offline'
  );
  const label = $derived(loading ? 'checking…' : online ? 'online' : 'offline');
</script>

<span class="status" {title}>
  <span
    class="dot"
    class:online={online && !loading}
    class:offline={!online && !loading}
    class:loading
    style:width="{size}px"
    style:height="{size}px"
  ></span>
  {#if showLabel}
    <span class="label">{label}</span>
  {/if}
</span>

<style>
  .status {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    line-height: 1;
  }

  .dot {
    display: block;
    border-radius: 50%;
    flex-shrink: 0;
    border: 2px solid var(--surface);
    box-shadow: 0 0 0 1px var(--border);
    align-self: center;
  }

  .dot.online {
    background: var(--success);
  }

  .dot.offline {
    background: var(--danger);
  }

  .dot.loading {
    background: var(--muted);
    animation: pulse 1.2s ease-in-out infinite;
  }

  .label {
    font-size: 0.8rem;
    color: var(--muted);
    font-weight: 500;
    line-height: 1;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 0.45;
    }
    50% {
      opacity: 1;
    }
  }
</style>
