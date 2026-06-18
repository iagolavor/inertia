<script lang="ts">
  import { blobUrl, type FeedItem } from '$lib/api';
  import Avatar from './Avatar.svelte';

  interface Props {
    post: FeedItem;
  }

  let { post }: Props = $props();

  function timeAgo(iso: string): string {
    const diff = Date.now() - new Date(iso).getTime();
    const mins = Math.floor(diff / 60_000);
    if (mins < 1) return 'agora';
    if (mins < 60) return `há ${mins}m`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `há ${hours}h`;
    return `há ${Math.floor(hours / 24)}d`;
  }

  function timeLeft(iso: string): string {
    const diff = new Date(iso).getTime() - Date.now();
    if (diff <= 0) return 'a expirar';
    const hours = Math.floor(diff / 3_600_000);
    if (hours < 1) return `${Math.floor(diff / 60_000)}m restantes`;
    return `${hours}h restantes`;
  }
</script>

<article class="post-card">
  <header class="post-header">
    <Avatar seed={post.author_id} alt={post.author_name} size={36} />
    <div class="post-meta">
      <span class="author">{post.author_name}</span>
      {#if post.is_own}<span class="own-badge">tu</span>{/if}
      <span class="time">{timeAgo(post.created_at)} · {timeLeft(post.expires_at)}</span>
    </div>
  </header>

  {#if post.media_ref}
    <img class="post-media" src={blobUrl(post.media_ref)} alt="" loading="lazy" />
  {/if}

  {#if post.body}
    <p class="post-body">{post.body}</p>
  {/if}
</article>

<style>
  .post-card {
    border-bottom: 1px solid var(--border);
    padding: 1rem 0;
  }

  .post-card:last-child {
    border-bottom: none;
    padding-bottom: 0;
  }

  .post-header {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    margin-bottom: 0.75rem;
  }

  .post-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.35rem;
    min-width: 0;
  }

  .author {
    font-weight: 600;
    font-size: 0.9rem;
  }

  .own-badge {
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--accent);
    text-transform: uppercase;
  }

  .time {
    font-size: 0.75rem;
    color: var(--muted);
    width: 100%;
  }

  .post-media {
    width: 100%;
    max-height: 420px;
    object-fit: cover;
    border-radius: 8px;
    margin-bottom: 0.65rem;
    display: block;
    background: var(--bg);
  }

  .post-body {
    margin: 0;
    font-size: 0.95rem;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
