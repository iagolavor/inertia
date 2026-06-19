<script lang="ts">
  import StatusDot from './StatusDot.svelte';
  import type { P2pStatus as P2pStatusInfo } from '$lib/api';

  interface Props {
    status: P2pStatusInfo | null;
    loading?: boolean;
    compact?: boolean;
  }

  let { status, loading = false, compact = false }: Props = $props();

  const connectedCount = $derived(status?.connected_peer_ids.length ?? 0);
  const running = $derived(Boolean(status?.running));
  const online = $derived(running && connectedCount > 0);
  const idle = $derived(running && connectedCount === 0);

  const title = $derived(
    loading
      ? 'Checking P2P…'
      : !running
        ? 'P2P not running — check Settings or restart the API'
        : connectedCount > 0
          ? `P2P connected to ${connectedCount} peer(s)`
          : 'P2P running — waiting for peers'
  );

  const label = $derived(
    loading
      ? 'P2P…'
      : !running
        ? 'P2P off'
        : connectedCount > 0
          ? `P2P ${connectedCount}`
          : 'P2P idle'
  );
</script>

<div
  class="p2p-status"
  class:compact
  class:is-online={online && !loading}
  class:is-idle={idle && !loading}
  class:is-offline={!running && !loading}
  class:is-loading={loading}
  {title}
  aria-label={title}
>
  <StatusDot online={online || idle} {loading} size={compact ? 8 : 9} />
  <span class="label">{label}</span>
</div>

<style>
  .p2p-status {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.35rem 0.65rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: transparent;
    color: var(--text);
    font-size: 0.8rem;
    font-weight: 500;
    line-height: 1;
    white-space: nowrap;
  }

  .p2p-status.compact {
    padding: 0.28rem 0.55rem;
    font-size: 0.75rem;
  }

  .p2p-status.is-online {
    border-color: color-mix(in srgb, var(--success) 45%, var(--border));
  }

  .p2p-status.is-idle {
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border));
  }

  .p2p-status.is-offline {
    border-color: color-mix(in srgb, var(--danger) 45%, var(--border));
  }

  .label {
    color: var(--muted);
  }

  .is-online .label {
    color: var(--success);
  }

  .is-idle .label {
    color: var(--accent);
  }

  .is-offline .label {
    color: var(--danger);
  }
</style>
