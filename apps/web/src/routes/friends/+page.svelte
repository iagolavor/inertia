<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Contact, type InboxEntry } from '$lib/api';
  import DmThreadList from '$lib/components/DmThreadList.svelte';
  import { buildDmThreads } from '$lib/dmThreads';

  let contacts = $state<Contact[]>([]);
  let inbox = $state<InboxEntry[]>([]);
  let loading = $state(true);
  let error = $state('');

  async function load() {
    loading = true;
    error = '';
    try {
      [contacts, inbox] = await Promise.all([api.listContacts(), api.listInbox()]);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load messages';
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();
  });

  const threads = $derived(buildDmThreads(contacts, inbox));
</script>

<div class="page-head">
  <h1 class="page-title">Friends</h1>
  <a class="head-action" href="/friends/add" aria-label="Add friend">+</a>
</div>
<p class="subtitle">Direct messages with your circle — encrypted and ephemeral.</p>

{#if error}
  <p class="error">{error}</p>
{/if}

<DmThreadList {threads} {loading} />

<style>
  .page-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.25rem;
  }

  .page-title {
    margin: 0;
    font-size: 1.35rem;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .head-action {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text);
    font-size: 1.25rem;
    font-weight: 400;
    line-height: 1;
    text-decoration: none;
  }

  .head-action:hover {
    background: color-mix(in srgb, var(--border) 25%, var(--surface));
    text-decoration: none;
  }

  .subtitle {
    color: var(--muted);
    margin: 0 0 1.25rem;
    font-size: 0.9rem;
  }
</style>
