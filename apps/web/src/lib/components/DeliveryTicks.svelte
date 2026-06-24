<script lang="ts">
  import type { ConversationMessage } from '$lib/api';
  import { deliveryTickState } from '$lib/dmThreads';

  interface Props {
    status: ConversationMessage['delivery_status'];
    optimistic?: boolean;
  }

  let { status, optimistic = false }: Props = $props();

  const tickState = $derived(deliveryTickState(status, optimistic));
</script>

{#if tickState}
  <span
    class="delivery-ticks"
    class:sending={tickState === 'sending'}
    class:sent={tickState === 'sent'}
    class:delivered={tickState === 'delivered'}
    class:failed={tickState === 'failed'}
    aria-label={tickState === 'sending'
      ? 'Sending'
      : tickState === 'sent'
        ? 'Sent'
        : tickState === 'delivered'
          ? 'Delivered'
          : 'Not delivered'}
  >
    {#if tickState === 'delivered'}
      <span class="tick" aria-hidden="true">✓</span><span class="tick overlap" aria-hidden="true">✓</span>
    {:else if tickState === 'sent'}
      <span class="tick" aria-hidden="true">✓</span>
    {:else if tickState === 'sending'}
      <span class="tick-pending" aria-hidden="true">◷</span>
    {:else if tickState === 'failed'}
      <span class="tick-failed" aria-hidden="true">!</span>
    {/if}
  </span>
{/if}

<style>
  .delivery-ticks {
    display: inline-flex;
    align-items: center;
    margin-left: 0.2rem;
    vertical-align: baseline;
    line-height: 1;
  }

  .tick {
    font-size: 0.72rem;
    font-weight: 700;
  }

  .tick.overlap {
    margin-left: -0.34rem;
  }

  .tick-pending,
  .tick-failed {
    font-size: 0.62rem;
    font-weight: 700;
  }

  .delivery-ticks.sending {
    color: var(--msg-meta);
    opacity: 0.7;
  }

  .delivery-ticks.sent {
    color: var(--msg-meta);
  }

  .delivery-ticks.delivered {
    color: var(--accent);
  }

  .delivery-ticks.failed {
    color: var(--danger);
  }
</style>
