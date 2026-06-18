<script lang="ts">
  import { api, blobUrl, type FeedItem, type PostComment } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import FormattedText from '$lib/components/FormattedText.svelte';

  interface Props {
    open: boolean;
    post: FeedItem | null;
    disabled?: boolean;
    onclose?: () => void;
    oncomment?: () => void;
  }

  let { open, post, disabled = false, onclose, oncomment }: Props = $props();

  let comments = $state<PostComment[]>([]);
  let loading = $state(false);
  let commentBody = $state('');
  let posting = $state(false);
  let error = $state('');

  $effect(() => {
    if (open && post) {
      void loadComments(post.content_id);
    } else {
      comments = [];
      commentBody = '';
      error = '';
    }
  });

  async function loadComments(postId: string) {
    loading = true;
    error = '';
    try {
      comments = await api.listPostComments(postId);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load comments';
    } finally {
      loading = false;
    }
  }

  function close() {
    onclose?.();
  }

  function onBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) close();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) close();
  }

  async function submitComment() {
    if (!post || !commentBody.trim() || posting || disabled) return;
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

  function timeAgo(iso: string): string {
    const diff = Date.now() - new Date(iso).getTime();
    const mins = Math.floor(diff / 60_000);
    if (mins < 1) return 'now';
    if (mins < 60) return `${mins}m`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h`;
    return `${Math.floor(hours / 24)}d`;
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open && post}
  <div class="backdrop" role="presentation" onclick={onBackdropClick}>
    <div class="modal" role="dialog" aria-modal="true" aria-label="Post">
      <header class="modal-header">
        <button type="button" class="icon-btn" onclick={close} aria-label="Close">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M6 6l12 12M18 6L6 18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
          </svg>
        </button>
        <span class="modal-title">Post</span>
        <span class="header-spacer"></span>
      </header>

      <div class="modal-body">
        <div class="post-header">
          <Avatar seed={post.author_id} alt={post.author_name} size={36} />
          <div class="post-meta">
            <span class="author">{post.author_name}</span>
            <span class="time">{timeAgo(post.created_at)}</span>
          </div>
        </div>

        {#if post.media_ref}
          <img class="post-media" src={blobUrl(post.media_ref)} alt="" />
        {/if}

        {#if post.body}
          <FormattedText text={post.body} class="post-caption" />
        {/if}

        <section class="comments-section">
          <h3 class="comments-title">
            Comments
            {#if comments.length > 0}
              <span class="count">({comments.length})</span>
            {/if}
          </h3>

          {#if loading && comments.length === 0}
            <p class="muted">Loading comments…</p>
          {:else if comments.length === 0}
            <p class="muted">No comments yet. Be the first.</p>
          {:else}
            <ul class="comments-list">
              {#each comments as comment (comment.id)}
                <li class="comment">
                  <Avatar seed={comment.author_id} alt={comment.author_name} size={28} />
                  <div class="comment-body">
                    <span class="comment-author">{comment.author_name}</span>
                    <FormattedText text={comment.body} class="comment-text" />
                    <span class="comment-time">{timeAgo(comment.created_at)}</span>
                  </div>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      </div>

      <footer class="comment-composer">
        <input
          bind:value={commentBody}
          placeholder="Add a comment…"
          disabled={posting || disabled}
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
          onclick={submitComment}
          disabled={posting || disabled || !commentBody.trim()}
        >
          Post
        </button>
      </footer>

      {#if error}
        <p class="error">{error}</p>
      {/if}
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    background: rgba(0, 0, 0, 0.5);
  }

  .modal {
    width: 100%;
    max-width: 480px;
    max-height: 92vh;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 16px 16px 0 0;
    overflow: hidden;
  }

  @media (min-width: 520px) {
    .backdrop {
      align-items: center;
      padding: 1rem;
    }

    .modal {
      border-radius: 14px;
      max-height: 88vh;
    }
  }

  .modal-header {
    display: grid;
    grid-template-columns: 2.5rem 1fr 2.5rem;
    align-items: center;
    padding: 0.65rem 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .modal-title {
    text-align: center;
    font-weight: 700;
    font-size: 0.95rem;
  }

  .header-spacer {
    width: 2.5rem;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2.5rem;
    height: 2.5rem;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .icon-btn svg {
    width: 1.1rem;
    height: 1.1rem;
  }

  .icon-btn:hover {
    color: var(--text);
    background: color-mix(in srgb, var(--border) 30%, transparent);
  }

  .modal-body {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
  }

  .post-header {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    margin-bottom: 0.85rem;
  }

  .post-meta {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }

  .author {
    font-weight: 600;
    font-size: 0.9rem;
  }

  .time {
    font-size: 0.75rem;
    color: var(--muted);
  }

  .post-media {
    width: 100%;
    max-height: 360px;
    object-fit: cover;
    border-radius: 8px;
    margin-bottom: 0.75rem;
    display: block;
    background: var(--bg);
  }

  :global(.post-caption) {
    font-size: 0.95rem;
    line-height: 1.45;
    margin-bottom: 1rem;
  }

  .comments-section {
    border-top: 1px solid var(--border);
    padding-top: 0.85rem;
  }

  .comments-title {
    margin: 0 0 0.75rem;
    font-size: 0.8rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }

  .count {
    font-weight: 600;
  }

  .muted {
    margin: 0;
    font-size: 0.85rem;
    color: var(--muted);
  }

  .comments-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }

  .comment {
    display: flex;
    gap: 0.55rem;
    align-items: flex-start;
  }

  .comment-body {
    min-width: 0;
    flex: 1;
  }

  .comment-author {
    font-weight: 600;
    font-size: 0.85rem;
    margin-right: 0.35rem;
  }

  :global(.comment-text) {
    display: inline;
    font-size: 0.9rem;
    line-height: 1.4;
  }

  .comment-time {
    display: block;
    margin-top: 0.2rem;
    font-size: 0.72rem;
    color: var(--muted);
  }

  .comment-composer {
    display: flex;
    gap: 0.5rem;
    padding: 0.75rem;
    border-top: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg) 50%, var(--surface));
  }

  .comment-composer input {
    flex: 1;
    min-width: 0;
    padding: 0.55rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 0.875rem;
  }

  .send-comment {
    padding: 0.55rem 0.9rem;
    border: none;
    border-radius: 999px;
    background: var(--accent);
    color: #fff;
    font: inherit;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
  }

  .send-comment:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .error {
    margin: 0;
    padding: 0 0.75rem 0.75rem;
    font-size: 0.85rem;
  }
</style>
