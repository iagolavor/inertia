<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { api, type Contact, type ConversationMessage } from '$lib/api';
  import { ApiRequestError } from '$lib/api-errors';
  import Avatar from '$lib/components/Avatar.svelte';
  import FormattedText from '$lib/components/FormattedText.svelte';
  import { identityState } from '$lib/identity.svelte';
  import {
    formatCacheAge,
    readCachedConversation,
    readCachedMessages,
    writeCachedConversation
  } from '$lib/local-cache';
  import { timeAgo } from '$lib/dmThreads';

  let contacts = $state<Contact[]>([]);
  let messages = $state<ConversationMessage[]>([]);
  let loading = $state(true);
  let sending = $state(false);
  let messageBody = $state('');
  let error = $state('');
  let showingCached = $state(false);
  let cacheAge = $state<string | null>(null);

  const contactId = $derived(page.params.contactId);

  const contact = $derived(contacts.find((c) => c.id === contactId) ?? null);

  async function hydrateFromCache() {
    if (!contactId) return false;
    const [msgCache, rosterCache] = await Promise.all([
      readCachedConversation(contactId),
      readCachedMessages()
    ]);
    if (rosterCache) contacts = rosterCache.contacts;
    if (!msgCache) return false;
    messages = msgCache.messages;
    cacheAge = formatCacheAge(msgCache.saved_at);
    showingCached = true;
    return true;
  }

  async function loadConversation() {
    if (!contactId || !identityState.apiOnline) return;
    messages = await api.listConversationMessages(contactId);
    await writeCachedConversation(contactId, messages);
    showingCached = false;
    cacheAge = null;
  }

  async function load() {
    if (!contactId) return;

    if (!identityState.apiOnline) {
      loading = true;
      error = '';
      await hydrateFromCache();
      loading = false;
      return;
    }

    loading = true;
    error = '';
    try {
      contacts = await api.listContacts();
      await loadConversation();
    } catch (e) {
      const hadCache = await hydrateFromCache();
      if (!hadCache) {
        error = e instanceof ApiRequestError ? e.message : 'Failed to load conversation';
      }
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void hydrateFromCache().then(() => load());
  });

  async function send() {
    if (!contactId || !messageBody.trim() || sending || !identityState.apiOnline) return;
    sending = true;
    error = '';
    try {
      await api.sendMessage(contactId, messageBody.trim());
      messageBody = '';
      await loadConversation();
    } catch (e) {
      error = e instanceof ApiRequestError ? e.message : 'Send failed';
    } finally {
      sending = false;
    }
  }

  function deliveryLabel(status: ConversationMessage['delivery_status']): string | null {
    if (!status || status === 'delivered') return null;
    if (status === 'pending') return 'Sending…';
    if (status === 'failed') return 'Not delivered';
    return status;
  }
</script>

<a class="back-link" href="/friends">← Messages</a>

{#if loading}
  <p class="empty">Loading…</p>
{:else if !contact}
  <p class="error">Friend not found.</p>
{:else}
  <header class="chat-header">
    <Avatar seed={contact.signing_pubkey} alt={contact.display_name} size={44} />
    <div class="chat-meta">
      <h1 class="chat-name">{contact.display_name}</h1>
      <span class="badge badge-{contact.connection_state}">{contact.connection_state}</span>
      {#if showingCached && cacheAge}
        <span class="cache-badge">saved · {cacheAge}</span>
      {/if}
    </div>
  </header>

  {#if !identityState.apiOnline}
    <p class="offline-hint muted">Read-only — reconnect the API to send messages.</p>
  {/if}

  <div class="chat-panel">
    {#if messages.length === 0}
      <p class="empty chat-empty">No messages yet. Say hello — they expire after 7 days.</p>
    {:else}
      <ul class="message-list">
        {#each messages as msg (msg.content_id)}
          <li class="message-row" class:own={msg.is_own}>
            <div class="message-bubble">
              <FormattedText text={msg.body} />
              <span class="message-time">
                {timeAgo(msg.at)}
                {#if deliveryLabel(msg.delivery_status)}
                  · {deliveryLabel(msg.delivery_status)}
                {/if}
              </span>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <form class="composer" onsubmit={(e) => { e.preventDefault(); void send(); }}>
    <input
      bind:value={messageBody}
      placeholder={identityState.apiOnline ? 'Message…' : 'API offline — reconnect to send'}
      disabled={sending || !identityState.apiOnline}
      autocomplete="off"
    />
    <button type="submit" class="btn" disabled={sending || !messageBody.trim() || !identityState.apiOnline}>
      {sending ? '…' : 'Send'}
    </button>
  </form>

  {#if error}
    <p class="error">{error}</p>
  {/if}
{/if}

<style>
  .back-link {
    display: inline-block;
    margin-bottom: 1rem;
    font-size: 0.875rem;
    font-weight: 600;
    text-decoration: none;
  }

  .back-link:hover {
    text-decoration: underline;
  }

  .chat-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .chat-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
  }

  .chat-name {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 700;
  }

  .cache-badge {
    font-size: 0.68rem;
    font-weight: 500;
    padding: 0.12rem 0.4rem;
    border-radius: 999px;
    border: 1px solid var(--border);
    color: var(--muted);
  }

  .offline-hint {
    margin: -0.5rem 0 0.75rem;
    font-size: 0.875rem;
  }

  .chat-panel {
    min-height: 280px;
    max-height: 55vh;
    overflow-y: auto;
    padding: 1rem;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    margin-bottom: 0.75rem;
  }

  .chat-empty {
    margin: 0;
    padding: 1rem 0;
  }

  .message-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
  }

  .message-row {
    display: flex;
    justify-content: flex-start;
  }

  .message-row.own {
    justify-content: flex-end;
  }

  .message-bubble {
    max-width: 85%;
    padding: 0.65rem 0.85rem;
    border-radius: 14px 14px 14px 4px;
    background: color-mix(in srgb, var(--accent) 12%, var(--bg));
    border: 1px solid color-mix(in srgb, var(--accent) 18%, var(--border));
  }

  .message-row.own .message-bubble {
    border-radius: 14px 14px 4px 14px;
    background: color-mix(in srgb, var(--accent) 28%, var(--bg));
    border-color: color-mix(in srgb, var(--accent) 35%, var(--border));
  }

  .message-time {
    display: block;
    margin-top: 0.35rem;
    font-size: 0.72rem;
    color: var(--muted);
  }

  .composer {
    display: flex;
    gap: 0.5rem;
  }

  .composer input {
    flex: 1;
    min-width: 0;
    padding: 0.65rem 0.85rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
  }

  .composer .btn {
    flex-shrink: 0;
    border-radius: 999px;
    padding: 0.65rem 1.1rem;
  }
</style>
