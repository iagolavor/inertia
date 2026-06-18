<script lang="ts">
  interface Props {
    online: boolean;
    loading?: boolean;
    size?: number;
    bordered?: boolean;
  }

  let { online, loading = false, size = 10, bordered = false }: Props = $props();

  const label = $derived(loading ? 'checking…' : online ? 'online' : 'offline');
</script>

<span
  class="dot"
  class:online={online && !loading}
  class:offline={!online && !loading}
  class:loading
  class:bordered
  style:width="{size}px"
  style:height="{size}px"
  title={loading ? 'Checking connection…' : online ? 'API online' : 'API offline'}
  role="img"
  aria-label={label}
></span>

<style>
  .dot {
    display: block;
    border-radius: 50%;
    flex-shrink: 0;
    border: none;
    box-shadow: none;
  }

  .dot.bordered {
    border: 2px solid var(--surface);
    box-shadow: 0 0 0 1px var(--border);
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
