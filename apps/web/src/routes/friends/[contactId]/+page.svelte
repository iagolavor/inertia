<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { api, type Contact, type InboxEntry } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import FormattedText from '$lib/components/FormattedText.svelte';
  import { timeAgo } from '$lib/dmThreads';

  let contacts = $state<Contact[]>([]);
  let inbox = $state<InboxEntry[]>([]);
  let loading = $state(true);
  let sending = $state(false);
  let messageBody = $state('');
  let error = $state('');

  const contactId = $derived(page.params.contactId);

  const contact = $derived(contacts.find((c) => c.id === contactId) ?? null);

  const messages = $derived(
    inbox
      .filter(
        (entry) =>
          entry.content_type === 'message' &&
          (entry.sender_id === contactId ||
            entry.sender_id === contact?.signing_pubkey)
      )
      .sort(
        (a, b) => new Date(a.received_at).getTime() - new Date(b.received_at).getTime()
      )
  );

  async function load() {
    loading = true;
    error = '';
    try {
      [contacts, inbox] = await Promise.all([api.listContacts(), api.listInbox()]);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load conversation';
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();
  });

  async function send() {
    if (!contactId || !messageBody.trim() || sending) return;
    sending = true;
    error = '';
    try {
      await api.sendMessage(contactId, messageBody.trim());
      messageBody = '';
      inbox = await api.listInbox();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Send failed';
    } finally {
      sending = false;
    }
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
    </div>
  </header>

  <div class="chat-panel">
    {#if messages.length === 0}
      <p class="empty chat-empty">No messages yet. Say hello — they expire after 7 days.</p>
    {:else}
      <ul class="message-list">
        {#each messages as msg (msg.content_id)}
          <li class="message-bubble">
            <FormattedText text={msg.body} />
            <span class="message-time">{timeAgo(msg.received_at)}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <form class="composer" onsubmit={(e) => { e.preventDefault(); void send(); }}>
    <input
      bind:value={messageBody}
      placeholder="Message…"
      disabled={sending}
      autocomplete="off"
    />
    <button type="submit" class="btn" disabled={sending || !messageBody.trim()}>
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

  .message-bubble {
    max-width: 85%;
    padding: 0.65rem 0.85rem;
    border-radius: 14px 14px 14px 4px;
    background: color-mix(in srgb, var(--accent) 12%, var(--bg));
    border: 1px solid color-mix(in srgb, var(--accent) 18%, var(--border));
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
