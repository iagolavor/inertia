<script lang="ts">
  import { blobUrl, type FeedItem } from '$lib/api';
  import Avatar from './Avatar.svelte';
  import FormattedText from './FormattedText.svelte';

  interface Props {
    post: FeedItem & { local_media_preview?: string; delivering?: boolean };
    onopen?: (post: FeedItem) => void;
  }

  let { post, onopen }: Props = $props();

  function timeAgo(iso: string): string {
    const diff = Date.now() - new Date(iso).getTime();
    const mins = Math.floor(diff / 60_000);
    if (mins < 1) return 'agora';
    if (mins < 60) return `há ${mins}m`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `há ${hours}h`;
    return `há ${Math.floor(hours / 24)}d`;
  }

  function timeLeft(iso: string, archived: boolean): string {
    if (archived) return 'guardado';
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
      {#if post.delivering}<span class="delivering-badge">sending…</span>{/if}
      <span class="time">{timeAgo(post.created_at)} · {timeLeft(post.expires_at, post.is_archived)}</span>
    </div>
  </header>

  {#if post.media_ref}
    <button type="button" class="media-btn" onclick={() => onopen?.(post)}>
      <img class="post-media" src={blobUrl(post.media_ref)} alt="" loading="lazy" />
    </button>
  {:else if post.local_media_preview}
    <div class="media-btn local-preview">
      <img class="post-media" src={post.local_media_preview} alt="" />
    </div>
  {/if}

  {#if post.body}
    <FormattedText text={post.body} class="post-body" />
  {/if}

  <div class="post-actions">
    <button type="button" class="action-btn" onclick={() => onopen?.(post)}>
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path
          d="M21 11.5a8.38 8.38 0 01-.9 3.8 8.5 8.5 0 01-7.6 4.7 8.38 8.38 0 01-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 01-.9-3.8 8.5 8.5 0 014.7-7.6 8.38 8.38 0 013.8-.9h.5a8.48 8.48 0 018 8v.5z"
          fill="none"
          stroke="currentColor"
          stroke-width="1.75"
          stroke-linejoin="round"
        />
      </svg>
      {#if (post.comment_count ?? 0) > 0}
        <span>{post.comment_count} comment{(post.comment_count ?? 0) === 1 ? '' : 's'}</span>
      {:else}
        <span>Comment</span>
      {/if}
    </button>
  </div>
</article>

<style>
  .post-card {
    background: var(--post-bg, transparent);
    border: var(--post-border, none);
    border-radius: var(--post-radius, 0);
    padding: var(--post-padding, 1rem 0);
    margin-bottom: var(--post-margin-bottom, 0);
    border-bottom: var(--post-border-bottom, 1px solid var(--border));
  }

  .post-card:last-child {
    border-bottom: var(--post-border-bottom-last, none);
    padding-bottom: var(--post-padding-last, 0);
    margin-bottom: 0;
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

  .delivering-badge {
    font-size: 0.7rem;
    font-weight: 600;
    color: var(--muted);
  }

  .local-preview {
    cursor: default;
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
    border-radius: var(--radius-lg, 8px);
    display: block;
    background: var(--bg);
  }

  .media-btn {
    display: block;
    width: 100%;
    padding: 0;
    margin-bottom: 0.65rem;
    border: none;
    background: none;
    cursor: pointer;
    border-radius: var(--radius-lg, 8px);
    overflow: hidden;
  }

  .media-btn:hover .post-media {
    opacity: 0.92;
  }

  .post-actions {
    margin-top: 0.5rem;
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.25rem 0;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .action-btn svg {
    width: 1.1rem;
    height: 1.1rem;
  }

  .action-btn:hover {
    color: var(--text);
  }

  :global(.post-body) {
    font-size: 0.95rem;
    line-height: 1.5;
  }
</style>
