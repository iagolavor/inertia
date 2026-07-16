<script lang="ts">
  import type { Contact } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import { connectionLabel, presenceIndicator, presenceTier, showsConnectionStatus } from '$lib/dmThreads';

  interface Props {
    contact: Contact;
    /** When set, avatar + name block links here (e.g. profile from chat). */
    href?: string | null;
    /** Shown on profile — link to open the conversation. */
    messageHref?: string | null;
    detail?: string | null;
    cacheAge?: string | null;
    avatarSize?: number;
  }

  let {
    contact,
    href = null,
    messageHref = null,
    detail = null,
    cacheAge = null,
    avatarSize = 48
  }: Props = $props();

  const tier = $derived(presenceTier(contact));
  const showStatus = $derived(showsConnectionStatus(contact));
</script>

<div class="friend-presence-header">
  {#if href}
    <a class="presence-link" {href}>
      <div
        class="presence-ring"
        class:connected={tier === 'connected'}
        class:reachable={tier === 'reachable'}
        class:muted={!tier}
      >
        <Avatar seed={contact.signing_pubkey} alt={contact.display_name} size={avatarSize} />
      </div>
      <div class="presence-meta">
        <h1 class="presence-name">{contact.display_name}</h1>
        {#if showStatus}
          <div
            class="connection-status"
            class:connected={tier === 'connected'}
            class:reachable={tier === 'reachable'}
          >
          {presenceIndicator(contact)}
          {connectionLabel(contact)}
          </div>
        {/if}
        {#if detail}
          <p class="presence-detail">{detail}</p>
        {/if}
      </div>
    </a>
  {:else}
    <div class="presence-link static">
      <div
        class="presence-ring"
        class:connected={tier === 'connected'}
        class:reachable={tier === 'reachable'}
        class:muted={!tier}
      >
        <Avatar seed={contact.signing_pubkey} alt={contact.display_name} size={avatarSize} />
      </div>
      <div class="presence-meta">
        <h1 class="presence-name">{contact.display_name}</h1>
        {#if showStatus}
          <div
            class="connection-status"
            class:connected={tier === 'connected'}
            class:reachable={tier === 'reachable'}
          >
          {presenceIndicator(contact)}
          {connectionLabel(contact)}
          </div>
        {/if}
        {#if detail}
          <p class="presence-detail">{detail}</p>
        {/if}
      </div>
    </div>
  {/if}

  {#if cacheAge}
    <span class="cache-badge">saved · {cacheAge}</span>
  {/if}

  {#if messageHref}
    <a class="header-action" href={messageHref}>Message</a>
  {/if}
</div>

<style>
  .friend-presence-header {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    margin-bottom: 0.75rem;
  }

  .presence-link {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex: 1;
    min-width: 0;
    text-decoration: none;
    color: inherit;
  }

  a.presence-link:hover {
    text-decoration: none;
  }

  a.presence-link:hover .presence-name {
    color: var(--accent);
  }

  .presence-meta {
    min-width: 0;
    flex: 1;
  }

  .presence-name {
    margin: 0 0 0.08rem;
    font-size: var(--font-size-xl);
    font-weight: var(--font-weight-bold);
    letter-spacing: var(--letter-tight);
    line-height: 1.25;
  }

  .connection-status {
    font-size: var(--font-size-xs);
    margin-bottom: 0;
    color: var(--connection-live);
  }

  .connection-status.reachable {
    color: var(--connection-reachable);
  }

  .presence-detail {
    margin: 0.12rem 0 0;
    font-size: var(--font-size-sm);
    color: var(--muted);
  }

  .cache-badge {
    flex-shrink: 0;
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    padding: 0.12rem 0.4rem;
    border-radius: 999px;
    border: 1px solid var(--border);
    color: var(--muted);
  }

  .header-action {
    flex-shrink: 0;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    padding: 0.4rem 0.65rem;
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--surface);
    text-decoration: none;
    color: var(--text);
  }

  .header-action:hover {
    background: var(--hover-bg);
    text-decoration: none;
  }
</style>
