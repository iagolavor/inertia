<script lang="ts">
  import type { Contact } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import { connectionLabel, presenceIndicator, presenceTier, showsConnectionStatus } from '$lib/dmThreads';

  interface Props {
    contact: Contact;
    /** When set, avatar + name block links here (e.g. profile from chat). */
    href?: string | null;
    /** Shown on profile: link to open the conversation. */
    messageHref?: string | null;
    /** Leading back control (e.g. Messages list). */
    backHref?: string | null;
    backLabel?: string;
    detail?: string | null;
    cacheAge?: string | null;
    avatarSize?: number;
  }

  let {
    contact,
    href = null,
    messageHref = null,
    backHref = null,
    backLabel = '← Messages',
    detail = null,
    cacheAge = null,
    avatarSize = 32
  }: Props = $props();

  const tier = $derived(presenceTier(contact));
  const showStatus = $derived(showsConnectionStatus(contact));
</script>

<div class="friend-presence-header" class:has-back={!!backHref}>
  {#if backHref}
    <a class="chat-back-link header-back" href={backHref}>{backLabel}</a>
  {/if}

  <div class="header-trail">
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
</div>

<style>
  .friend-presence-header {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    margin-bottom: 0.75rem;
  }

  .friend-presence-header.has-back {
    justify-content: space-between;
  }

  .header-back {
    flex-shrink: 0;
    margin-bottom: 0;
    white-space: nowrap;
  }

  .header-trail {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
    flex: 1;
  }

  .friend-presence-header.has-back .header-trail {
    flex: 0 1 auto;
    justify-content: flex-end;
  }

  .presence-link {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex: 1;
    min-width: 0;
    text-decoration: none;
    color: inherit;
  }

  .friend-presence-header.has-back .presence-link {
    flex: 0 1 auto;
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

  .friend-presence-header.has-back .presence-meta {
    flex: 0 1 auto;
  }

  .presence-name {
    margin: 0 0 0.08rem;
    font-size: var(--font-size-base);
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
