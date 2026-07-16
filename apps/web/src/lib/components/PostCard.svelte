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

  let expanded = $state(false);
  let commentsOpen = $state(false);
  let comments = $state<PostComment[]>([]);
  let commentsLoading = $state(false);
  let commentBody = $state('');
  let posting = $state(false);
  let error = $state('');
  /** Live count after expand/load; feed `comment_count` can be stale from cache. */
  let loadedCommentCount = $state<number | null>(null);

  const previewThumb = $derived.by(() => {
    if (post.local_media_preview) return post.local_media_preview;
    if (post.media_kind === 'video' && post.thumb_ref) return blobUrl(post.thumb_ref);
    if (post.media_ref) return blobUrl(post.media_ref);
    return null;
  });

  const previewSnippet = $derived.by(() => {
    const flat = post.body.replace(/\s+/g, ' ').trim();
    if (flat) {
      if (flat.length <= 140) return flat;
      return `${flat.slice(0, 139)}…`;
    }
    if (post.media_kind === 'video') return 'Video';
    if (post.media_ref || post.local_media_preview) return 'Photo';
    return 'Post';
  });

  const displayCommentCount = $derived(
    Math.max(post.comment_count ?? 0, loadedCommentCount ?? 0, comments.length)
  );

  const commentLabel = $derived.by(() => {
    const n = displayCommentCount;
    if (n > 0) return `${n} comment${n === 1 ? '' : 's'}`;
    return 'Comment';
  });

  $effect(() => {
    post.content_id;
    loadedCommentCount = null;
  });

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

  function expand() {
    expanded = true;
    void syncCommentCount();
  }

  function collapse() {
    expanded = false;
    commentsOpen = false;
    error = '';
  }

  async function syncCommentCount() {
    try {
      const rows = await api.listPostComments(post.content_id);
      loadedCommentCount = rows.length;
      if (commentsOpen || comments.length > 0) {
        comments = rows;
      }
    } catch {
      // keep last known count
    }
  }

  async function loadComments() {
    commentsLoading = true;
    error = '';
    try {
      comments = await api.listPostComments(post.content_id);
      loadedCommentCount = comments.length;
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
      loadedCommentCount = comments.length;
      commentBody = '';
      oncomment?.();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to post comment';
    } finally {
      posting = false;
    }
  }
</script>

<article class="post-card" class:preview={!expanded} class:expanded>
  {#if !expanded}
    <button type="button" class="preview-hit" onclick={expand}>
      <div class="preview-row">
        {#if previewThumb}
          <div class="preview-thumb-wrap">
            <img class="preview-thumb" src={previewThumb} alt="" loading="lazy" />
            {#if post.media_kind === 'video'}
              <span class="preview-video-badge">Video</span>
            {/if}
          </div>
        {/if}
        <div class="preview-content">
          <div class="preview-meta">
            <Avatar seed={post.author_id} alt={post.author_name} size={22} />
            <span class="author">{post.author_name}</span>
            {#if post.is_own}<span class="own-badge">tu</span>{/if}
            {#if post.delivering}<span class="delivering-badge">sending…</span>{/if}
            <span class="time">{timeAgo(post.created_at)} · {timeLeft(post.expires_at, post.is_archived)}</span>
          </div>
          <div class="preview-bottom">
            <p class="preview-snippet">{previewSnippet}</p>
            <span class="preview-comments">{commentLabel}</span>
          </div>
        </div>
      </div>
    </button>
  {:else}
    <header class="post-header">
      <Avatar seed={post.author_id} alt={post.author_name} size={36} />
      <div class="post-meta">
        <span class="author">{post.author_name}</span>
        {#if post.is_own}<span class="own-badge">tu</span>{/if}
        {#if post.delivering}<span class="delivering-badge">sending…</span>{/if}
        <span class="time">{timeAgo(post.created_at)} · {timeLeft(post.expires_at, post.is_archived)}</span>
      </div>
      <button type="button" class="collapse-btn" onclick={collapse}>Close</button>
    </header>

    <div class="expanded-media">
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
    </div>

    <div class="expanded-footer">
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
          <span>{commentLabel}</span>
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
    </div>
  {/if}
</article>

<style>
  .post-card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg, 10px);
    margin-bottom: 0.65rem;
    overflow: hidden;
  }

  .post-card.preview {
    padding: 0;
  }

  .post-card.expanded {
    display: flex;
    flex-direction: column;
    padding: 0.75rem;
  }

  .post-card:last-child {
    margin-bottom: 0;
  }

  .preview-hit {
    display: block;
    width: 100%;
    padding: 0.65rem 0.75rem;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .preview-hit:hover {
    background: color-mix(in srgb, var(--border) 18%, transparent);
  }

  .preview-row {
    display: flex;
    gap: 0.75rem;
    align-items: stretch;
    min-height: 5.5rem;
  }

  .preview-thumb-wrap {
    position: relative;
    flex-shrink: 0;
    width: 5.5rem;
    height: 5.5rem;
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid var(--border);
    background: var(--bg);
  }

  .preview-thumb {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .preview-video-badge {
    position: absolute;
    left: 0.3rem;
    bottom: 0.3rem;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.65);
    color: #fff;
    font-size: 0.58rem;
    font-weight: 700;
    letter-spacing: 0.03em;
    text-transform: uppercase;
  }

  .preview-content {
    min-width: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 5.5rem;
  }

  .preview-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
    min-width: 0;
  }

  .preview-bottom {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .preview-snippet {
    margin: 0;
    font-size: 0.92rem;
    font-weight: 600;
    line-height: 1.35;
    color: var(--text);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .preview-comments {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--muted);
  }

  .collapse-btn {
    flex-shrink: 0;
    margin-left: auto;
    align-self: center;
    padding: 0.25rem 0.55rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
  }

  .collapse-btn:hover {
    background: color-mix(in srgb, var(--border) 22%, var(--bg));
  }

  .post-header {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    margin-bottom: 0.65rem;
  }

  .post-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.35rem;
    min-width: 0;
    flex: 1;
  }

  .expanded-media {
    min-width: 0;
  }

  .expanded-media:empty {
    display: none;
  }

  .expanded-footer {
    margin-top: auto;
    padding-top: 0.55rem;
  }

  .expanded-footer :global(.post-body) {
    margin: 0;
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
    margin-bottom: 0;
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
  }

  .preview-meta .time {
    width: auto;
  }

  .post-meta .time {
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
    margin-top: 0.35rem;
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .action-btn .chevron {
    width: 0.85rem;
    height: 0.85rem;
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
  }

  :global(.post-body) {
    font-size: 0.95rem;
    line-height: 1.5;
  }

  .comments-panel {
    margin-top: 0.45rem;
    padding: 0.55rem 0 0 0.7rem;
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
    background: var(--bg);
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

  @media (max-width: 480px) {
    .preview-thumb-wrap {
      width: 4.5rem;
      height: 4.5rem;
    }
  }
</style>
