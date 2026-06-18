<script lang="ts">
  import { goto } from '$app/navigation';
  import type { DmThread } from '$lib/dmThreads';
  import { previewText, timeAgo } from '$lib/dmThreads';
  import Avatar from '$lib/components/Avatar.svelte';

  interface Props {
    threads: DmThread[];
    loading?: boolean;
  }

  let { threads, loading = false }: Props = $props();

  function openThread(contactId: string) {
    void goto(`/friends/${contactId}`);
  }
</script>

{#if loading}
  <p class="empty">Loading…</p>
{:else if threads.length === 0}
  <div class="empty-state">
    <p class="empty">No friends yet.</p>
    <a class="btn" href="/friends/add">Add a friend</a>
  </div>
{:else}
  <ul class="thread-list">
    {#each threads as thread (thread.contact.id)}
      <li>
        <button type="button" class="thread-row" onclick={() => openThread(thread.contact.id)}>
          <Avatar seed={thread.contact.signing_pubkey} alt={thread.contact.display_name} size={48} />
          <div class="thread-meta">
            <div class="thread-top">
              <span class="thread-name">{thread.contact.display_name}</span>
              {#if thread.lastMessage}
                <span class="thread-time">{timeAgo(thread.lastMessage.received_at)}</span>
              {/if}
            </div>
            <p class="thread-preview">
              {#if thread.lastMessage}
                {previewText(thread.lastMessage.body)}
              {:else}
                <span class="no-msgs">No messages yet</span>
              {/if}
            </p>
          </div>
          <span
            class="online-dot online-{thread.contact.connection_state}"
            title={thread.contact.connection_state}
            aria-hidden="true"
          ></span>
        </button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .thread-list {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
    background: var(--surface);
  }

  .thread-row {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    width: 100%;
    padding: 0.85rem 1rem;
    border: none;
    border-bottom: 1px solid var(--border);
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
    font: inherit;
  }

  .thread-list li:last-child .thread-row {
    border-bottom: none;
  }

  .thread-row:hover {
    background: color-mix(in srgb, var(--border) 22%, transparent);
  }

  .thread-meta {
    min-width: 0;
    flex: 1;
  }

  .thread-top {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.15rem;
  }

  .thread-name {
    font-weight: 600;
    font-size: 0.95rem;
  }

  .thread-time {
    flex-shrink: 0;
    font-size: 0.75rem;
    color: var(--muted);
  }

  .thread-preview {
    margin: 0;
    font-size: 0.85rem;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .no-msgs {
    font-style: italic;
  }

  .online-dot {
    flex-shrink: 0;
    width: 0.55rem;
    height: 0.55rem;
    border-radius: 50%;
    background: var(--muted);
  }

  .online-dot.online-online {
    background: var(--success);
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
