<script lang="ts">
  import StatusDot from './StatusDot.svelte';
  import TipPanel from './TipPanel.svelte';
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

  /** Header pill - always "P2P"; details live in the tap panel. */
  const displayLabel = $derived(compact ? 'P2P' : (status?.labels.headline ?? 'P2P'));

  const summaryLabel = $derived.by(() => {
    if (loading || !status) return 'P2P connection status';
    return `P2P: ${status.labels.headline}`;
  });

  const dotOnline = $derived(
    tone === 'online' || tone === 'idle' || tone === 'warn' || tone === 'loading'
  );

  const showPulse = $derived(!loading && (presencePulseActive() || tone === 'warn'));
</script>

<TipPanel label={summaryLabel} align="end">
  {#snippet trigger()}
    <span
      class="p2p-status"
      class:compact
      class:is-online={tone === 'online' && !loading}
      class:is-idle={tone === 'idle' && !loading}
      class:is-warn={(tone === 'warn' || showPulse) && !loading}
      class:is-offline={(tone === 'off' || tone === 'error') && !loading}
      class:is-loading={loading}
      class:is-pulse={showPulse}
    >
      <StatusDot
        online={dotOnline}
        {loading}
        pulse={showPulse && !loading}
        size={compact ? 8 : 9}
      />
      <span class="label">{displayLabel}</span>
    </span>
  {/snippet}

  <div class="panel-body">
    {#if loading || !status}
      <p class="panel-title">Checking P2P…</p>
    {:else}
      <p class="panel-title">{status.labels.headline}</p>
      <ul class="status-layers" aria-label="Connection status">
        <li><span class="layer-key">Node</span> {status.labels.node}</li>
        <li>
          <span class="layer-key">Relay</span>
          {status.labels.relay.replace(/^Relay: /, '')}
        </li>
        <li>
          <span class="layer-key">Friends</span>
          {status.labels.friends.replace(/^Friends: /, '')}
        </li>
        <li>
          <span class="layer-key">Outbox</span>
          {status.labels.sync.replace(/^Outbox: /, '')}
        </li>
      </ul>
      {#if activityLines.length > 0 && status.running}
        <ul class="activity-strip" aria-live="polite">
          {#each activityLines as line}
            <li>{line}</li>
          {/each}
        </ul>
      {/if}
    {/if}
  </div>
</TipPanel>

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

  .panel-body {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .panel-title {
    margin: 0;
    font-size: 0.82rem;
    font-weight: 700;
    line-height: 1.3;
  }

  .status-layers {
    list-style: none;
    margin: 0;
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
    margin: 0;
    padding: 0.4rem 0 0;
    border-top: 1px solid var(--border);
    font-size: 0.72rem;
    color: var(--muted);
    line-height: 1.35;
  }

  .activity-strip li {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
