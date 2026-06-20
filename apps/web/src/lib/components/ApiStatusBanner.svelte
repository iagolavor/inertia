<script lang="ts">
  import type { ApiErrorKind } from '$lib/api-errors';
  import DevSetupActions from '$lib/components/DevSetupActions.svelte';
  import { identityState } from '$lib/identity.svelte';

  interface Props {
    staleLabel?: string | null;
  }

  let { staleLabel = null }: Props = $props();

  const visible = $derived(
    !identityState.loading && (!identityState.apiOnline || identityState.apiError !== null)
  );

  const kind = $derived(identityState.apiError?.kind ?? ('offline' as ApiErrorKind));
  const message = $derived(
    identityState.apiError?.message ??
      (identityState.apiOnline
        ? null
        : 'Local API is offline — your last saved data is shown below.')
  );

  const showSetupActions = $derived(kind === 'offline' || kind === 'timeout');
</script>

{#if visible && message}
  <div
    class="api-banner"
    class:is-offline={kind === 'offline' || kind === 'timeout'}
    class:is-server={kind === 'server'}
    role="status"
    aria-live="polite"
  >
    <div class="api-banner-body">
      <span class="api-banner-icon" aria-hidden="true">
        {#if kind === 'offline' || kind === 'timeout'}⚡{:else}⚠{/if}
      </span>
      <div class="api-banner-text">
        <strong>{kind === 'offline' || kind === 'timeout' ? 'API offline' : 'API problem'}</strong>
        <span>{message}</span>
        {#if staleLabel}
          <span class="stale-hint">Showing saved data · {staleLabel}</span>
        {/if}
        <DevSetupActions compact showHint={false} showWeb={showSetupActions} />
      </div>
    </div>
  </div>
{/if}

<style>
  .api-banner {
    margin: 0 auto 1rem;
    max-width: 720px;
    padding: 0.65rem 0.85rem;
    border-radius: 10px;
    border: 1px solid color-mix(in srgb, var(--warning) 45%, var(--border));
    background: color-mix(in srgb, var(--warning) 10%, var(--surface));
    animation: slide-in 0.35s ease-out;
  }

  @keyframes slide-in {
    from {
      opacity: 0;
      transform: translateY(-6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .api-banner.is-offline {
    border-color: color-mix(in srgb, var(--danger) 40%, var(--border));
    background: color-mix(in srgb, var(--danger) 8%, var(--surface));
  }

  .api-banner.is-server {
    border-color: color-mix(in srgb, var(--warning) 50%, var(--border));
  }

  .api-banner-body {
    display: flex;
    gap: 0.55rem;
    min-width: 0;
  }

  .api-banner-icon {
    flex-shrink: 0;
    font-size: 0.95rem;
    line-height: 1.4;
  }

  .api-banner-text {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    font-size: 0.8125rem;
    line-height: 1.4;
    color: var(--text);
    min-width: 0;
  }

  .api-banner-text strong {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }

  .stale-hint {
    font-size: 0.75rem;
    color: var(--muted);
  }
</style>
