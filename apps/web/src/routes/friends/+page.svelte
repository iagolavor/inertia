<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type Contact, type InboxEntry } from '$lib/api';
  import { ApiRequestError } from '$lib/api-errors';
  import DmThreadList from '$lib/components/DmThreadList.svelte';
  import { buildDmThreads } from '$lib/dmThreads';
  import { identityState } from '$lib/identity.svelte';
  import { formatCacheAge, readCachedMessages, writeCachedMessages } from '$lib/local-cache';
  import { registerInboxRefresh, startInboxPolling, stopInboxPolling } from '$lib/presence.svelte';

  let contacts = $state<Contact[]>([]);
  let inbox = $state<InboxEntry[]>([]);
  let loading = $state(true);
  let error = $state('');
  let showingCached = $state(false);
  let cacheAge = $state<string | null>(null);

  async function hydrateFromCache() {
    const cached = await readCachedMessages();
    if (!cached) return false;
    contacts = cached.contacts;
    inbox = cached.inbox;
    cacheAge = formatCacheAge(cached.saved_at);
    showingCached = true;
    return true;
  }

  async function silentLoad() {
    if (!identityState.identity || !identityState.apiOnline) return;
    try {
      [contacts, inbox] = await Promise.all([api.listContacts(), api.listInbox()]);
      showingCached = false;
      cacheAge = null;
      await writeCachedMessages(contacts, inbox);
    } catch {
      // background refresh — keep last good snapshot
    }
  }

  async function load() {
    if (!identityState.identity) {
      loading = false;
      return;
    }

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
      [contacts, inbox] = await Promise.all([api.listContacts(), api.listInbox()]);
      showingCached = false;
      cacheAge = null;
      await writeCachedMessages(contacts, inbox);
    } catch (e) {
      const hadCache = await hydrateFromCache();
      if (!hadCache) {
        error = e instanceof ApiRequestError ? e.message : 'Failed to load messages';
      }
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void hydrateFromCache().then(() => load());
    startInboxPolling(silentLoad);
    return () => stopInboxPolling();
  });

  const threads = $derived(buildDmThreads(contacts, inbox));
</script>

<div class="page-head">
  <h1 class="page-title">
    Messages
    {#if showingCached && cacheAge}
      <span class="cache-badge">saved · {cacheAge}</span>
    {/if}
  </h1>
  <a class="head-action" href="/friends/add" aria-label="Add friend">+</a>
</div>
<p class="subtitle">Connected — active session. Reachable — seen in the last day.</p>

{#if !identityState.apiOnline && identityState.identity}
  <p class="offline-hint muted">Thread list may be outdated while the API is offline.</p>
{/if}

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
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .cache-badge {
    font-size: 0.68rem;
    font-weight: 500;
    padding: 0.12rem 0.4rem;
    border-radius: 999px;
    border: 1px solid var(--border);
    color: var(--muted);
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
    background: var(--hover-bg);
    text-decoration: none;
  }

  .subtitle {
    color: var(--muted);
    margin: 0 0 1.25rem;
    font-size: 0.9rem;
  }

  .offline-hint {
    margin: -0.5rem 0 1rem;
    font-size: 0.875rem;
  }
</style>
