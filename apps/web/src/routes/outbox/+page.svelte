<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Contact, type OutboxEntry } from '$lib/api';

  let outbox = $state<OutboxEntry[]>([]);
  let contacts = $state<Contact[]>([]);
  let loading = $state(true);
  let error = $state('');
  let retrying = $state<string | null>(null);

  const contactNames = $derived(
    Object.fromEntries(contacts.map((c) => [c.id, c.display_name]))
  );

  onMount(load);

  async function load() {
    loading = true;
    error = '';
    try {
      [outbox, contacts] = await Promise.all([api.listOutbox(), api.listContacts()]);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load outbox';
    } finally {
      loading = false;
    }
  }

  async function retry(entry: OutboxEntry) {
    retrying = entry.content_id;
    error = '';
    try {
      await api.retryOutbox(entry.content_id, entry.recipient_id);
      await load();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Retry failed';
    } finally {
      retrying = null;
    }
  }

  function formatTime(iso: string) {
    return new Date(iso).toLocaleString();
  }
</script>

<h1>Outbox</h1>
<p class="subtitle">Pending and failed deliveries — retry or let them expire.</p>

{#if error}<p class="error">{error}</p>{/if}

{#if loading}
  <p class="empty">Loading...</p>
{:else if outbox.length === 0}
  <p class="empty">Outbox is empty. All messages delivered or expired.</p>
{:else}
  {#each outbox as entry}
    <div class="card">
      <div style="display: flex; justify-content: space-between; align-items: center;">
        <strong>To {contactNames[entry.recipient_id] ?? entry.recipient_id}</strong>
        <span class="badge badge-{entry.status}">{entry.status}</span>
      </div>
      <p style="color: var(--muted); font-size: 0.8rem; margin: 0.5rem 0;">
        ID: {entry.content_id.slice(0, 12)}... · Retries: {entry.retry_count}
      </p>
      <p style="color: var(--muted); font-size: 0.8rem; margin: 0.5rem 0;">
        Expires: {formatTime(entry.expires_at)}
      </p>
      {#if entry.status === 'failed' || entry.status === 'pending' || entry.status === 'sent'}
        <button
          class="btn btn-secondary"
          onclick={() => retry(entry)}
          disabled={retrying === entry.content_id}
        >
          {retrying === entry.content_id ? 'Retrying...' : 'Retry send'}
        </button>
      {/if}
    </div>
  {/each}
{/if}
