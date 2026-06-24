<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { page } from '$app/state';
  import { api, type Contact, type ConversationMessage } from '$lib/api';
  import { ApiRequestError } from '$lib/api-errors';
  import FriendPresenceHeader from '$lib/components/FriendPresenceHeader.svelte';
  import FormattedText from '$lib/components/FormattedText.svelte';
  import {
    createOptimisticMessage,
    isOptimisticMessageId,
    mergeConversationMessages,
    timeAgo
  } from '$lib/dmThreads';
  import { identityState } from '$lib/identity.svelte';
  import {
    formatCacheAge,
    readCachedConversation,
    readCachedMessages,
    writeCachedConversation
  } from '$lib/local-cache';
  import { startConversationPolling, stopConversationPolling } from '$lib/presence.svelte';

  let contacts = $state<Contact[]>([]);
  let messages = $state<ConversationMessage[]>([]);
  let loading = $state(true);
  let sending = $state(false);
  let messageBody = $state('');
  let error = $state('');
  let showingCached = $state(false);
  let cacheAge = $state<string | null>(null);
  let chatPanel = $state<HTMLDivElement | null>(null);

  const contactId = $derived(page.params.contactId);

  const contact = $derived(contacts.find((c) => c.id === contactId) ?? null);

  async function scrollToLatest() {
    await tick();
    if (!chatPanel) return;
    chatPanel.scrollTop = chatPanel.scrollHeight;
  }

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
    const optimistic = messages.filter((m) => isOptimisticMessageId(m.content_id));
    const serverMessages = await api.listConversationMessages(contactId);
    messages = mergeConversationMessages(serverMessages, optimistic);
    await writeCachedConversation(contactId, messages.filter((m) => !isOptimisticMessageId(m.content_id)));
    showingCached = false;
    cacheAge = null;
  }

  async function silentRefresh() {
    if (!contactId || !identityState.apiOnline) return;
    try {
      contacts = await api.listContacts();
      await loadConversation();
    } catch {
      // background refresh — keep last good snapshot
    }
  }

  async function load() {
    if (!contactId) return;

    if (!identityState.apiOnline) {
      loading = true;
      error = '';
      await hydrateFromCache();
      loading = false;
      await scrollToLatest();
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
      await scrollToLatest();
    }
  }

  onMount(() => {
    void hydrateFromCache().then(() => load());
    startConversationPolling(silentRefresh);
    return () => stopConversationPolling();
  });

  async function send() {
    if (!contactId || !messageBody.trim() || sending || !identityState.apiOnline) return;

    const body = messageBody.trim();
    const optimistic = createOptimisticMessage(body);
    messages = [...messages, optimistic];
    messageBody = '';
    error = '';
    sending = true;
    await scrollToLatest();

    try {
      await api.sendMessage(contactId, body);
      await loadConversation();
      await scrollToLatest();
    } catch (e) {
      messages = messages.map((m) =>
        m.content_id === optimistic.content_id ? { ...m, delivery_status: 'failed' } : m
      );
      error = e instanceof ApiRequestError ? e.message : 'Send failed';
    } finally {
      sending = false;
    }
  }

  function messageMeta(msg: ConversationMessage): string {
    const who = msg.is_own ? 'You' : (contact?.display_name ?? 'Them');
    const time = isOptimisticMessageId(msg.content_id) ? 'now' : timeAgo(msg.at);
    const delivery = deliveryLabel(msg.delivery_status);
    return delivery ? `${who} · ${time} · ${delivery}` : `${who} · ${time}`;
  }

  function deliveryLabel(status: ConversationMessage['delivery_status']): string | null {
    if (!status || status === 'delivered') return null;
    if (status === 'pending') return 'Sending…';
    if (status === 'failed') return 'Not delivered';
    return status;
  }
</script>

{#if loading}
  <p class="empty">Loading…</p>
{:else if !contact}
  <a class="chat-back-link" href="/friends">← Messages</a>
  <p class="error">Friend not found.</p>
{:else}
  <a class="chat-back-link" href="/friends">← Messages</a>

  <FriendPresenceHeader
    {contact}
    href="/friends/{contact.id}/profile"
    cacheAge={showingCached ? cacheAge : null}
  />

  {#if !identityState.apiOnline}
    <p class="offline-hint muted">Read-only — reconnect the API to send messages.</p>
  {/if}

  <p class="ephemeral-note">Messages auto-delete after 7 days</p>

  <div class="chat-panel" bind:this={chatPanel}>
    {#if messages.length === 0}
      <p class="empty chat-empty">No messages yet. Say hello.</p>
    {:else}
      <ul class="message-list">
        {#each messages as msg (msg.content_id)}
          <li class="stack-msg" class:own={msg.is_own} class:pending={msg.delivery_status === 'pending'}>
            <div class="msg-body" class:own={msg.is_own}>
              <FormattedText text={msg.body} />
            </div>
            <span class="msg-meta" class:own={msg.is_own}>{messageMeta(msg)}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <form class="composer" onsubmit={(e) => { e.preventDefault(); void send(); }}>
    <input
      class="composer-input"
      bind:value={messageBody}
      placeholder={identityState.apiOnline ? 'Message…' : 'API offline — reconnect to send'}
      disabled={!identityState.apiOnline}
      autocomplete="off"
    />
    <button
      type="submit"
      class="btn btn-secondary"
      disabled={sending || !messageBody.trim() || !identityState.apiOnline}
    >
      Send
    </button>
  </form>

  {#if error}
    <p class="error">{error}</p>
  {/if}
{/if}

<style>
  .offline-hint {
    margin: 0 0 0.5rem;
    font-size: 0.875rem;
  }

  .ephemeral-note {
    margin: 0 0 0.75rem;
  }

  .chat-panel {
    min-height: 280px;
    max-height: 55vh;
    overflow-y: auto;
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
    gap: 0.55rem;
  }

  .stack-msg {
    max-width: 88%;
  }

  .stack-msg.own {
    align-self: flex-end;
    margin-left: auto;
  }

  .stack-msg.pending .msg-body.own {
    opacity: 0.88;
  }

  .composer {
    display: flex;
    gap: 0.45rem;
  }

  .composer .btn {
    flex-shrink: 0;
    border-radius: var(--composer-radius);
    padding: 0.55rem 0.85rem;
  }
</style>
