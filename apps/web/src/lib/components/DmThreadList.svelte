<script lang="ts">
  import type { DmThread } from '$lib/dmThreads';
  import {
    connectionLabel,
    groupDmThreads,
    presenceIndicator,
    presenceTier,
    showsConnectionStatus,
    messageTtlLabel,
    previewText
  } from '$lib/dmThreads';
  import { openConversation } from '$lib/conversation-open';
  import Avatar from '$lib/components/Avatar.svelte';

  interface Props {
    threads: DmThread[];
    loading?: boolean;
  }

  let { threads, loading = false }: Props = $props();

  const groups = $derived(groupDmThreads(threads));

  function openThread(contact: DmThread['contact']) {
    openConversation(contact);
  }
</script>

{#snippet presenceRow(thread: DmThread)}
  {@const tier = presenceTier(thread.contact)}
  {@const showStatus = showsConnectionStatus(thread.contact)}
  <li>
    <button
      type="button"
      class="presence-row"
      class:connected={tier === 'connected'}
      class:reachable={tier === 'reachable'}
      onclick={() => openThread(thread.contact)}
    >
      <div
        class="presence-ring"
        class:connected={tier === 'connected'}
        class:reachable={tier === 'reachable'}
        class:muted={!tier}
      >
        <Avatar seed={thread.contact.signing_pubkey} alt={thread.contact.display_name} size={48} />
      </div>
      <div class="presence-meta">
        <div class="presence-name">{thread.contact.display_name}</div>
        {#if showStatus}
          <div
            class="presence-status connection-status"
            class:connected={tier === 'connected'}
            class:reachable={tier === 'reachable'}
          >
            {presenceIndicator(thread.contact)}
            {connectionLabel(thread.contact)}
          </div>
        {/if}
        <p class="presence-preview">
          {#if thread.lastMessage}
            {previewText(thread.lastMessage.body)}
          {:else}
            <span class="no-msgs">No messages yet</span>
          {/if}
        </p>
      </div>
      {#if thread.lastMessage}
        <span class="ttl-chip">{messageTtlLabel(thread.lastMessage.expires_at)}</span>
      {/if}
    </button>
  </li>
{/snippet}

{#if loading}
  <p class="empty">Loading…</p>
{:else if threads.length === 0}
  <div class="empty-state">
    <p class="empty">No friends yet.</p>
    <a class="btn" href="/connections">Connections</a>
  </div>
{:else}
  {#if groups.connected.length > 0}
    <h2 class="group-label">Connected</h2>
    <ul class="presence-list">
      {#each groups.connected as thread (thread.contact.id)}
        {@render presenceRow(thread)}
      {/each}
    </ul>
  {/if}

  {#if groups.reachable.length > 0}
    <h2 class="group-label">Reachable</h2>
    <ul class="presence-list">
      {#each groups.reachable as thread (thread.contact.id)}
        {@render presenceRow(thread)}
      {/each}
    </ul>
  {/if}

  {#if groups.other.length > 0}
    <ul class="presence-list" class:ungrouped={groups.connected.length > 0 || groups.reachable.length > 0}>
      {#each groups.other as thread (thread.contact.id)}
        {@render presenceRow(thread)}
      {/each}
    </ul>
  {/if}
{/if}

<style>
  .group-label {
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
    margin: 0.85rem 0 0.4rem;
    display: flex;
    align-items: center;
    gap: 0.45rem;
  }

  .group-label:first-child {
    margin-top: 0;
  }

  .group-label::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--border);
  }

  .presence-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .presence-list.ungrouped {
    margin-top: 0.85rem;
  }

  .presence-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    padding: 0.65rem 0.55rem;
    border: none;
    border-radius: 12px;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
    font: inherit;
    margin-bottom: 0.15rem;
  }

  .presence-row:hover {
    background: var(--hover-bg);
  }

  .presence-meta {
    min-width: 0;
    flex: 1;
  }

  .presence-name {
    font-weight: 700;
    font-size: 0.92rem;
    margin-bottom: 0.08rem;
  }

  .presence-status {
    font-size: 0.72rem;
    margin-bottom: 0.12rem;
  }

  .presence-preview {
    margin: 0;
    font-size: 0.78rem;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .no-msgs {
    font-style: italic;
  }

  .empty-state {
    text-align: center;
    padding: 2rem 1rem;
  }

  .empty-state .btn {
    display: inline-flex;
    margin-top: 0.75rem;
  }
</style>
