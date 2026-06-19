<script lang="ts">
  import StatusDot from './StatusDot.svelte';
  import type { P2pStatus as P2pStatusInfo } from '$lib/api';

  interface Props {
    status: P2pStatusInfo | null;
    loading?: boolean;
    compact?: boolean;
  }

  let { status, loading = false, compact = false }: Props = $props();

  const relayPeerId = $derived(status?.relay_peer_id ?? null);

  const friendCount = $derived(
    status?.connected_peer_ids.filter((id) => id !== relayPeerId).length ?? 0
  );

  const running = $derived(Boolean(status?.running));
  const relayConfigured = $derived(Boolean(status?.relay_configured));
  const relayConnected = $derived(Boolean(status?.relay_connected));
  const relayTcpOk = $derived(status?.relay_tcp_reachable === true);
  const relayTcpFailed = $derived(status?.relay_tcp_reachable === false);

  const friendsOnline = $derived(running && friendCount > 0);
  const relayHealthy = $derived(relayConfigured && relayConnected && relayTcpOk);
  const relayWaiting = $derived(
    running && relayConfigured && !relayConnected && relayTcpOk
  );
  const relayDown = $derived(relayConfigured && relayTcpFailed);

  const online = $derived(friendsOnline || relayHealthy);
  const idle = $derived(running && !friendsOnline && !relayDown && !relayHealthy);

  const title = $derived(
    loading
      ? 'Checking P2P…'
      : !running
        ? 'P2P not running — check Settings or restart the API'
        : [
            friendCount > 0 ? `Connected to ${friendCount} friend(s)` : null,
            relayConfigured
              ? relayConnected
                ? 'Relay: libp2p connected'
                : relayTcpOk
                  ? 'Relay: port open, libp2p not connected yet'
                  : relayTcpFailed
                    ? 'Relay: TCP port unreachable — check VPS firewall'
                    : 'Relay: checking…'
              : 'No relay configured',
            friendCount === 0 && !relayConfigured ? 'Waiting for friends or relay config' : null
          ]
            .filter(Boolean)
            .join(' · ')
  );

  const label = $derived(
    loading
      ? 'P2P…'
      : !running
        ? 'P2P off'
        : relayDown
          ? 'Relay down'
          : friendsOnline
            ? `P2P ${friendCount}`
            : relayHealthy
              ? 'Relay OK'
              : relayWaiting
                ? 'Relay…'
                : 'P2P idle'
  );
</script>

<div
  class="p2p-status"
  class:compact
  class:is-online={online && !loading}
  class:is-idle={idle && !loading && !relayWaiting}
  class:is-warn={relayWaiting && !loading}
  class:is-offline={(!running || relayDown) && !loading}
  class:is-loading={loading}
  {title}
  aria-label={title}
>
  <StatusDot online={online || idle || relayWaiting} {loading} size={compact ? 8 : 9} />
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

  .p2p-status.is-warn {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
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

  .is-warn .label {
    color: var(--accent);
  }

  .is-offline .label {
    color: var(--danger);
  }
</style>
