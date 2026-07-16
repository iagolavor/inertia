<script lang="ts">
  import { tick } from 'svelte';
  import { page } from '$app/state';
  import { api, type Contact, type ConversationMessage } from '$lib/api';
  import { ApiRequestError } from '$lib/api-errors';
  import FriendPresenceHeader from '$lib/components/FriendPresenceHeader.svelte';
  import DeliveryTicks from '$lib/components/DeliveryTicks.svelte';
  import FormattedText from '$lib/components/FormattedText.svelte';
  import {
    contactWithLivePresence,
    createOptimisticMessage,
    isOptimisticMessageId,
    mergeConversationMessages,
    timeAgo
  } from '$lib/dmThreads';
  import { identityState } from '$lib/identity.svelte';
  import { takeConversationPrefetch, type ConversationPrefetch } from '$lib/conversation-open';
  import {
    applyServerConversation,
    refreshConversationSilently,
    seedConversationSnapshot,
    setOpenConversation,
    subscribeConversationSync
  } from '$lib/conversation-sync';
  import {
    formatCacheAge,
    readCachedConversation,
    readCachedMessages
  } from '$lib/local-cache';
  import { registerConversationRefresh } from '$lib/presence.svelte';
  import { markDmThreadRead } from '$lib/dm-unread';

  let contact = $state<Contact | null>(null);
  let durable = $state<ConversationMessage[]>([]);
  let optimistics = $state<ConversationMessage[]>([]);
  let loading = $state(true);
  let sending = $state(false);
  let messageBody = $state('');
  let error = $state('');
  let showingCached = $state(false);
  let cacheAge = $state<string | null>(null);
  let chatPanel = $state<HTMLDivElement | null>(null);
  let loadSeq = 0;

  const contactId = $derived(page.params.contactId);
  const messages = $derived(mergeConversationMessages(durable, optimistics));

  const displayContact = $derived(
    contact && identityState.p2pStatus
      ? contactWithLivePresence(contact, identityState.p2pStatus.connected_peer_ids)
      : contact
  );

  function pruneOptimistics(
    nextDurable: ConversationMessage[],
    pending: ConversationMessage[]
  ): ConversationMessage[] {
    return pending.filter((opt) => {
      if (!isOptimisticMessageId(opt.content_id)) return false;
      return !nextDurable.some(
        (row) =>
          row.is_own &&
          row.body === opt.body &&
          Math.abs(new Date(row.at).getTime() - new Date(opt.at).getTime()) < 60_000
      );
    });
  }

  function waitForLayout(): Promise<void> {
    return new Promise((resolve) => {
      requestAnimationFrame(() => {
        requestAnimationFrame(() => resolve());
      });
    });
  }

  async function scrollToLatest() {
    await tick();
    // Flex chat height often settles one frame after the panel mounts.
    await waitForLayout();
    const panel = chatPanel;
    if (!panel) return;
    // Only scroll the message panel - scrollIntoView can move the page and hide the composer.
    panel.scrollTop = panel.scrollHeight;
  }

  async function hydrateFromCache() {
    if (!contactId) return false;
    const [msgCache, rosterCache] = await Promise.all([
      readCachedConversation(contactId),
      readCachedMessages()
    ]);
    if (rosterCache) {
      contact = rosterCache.contacts.find((c) => c.id === contactId) ?? contact;
    }
    if (!msgCache) return false;
    seedConversationSnapshot(contactId, msgCache.messages);
    cacheAge = formatCacheAge(msgCache.saved_at);
    showingCached = true;
    return true;
  }

  async function loadConversation(serverMessages?: ConversationMessage[]) {
    if (!contactId || !identityState.apiOnline) return;
    const seq = ++loadSeq;
    const resolved =
      serverMessages ?? (await api.listConversationMessages(contactId));
    if (seq !== loadSeq) return;
    applyServerConversation(contactId, resolved);
    showingCached = false;
    cacheAge = null;
  }

  async function fetchContactAndMessages(prefetch?: ConversationPrefetch) {
    if (!contactId) return;

    if (prefetch) {
      contact = prefetch.contact;
      await scrollToLatest();
      const [freshContact, serverMessages] = await Promise.all([
        prefetch.contactPromise,
        prefetch.messagesPromise
      ]);
      contact = freshContact;
      await loadConversation(serverMessages);
      return;
    }

    const [freshContact, serverMessages] = await Promise.all([
      api.getContact(contactId),
      api.listConversationMessages(contactId)
    ]);
    contact = freshContact;
    await loadConversation(serverMessages);
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
      const prefetch = takeConversationPrefetch(contactId);
      await fetchContactAndMessages(prefetch ?? undefined);
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

  $effect(() => {
    const id = contactId;
    if (!id) return;

    optimistics = [];
    setOpenConversation(id);
    markDmThreadRead(id);
    let lastCount = 0;

    registerConversationRefresh(() => refreshConversationSilently(id));

    const unsub = subscribeConversationSync((next) => {
      const grew = next.length > lastCount;
      lastCount = next.length;
      durable = next;
      if (grew) void scrollToLatest();
    });

    void hydrateFromCache().then(() => load());

    return () => {
      unsub();
      registerConversationRefresh(null);
      setOpenConversation(null);
    };
  });

  // Scroll after Loading unmounts and the panel has real flex height (open + cache hydrate).
  $effect(() => {
    if (loading || !chatPanel || messages.length === 0) return;
    void scrollToLatest();
  });

  async function send() {
    if (!contactId || !messageBody.trim() || sending || !identityState.apiOnline) return;

    const body = messageBody.trim();
    const optimistic = createOptimisticMessage(body);
    optimistics = [...optimistics, optimistic];
    messageBody = '';
    error = '';
    sending = true;
    await scrollToLatest();

    try {
      await api.sendMessage(contactId, body);
      await refreshConversationSilently(contactId);
      optimistics = pruneOptimistics(durable, optimistics);
      await scrollToLatest();
    } catch (e) {
      optimistics = optimistics.map((m) =>
        m.content_id === optimistic.content_id ? { ...m, delivery_status: 'failed' } : m
      );
      error = e instanceof ApiRequestError ? e.message : 'Send failed';
    } finally {
      sending = false;
    }
  }

  function messageTime(msg: ConversationMessage): string {
    return isOptimisticMessageId(msg.content_id) ? 'now' : timeAgo(msg.at);
  }
</script>

{#if loading}
  <p class="empty">Loading…</p>
{:else if !displayContact}
  <a class="chat-back-link" href="/messages">← Messages</a>
  <p class="error">Friend not found.</p>
{:else}
  <div class="chat-fill">
  <a class="chat-back-link" href="/messages">← Messages</a>

  <FriendPresenceHeader
    contact={displayContact}
    href="/friends/{displayContact.id}/profile"
    cacheAge={showingCached ? cacheAge : null}
  />

  {#if !identityState.apiOnline}
    <p class="offline-hint muted">Read-only - reconnect the API to send messages.</p>
  {/if}

  <div class="chat-shell">
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
              <span class="msg-meta" class:own={msg.is_own}>
                {messageTime(msg)}
                {#if msg.is_own}
                  <DeliveryTicks
                    status={msg.delivery_status}
                    optimistic={isOptimisticMessageId(msg.content_id)}
                  />
                {/if}
              </span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <form class="composer" onsubmit={(e) => { e.preventDefault(); void send(); }}>
      <input
        class="composer-input"
        bind:value={messageBody}
        placeholder={identityState.apiOnline ? 'Message…' : 'API offline - reconnect to send'}
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
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}
  </div>
{/if}

<style>
  .offline-hint {
    margin: 0 0 0.5rem;
    font-size: 0.875rem;
  }

  .ephemeral-note {
    margin: 0.65rem 0.75rem 0.35rem;
  }

  .chat-fill {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    width: 100%;
  }

  .chat-shell {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    overflow: hidden;
  }

  .chat-panel {
    flex: 1;
    min-height: 8rem;
    overflow-y: auto;
    padding: 0.55rem 0.75rem 0.75rem;
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
    width: fit-content;
    align-self: flex-start;
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
    flex-shrink: 0;
    gap: 0.45rem;
    padding: 0.55rem 0.65rem;
    border-top: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg) 45%, var(--surface));
    position: relative;
    z-index: 1;
  }

  .composer .btn {
    flex-shrink: 0;
    border-radius: var(--composer-radius);
    padding: 0.55rem 0.85rem;
  }
</style>
