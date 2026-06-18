<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Contact, type InboxEntry } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import FormattedText from '$lib/components/FormattedText.svelte';

  let contacts = $state<Contact[]>([]);
  let inbox = $state<InboxEntry[]>([]);
  let selectedRecipient = $state('');
  let messageBody = $state('');
  let loading = $state(true);
  let sending = $state(false);
  let error = $state('');
  let success = $state('');

  onMount(async () => {
    try {
      [contacts, inbox] = await Promise.all([api.listContacts(), api.listInbox()]);
      if (contacts.length > 0) selectedRecipient = contacts[0].id;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load';
    } finally {
      loading = false;
    }
  });

  async function send() {
    if (!selectedRecipient || !messageBody.trim()) {
      error = 'Select a recipient and enter a message';
      return;
    }
    sending = true;
    error = '';
    success = '';
    try {
      const result = await api.sendMessage(selectedRecipient, messageBody.trim());
      success = `Message queued (${result.content_id.slice(0, 8)}...)`;
      messageBody = '';
      inbox = await api.listInbox();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Send failed';
    } finally {
      sending = false;
    }
  }

  function formatTime(iso: string) {
    return new Date(iso).toLocaleString();
  }
</script>

<h1>Messages</h1>
<p class="subtitle">End-to-end encrypted. Expire after 7 days.</p>

<div class="card">
  <h3>Send message</h3>
  {#if contacts.length === 0}
    <p class="empty">Add a friend first.</p>
  {:else}
    <div class="field">
      <label for="recipient">To</label>
      <select
        id="recipient"
        bind:value={selectedRecipient}
        style="padding: 0.6rem; border-radius: 8px; background: var(--bg); color: var(--text); border: 1px solid var(--border);"
      >
        {#each contacts as c}
          <option value={c.id}>{c.display_name}</option>
        {/each}
      </select>
    </div>
    <div class="field">
      <label for="body">Message</label>
      <textarea id="body" bind:value={messageBody} rows="3" placeholder="Say something ephemeral..."></textarea>
    </div>
    {#if error}<p class="error">{error}</p>{/if}
    {#if success}<p style="color: var(--success); font-size: 0.875rem;">{success}</p>{/if}
    <button class="btn" onclick={send} disabled={sending}>
      {sending ? 'Sending...' : 'Send'}
    </button>
  {/if}
</div>

<h3 style="margin-top: 2rem;">Inbox</h3>
{#if loading}
  <p class="empty">Loading...</p>
{:else if inbox.length === 0}
  <p class="empty">No messages yet.</p>
{:else}
  {#each inbox as msg}
    {@const sender = contacts.find((c) => c.id === msg.sender_id)}
    <div class="card message-row">
      <Avatar
        seed={sender?.signing_pubkey ?? msg.sender_id}
        alt={sender?.display_name ?? msg.sender_id}
        size={40}
      />
      <div class="message-meta">
        <div style="display: flex; justify-content: space-between;">
          <strong>From {sender?.display_name ?? `${msg.sender_id.slice(0, 12)}...`}</strong>
          <span style="color: var(--muted); font-size: 0.8rem;">{formatTime(msg.received_at)}</span>
        </div>
        <FormattedText text={msg.body} />
        <p style="color: var(--muted); font-size: 0.75rem; margin: 0;">
          Expires: {formatTime(msg.expires_at)}
        </p>
      </div>
    </div>
  {/each}
{/if}

<style>
  .message-row {
    display: flex;
    align-items: flex-start;
    gap: 0.85rem;
  }

  .message-meta {
    min-width: 0;
    flex: 1;
  }
</style>
