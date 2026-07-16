<script lang="ts">
  import StatusDot from './StatusDot.svelte';
  import { toggleApiBridge } from '$lib/identity.svelte';

  interface Props {
    online: boolean;
    loading?: boolean;
    compact?: boolean;
  }

  let { online, loading = false, compact = false }: Props = $props();

  const title = $derived(
    loading
      ? 'Checking API…'
      : online
        ? 'Click to disconnect the API bridge'
        : 'API offline - use Start API in the banner, or click here to Retry'
  );
  const label = $derived(loading ? 'API' : online ? 'API' : 'Off');
</script>

<button
  type="button"
  class="online-status"
  class:compact
  class:is-online={online && !loading}
  class:is-offline={!online && !loading}
  class:is-loading={loading}
  {title}
  aria-pressed={online}
  aria-label={title}
  disabled={loading}
  onclick={(e) => {
    e.stopPropagation();
    toggleApiBridge();
  }}
>
  <StatusDot {online} {loading} size={compact ? 8 : 9} />
  <span class="label">{label}</span>
</button>

<style>
  .online-status {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.35rem 0.65rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 0.8rem;
    font-weight: 500;
    line-height: 1;
    white-space: nowrap;
    flex-shrink: 0;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }

  .online-status:hover:not(:disabled) {
    background: color-mix(in srgb, var(--border) 35%, transparent);
  }

  .online-status:disabled {
    cursor: wait;
    opacity: 0.75;
  }

  .online-status.compact {
    padding: 0.28rem 0.55rem;
    font-size: 0.75rem;
  }

  .online-status.is-online {
    border-color: color-mix(in srgb, var(--success) 45%, var(--border));
  }

  .online-status.is-offline {
    border-color: color-mix(in srgb, var(--danger) 45%, var(--border));
  }

  .label {
    color: var(--muted);
  }

  .is-online .label {
    color: var(--success);
  }

  .is-offline .label {
    color: var(--danger);
  }
</style>
