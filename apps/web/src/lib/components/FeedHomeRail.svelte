<script lang="ts">
  import type { Contact, InboxEntry } from '$lib/api';
  import {
    buildDmThreads,
    contactWithLivePresence,
    presenceIndicator,
    presenceTier,
    previewText,
    timeAgo,
    type DmThread
  } from '$lib/dmThreads';
  import { countUnreadForContact } from '$lib/dm-unread';
  import { openConversation } from '$lib/conversation-open';
  import { identityState } from '$lib/identity.svelte';

  interface Props {
    contacts: Contact[];
    inbox: InboxEntry[];
    unreadTick?: number;
  }

  let { contacts, inbox, unreadTick = 0 }: Props = $props();

  const liveContacts = $derived(
    contacts
      .map((c) =>
        contactWithLivePresence(c, identityState.p2pStatus?.connected_peer_ids)
      )
      .filter((c) => presenceTier(c) != null)
  );

  const connectedCount = $derived(
    liveContacts.filter((c) => presenceTier(c) === 'connected').length
  );
  const reachableCount = $derived(
    liveContacts.filter((c) => presenceTier(c) === 'reachable').length
  );

  const activeChats = $derived.by(() => {
    unreadTick;
    return buildDmThreads(
      contacts.map((c) =>
        contactWithLivePresence(c, identityState.p2pStatus?.connected_peer_ids)
      ),
      inbox
    )
      .filter((t) => t.lastMessage != null)
      .slice(0, 6);
  });

  function unreadFor(thread: DmThread): number {
    unreadTick;
    return countUnreadForContact(
      inbox,
      thread.contact.id,
      thread.contact.signing_pubkey
    );
  }

  function openThread(thread: DmThread) {
    openConversation(thread.contact);
  }

  function openPresence(contact: Contact) {
    openConversation(contact);
  }
</script>

<aside class="home-rail" aria-label="Circle activity">
  <section class="rail-panel">
    <header class="rail-head">
      <h2 class="rail-title">Online now</h2>
      <span class="rail-meta">
        {#if liveContacts.length === 0}
          none
        {:else}
          {connectedCount} connected{#if reachableCount > 0}
            · {reachableCount} reachable
          {/if}
        {/if}
      </span>
    </header>
    {#if liveContacts.length > 0}
      <div class="presence-strip">
        {#each liveContacts as contact (contact.id)}
          {@const tier = presenceTier(contact)}
          <button
            type="button"
            class="presence-chip"
            class:connected={tier === 'connected'}
            class:reachable={tier === 'reachable'}
            onclick={() => openPresence(contact)}
            aria-label={`${contact.display_name}, ${tier}`}
          >
            <span
              class="connection-status"
              class:connected={tier === 'connected'}
              class:reachable={tier === 'reachable'}
              aria-hidden="true"
            >{presenceIndicator(contact)}</span>
            {contact.display_name}
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <section class="rail-panel">
    <header class="rail-head">
      <h2 class="rail-title">Active chats</h2>
    </header>
    {#if activeChats.length === 0}
      <p class="rail-empty">No recent chats yet.</p>
    {:else}
      <ul class="chat-list">
        {#each activeChats as thread (thread.contact.id)}
          {@const unread = unreadFor(thread)}
          <li>
            <button type="button" class="chat-row" onclick={() => openThread(thread)}>
              <div class="chat-main">
                <div class="chat-name-row">
                  <span class="chat-name">{thread.contact.display_name}</span>
                  {#if unread > 0}
                    <span class="chat-unread">{unread} new</span>
                  {/if}
                </div>
                <p class="chat-preview">
                  {previewText(thread.lastMessage?.body ?? '', 56)}
                </p>
              </div>
              <span class="chat-when">{timeAgo(thread.lastActivity)}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
    <p class="rail-footnote">Full inbox is in Messages next to Feed.</p>
  </section>
</aside>

<style>
  .home-rail {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-width: 0;
  }

  .rail-panel {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    padding: 0.75rem 0.85rem;
  }

  .rail-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
    margin-bottom: 0.55rem;
  }

  .rail-title {
    margin: 0;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    color: var(--text);
  }

  .rail-meta {
    flex-shrink: 0;
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-medium);
    color: var(--muted);
    text-align: right;
  }

  .rail-empty,
  .rail-footnote {
    margin: 0;
    font-size: var(--font-size-xs);
    color: var(--muted);
    line-height: 1.4;
  }

  .rail-empty {
    margin: 0.25rem 0 0.5rem;
  }

  .rail-footnote {
    margin-top: 0.65rem;
  }

  .presence-strip {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    max-height: 7.5rem;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
    padding-right: 0.1rem;
  }

  .presence-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.28rem 0.55rem;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg) 55%, var(--surface));
    color: var(--text);
    font: inherit;
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-semibold);
    cursor: pointer;
  }

  .presence-chip.connected {
    border-color: color-mix(in srgb, var(--success) 40%, var(--border));
  }

  .presence-chip.reachable {
    border-color: color-mix(in srgb, var(--warning) 40%, var(--border));
  }

  .presence-chip:hover {
    background: color-mix(in srgb, var(--border) 22%, var(--surface));
  }

  .presence-chip .connection-status {
    font-size: 0.65rem;
    line-height: 1;
  }

  .chat-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .chat-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.55rem;
    width: 100%;
    padding: 0.45rem 0.2rem;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .chat-row:hover {
    background: color-mix(in srgb, var(--border) 22%, transparent);
  }

  .chat-main {
    min-width: 0;
    flex: 1;
  }

  .chat-name-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
  }

  .chat-name {
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
  }

  .chat-unread {
    font-size: var(--font-size-xs);
    font-weight: var(--font-weight-bold);
    padding: 0.12rem 0.35rem;
    border-radius: 999px;
    background: var(--accent);
    color: var(--btn-on-accent, #fff);
  }

  .chat-preview {
    margin: 0.15rem 0 0;
    font-size: var(--font-size-xs);
    color: var(--muted);
    line-height: 1.35;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chat-when {
    flex-shrink: 0;
    font-size: var(--font-size-xs);
    color: var(--muted);
    padding-top: 0.1rem;
  }
</style>
