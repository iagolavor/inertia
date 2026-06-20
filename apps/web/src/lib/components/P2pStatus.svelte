<script lang="ts">
  import StatusDot from './StatusDot.svelte';
  import type { P2pStatus as P2pStatusInfo } from '$lib/api';
  import { formatActivityLine, presencePulseActive } from '$lib/presence.svelte';

  interface Props {
    status: P2pStatusInfo | null;
    loading?: boolean;
    compact?: boolean;
  }

  let { status, loading = false, compact = false }: Props = $props();

  const tone = $derived(loading ? 'loading' : (status?.tone ?? 'off'));

  const activityLines = $derived(
    (status?.recent_activity ?? []).slice(0, 6).map(formatActivityLine)
  );

  /** Header pill — always "P2P"; details live in the tooltip. */
  const displayLabel = $derived(compact ? 'P2P' : (status?.labels.headline ?? 'P2P'));

  const tooltip = $derived.by(() => {
    if (loading || !status) return 'Checking P2P connection…';

    const lines = [
      status.labels.headline,
      '',
      status.labels.node,
      status.labels.relay,
      status.labels.friends,
      status.labels.sync
    ];

    if (activityLines.length > 0) {
      lines.push('', 'Recent activity:');
      for (const line of activityLines) {
        lines.push(`· ${line}`);
      }
    }

    return lines.join('\n');
  });

  const dotOnline = $derived(
    tone === 'online' || tone === 'idle' || tone === 'warn' || tone === 'loading'
  );

  const showPulse = $derived(
    !loading && (presencePulseActive() || tone === 'warn')
  );
</script>

<div
  class="p2p-status"
  class:compact
  class:is-online={tone === 'online' && !loading}
  class:is-idle={tone === 'idle' && !loading}
  class:is-warn={(tone === 'warn' || showPulse) && !loading}
  class:is-offline={(tone === 'off' || tone === 'error') && !loading}
  class:is-loading={loading}
  class:is-pulse={showPulse}
  title={tooltip}
  aria-label={tooltip.replaceAll('\n', '. ')}
>
  <StatusDot
    online={dotOnline}
    {loading}
    pulse={showPulse && !loading}
    size={compact ? 8 : 9}
  />
  <span class="label">{displayLabel}</span>
</div>

{#if !compact && status && activityLines.length > 0 && status.running}
  <ul class="status-layers" aria-label="Connection status">
    <li><span class="layer-key">Node</span> {status.labels.node}</li>
    <li><span class="layer-key">Relay</span> {status.labels.relay.replace(/^Relay: /, '')}</li>
    <li><span class="layer-key">Friends</span> {status.labels.friends.replace(/^Friends: /, '')}</li>
    <li><span class="layer-key">Outbox</span> {status.labels.sync.replace(/^Outbox: /, '')}</li>
  </ul>
  {#if activityLines.length > 0}
    <ul class="activity-strip" aria-live="polite">
      {#each activityLines as line}
        <li>{line}</li>
      {/each}
    </ul>
  {/if}
{/if}

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
    flex-shrink: 0;
    transition: border-color 0.25s ease;
  }

  .p2p-status.is-pulse {
    animation: border-pulse 1.4s ease-in-out infinite;
  }

  @keyframes border-pulse {
    0%,
    100% {
      border-color: color-mix(in srgb, var(--accent) 35%, var(--border));
    }
    50% {
      border-color: color-mix(in srgb, var(--accent) 70%, var(--border));
    }
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

  .status-layers {
    list-style: none;
    margin: 0.35rem 0 0;
    padding: 0;
    font-size: 0.72rem;
    color: var(--muted);
    line-height: 1.45;
  }

  .status-layers li {
    margin-bottom: 0.15rem;
  }

  .layer-key {
    display: inline-block;
    min-width: 3.25rem;
    font-weight: 600;
    color: var(--text);
  }

  .activity-strip {
    list-style: none;
    margin: 0.35rem 0 0;
    padding: 0;
    font-size: 0.72rem;
    color: var(--muted);
    line-height: 1.35;
    max-width: 16rem;
  }

  .activity-strip li {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
