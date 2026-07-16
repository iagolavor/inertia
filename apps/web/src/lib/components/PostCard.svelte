<script lang="ts">
  import { api, blobUrl, type FeedItem, type PostComment } from '$lib/api';
  import Avatar from './Avatar.svelte';
  import FormattedText from './FormattedText.svelte';
  import VideoMedia from './VideoMedia.svelte';

  interface Props {
    post: FeedItem & {
      local_media_preview?: string;
      delivering?: boolean;
      media_kind?: 'photo' | 'video' | null;
      media_ready?: boolean;
    };
    disabled?: boolean;
    oncomment?: () => void;
  }

  let { post, disabled = false, oncomment }: Props = $props();

  let commentsOpen = $state(false);
  let comments = $state<PostComment[]>([]);
  let commentsLoading = $state(false);
  let commentBody = $state('');
  let posting = $state(false);
  let error = $state('');

  function timeAgo(iso: string): string {
    const diff = Date.now() - new Date(iso).getTime();
    const mins = Math.floor(diff / 60_000);
    if (mins < 1) return 'agora';
    if (mins < 60) return `há ${mins}m`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `há ${hours}h`;
    return `há ${Math.floor(hours / 24)}d`;
  }

  function commentTimeAgo(iso: string): string {
    const diff = Date.now() - new Date(iso).getTime();
    const mins = Math.floor(diff / 60_000);
    if (mins < 1) return 'now';
    if (mins < 60) return `${mins}m`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h`;
    return `${Math.floor(hours / 24)}d`;
  }

  function timeLeft(iso: string, archived: boolean): string {
    if (archived) return 'guardado';
    const diff = new Date(iso).getTime() - Date.now();
    if (diff <= 0) return 'a expirar';
    const hours = Math.floor(diff / 3_600_000);
    if (hours < 1) return `${Math.floor(diff / 60_000)}m restantes`;
    return `${hours}h restantes`;
  }

  async function loadComments() {
    commentsLoading = true;
    error = '';
    try {
      comments = await api.listPostComments(post.content_id);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load comments';
    } finally {
      commentsLoading = false;
    }
  }

  function toggleComments() {
    commentsOpen = !commentsOpen;
    if (commentsOpen) {
      void loadComments();
    } else {
      error = '';
    }
  }

  async function submitComment() {
    if (!commentBody.trim() || posting || disabled) return;
    posting = true;
    error = '';
    try {
      const comment = await api.addPostComment(post.content_id, commentBody.trim());
      comments = [...comments, comment];
      commentBody = '';
      oncomment?.();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to post comment';
    } finally {
      posting = false;
    }
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

  {#if post.media_kind === 'video' && post.media_ref}
    <VideoMedia
      rootHash={post.media_ref}
      thumbRef={post.thumb_ref ?? post.media_ref}
      mediaReady={post.media_ready ?? false}
    />
  {:else if post.media_kind === 'video' && post.local_media_preview}
    <div class="media-wrap local-preview">
      <img class="post-media" src={post.local_media_preview} alt="" />
      <span class="video-badge">Video</span>
    </div>
  {:else if post.media_ref}
    <div class="media-wrap">
      <img class="post-media" src={blobUrl(post.media_ref)} alt="" loading="lazy" />
    </div>
  {:else if post.local_media_preview}
    <div class="media-wrap local-preview">
      <img class="post-media" src={post.local_media_preview} alt="" />
    </div>
  {/if}

  {#if post.body}
    <FormattedText text={post.body} class="post-body" />
  {/if}

  <div class="post-actions">
    <button
      type="button"
      class="action-btn"
      class:open={commentsOpen}
      aria-expanded={commentsOpen}
      aria-controls={`comments-${post.content_id}`}
      aria-label={commentsOpen ? 'Hide comments' : 'Show comments'}
      onclick={toggleComments}
    >
      <svg class="comment-icon" viewBox="0 0 24 24" aria-hidden="true">
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
      <svg class="chevron" viewBox="0 0 24 24" aria-hidden="true">
        <path
          d="M6 9l6 6 6-6"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
  </div>

  {#if commentsOpen}
    <section class="comments-panel" id={`comments-${post.content_id}`} aria-label="Comments">
      {#if commentsLoading && comments.length === 0}
        <p class="muted">Loading comments…</p>
      {:else if comments.length === 0}
        <p class="muted">No comments yet.</p>
      {:else}
        <ul class="comments-list">
          {#each comments as comment (comment.id)}
            <li class="comment">
              <Avatar seed={comment.author_id} alt={comment.author_name} size={24} />
              <div class="comment-body">
                <div class="comment-meta">
                  <span class="comment-author">{comment.author_name}</span>
                  <span class="comment-time">{commentTimeAgo(comment.created_at)}</span>
                </div>
                <FormattedText text={comment.body} class="comment-text" />
              </div>
            </li>
          {/each}
        </ul>
      {/if}

      {#if error}
        <p class="error">{error}</p>
      {/if}

      <div class="comment-composer">
        <input
          bind:value={commentBody}
          placeholder="Add a comment…"
          disabled={posting || disabled || commentsLoading}
          onkeydown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault();
              void submitComment();
            }
          }}
        />
        <button
          type="button"
          class="send-comment"
          onclick={() => void submitComment()}
          disabled={posting || disabled || commentsLoading || !commentBody.trim()}
        >
          Post
        </button>
      </div>
    </section>
  {/if}
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

  .media-wrap {
    display: block;
    width: 100%;
    margin-bottom: 0.65rem;
    border-radius: var(--radius-lg, 8px);
    overflow: hidden;
  }

  .local-preview {
    position: relative;
  }

  .video-badge {
    position: absolute;
    left: 0.5rem;
    bottom: 0.5rem;
    padding: 0.15rem 0.45rem;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.65);
    color: #fff;
    font-size: 0.65rem;
    font-weight: 700;
    letter-spacing: 0.04em;
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
    border-radius: var(--radius-lg, 8px);
    display: block;
    background: var(--bg);
  }

  .post-actions {
    margin-top: 0.5rem;
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0.45rem 0.3rem 0;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .action-btn .comment-icon {
    width: 1.1rem;
    height: 1.1rem;
  }

  .action-btn .chevron {
    width: 0.85rem;
    height: 0.85rem;
    margin-left: 0.05rem;
    transition: transform 0.15s ease;
  }

  .action-btn.open .chevron {
    transform: rotate(180deg);
  }

  .action-btn:hover {
    color: var(--text);
  }

  .action-btn.open {
    color: var(--text);
    background: color-mix(in srgb, var(--border) 28%, transparent);
  }

  :global(.post-body) {
    font-size: 0.95rem;
    line-height: 1.5;
  }

  .comments-panel {
    margin-top: 0.55rem;
    padding: 0.7rem 0 0.15rem 0.7rem;
    border-top: 1px solid var(--border);
    border-left: 2px solid color-mix(in srgb, var(--accent) 45%, var(--border));
  }

  .muted {
    margin: 0 0 0.65rem;
    font-size: 0.85rem;
    color: var(--muted);
  }

  .comments-list {
    list-style: none;
    margin: 0 0 0.75rem;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }

  .comment {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
  }

  .comment-body {
    min-width: 0;
    flex: 1;
  }

  .comment-meta {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    min-height: 24px;
  }

  .comment-author {
    font-weight: 600;
    font-size: 0.82rem;
    line-height: 1.2;
  }

  .comment-time {
    font-size: 0.7rem;
    line-height: 1.2;
    color: var(--muted);
  }

  :global(.comment-text) {
    display: block;
    margin-top: 0.1rem;
    font-size: 0.88rem;
    line-height: 1.4;
  }

  .comment-composer {
    display: flex;
    gap: 0.45rem;
    align-items: center;
  }

  .comment-composer input {
    flex: 1;
    min-width: 0;
    padding: 0.45rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 0.85rem;
  }

  .comment-composer input::placeholder {
    color: var(--muted);
  }

  .send-comment {
    padding: 0.45rem 0.75rem;
    border: none;
    border-radius: 8px;
    background: var(--accent);
    color: var(--btn-on-accent, #fff);
    font: inherit;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .send-comment:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .error {
    margin: 0 0 0.55rem;
    font-size: 0.85rem;
    color: var(--danger);
  }
</style>
