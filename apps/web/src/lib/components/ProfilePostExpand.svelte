<script lang="ts">
	import { api, blobUrl, type FeedItem, type PostComment, type ProfileComment } from '$lib/api';
	import Avatar from '$lib/components/Avatar.svelte';
	import FormattedText from '$lib/components/FormattedText.svelte';
	import VideoMedia from '$lib/components/VideoMedia.svelte';

	interface Props {
		post?: FeedItem | null;
		loading?: boolean;
		disabled?: boolean;
		draft?: boolean;
		compact?: boolean;
		previewImageUrl?: string | null;
		caption?: string;
		oncaptionchange?: (value: string) => void;
		authorId?: string;
		authorName?: string;
		showClose?: boolean;
		/** Durable profile item id for profile comments (preferred over feed post comments). */
		profileItemId?: string | null;
		/** Friend contact id when viewing someone else's profile. */
		ownerContactId?: string | null;
		onclose?: () => void;
		oncomment?: () => void;
	}

	let {
		post = null,
		loading = false,
		disabled = false,
		draft = false,
		compact = false,
		previewImageUrl = null,
		caption = '',
		oncaptionchange,
		authorId = '',
		authorName = '',
		showClose = true,
		profileItemId = null,
		ownerContactId = null,
		onclose,
		oncomment
	}: Props = $props();

	let comments = $state<Array<PostComment | ProfileComment>>([]);
	let commentsLoading = $state(false);
	let commentBody = $state('');
	let posting = $state(false);
	let error = $state('');

	const displayAuthorId = $derived(draft ? authorId : (post?.author_id ?? ''));
	const displayAuthorName = $derived(draft ? authorName : (post?.author_name ?? ''));
	const isVideo = $derived(!draft && post?.media_kind === 'video' && !!post?.media_ref);
	const mediaUrl = $derived(
		draft
			? previewImageUrl
			: post?.media_kind === 'video'
				? null
				: post?.media_ref
					? blobUrl(post.media_ref)
					: null
	);
	const useProfileComments = $derived(!!profileItemId);

	$effect(() => {
		if (draft || !post) {
			comments = [];
			commentBody = '';
			error = '';
			return;
		}
		if (profileItemId) {
			void loadProfileComments(profileItemId);
		} else {
			void loadFeedComments(post.content_id);
		}
	});

	async function loadFeedComments(postId: string) {
		commentsLoading = true;
		error = '';
		try {
			comments = await api.listPostComments(postId);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load comments';
		} finally {
			commentsLoading = false;
		}
	}

	async function loadProfileComments(itemId: string) {
		commentsLoading = true;
		error = '';
		try {
			comments = ownerContactId
				? await api.listFriendProfileComments(ownerContactId, itemId)
				: await api.listOwnProfileComments(itemId);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load comments';
		} finally {
			commentsLoading = false;
		}
	}

	async function submitComment() {
		if (draft || !post || !commentBody.trim() || posting || disabled) return;
		posting = true;
		error = '';
		try {
			const body = commentBody.trim();
			if (useProfileComments && profileItemId) {
				const comment = ownerContactId
					? await api.addFriendProfileComment(ownerContactId, profileItemId, body)
					: await api.addOwnProfileComment(profileItemId, body);
				comments = [...comments, comment];
			} else {
				const comment = await api.addPostComment(post.content_id, body);
				comments = [...comments, comment];
			}
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

<article class="expand-panel" class:compact class:draft>
	<header class="expand-header">
		<div class="expand-meta">
			{#if draft || post}
				<Avatar seed={displayAuthorId} alt={displayAuthorName} size={compact ? 28 : 32} />
				<div class="meta-text">
					<span class="author">{displayAuthorName}</span>
					<span class="time">{draft ? 'Preview' : post ? timeAgo(post.created_at) : ''}</span>
				</div>
			{:else}
				<span class="loading-label">{loading ? 'Loading post…' : 'Post unavailable'}</span>
			{/if}
		</div>
		{#if showClose}
			<button type="button" class="close-btn" aria-label="Close" onclick={() => onclose?.()}>
				<svg viewBox="0 0 24 24" aria-hidden="true">
					<path d="M6 6l12 12M18 6L6 18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
				</svg>
			</button>
		{/if}
	</header>

	<div class="expand-body">
		{#if loading}
			<p class="muted">Loading…</p>
		{:else if draft || post}
			{#if draft}
				<div class="draft-compose">
					{#if mediaUrl}
						<img class="expand-media" src={mediaUrl} alt="" />
					{/if}
					<textarea
						class="caption-draft"
						value={caption}
						rows={compact ? 2 : 3}
						placeholder="Write a caption…"
						aria-label="Caption"
						disabled={disabled}
						oninput={(e) => oncaptionchange?.((e.currentTarget as HTMLTextAreaElement).value)}
					></textarea>
				</div>
			{:else if post}
				<div class="post-content">
					{#if isVideo}
						<VideoMedia
							rootHash={post.media_ref!}
							thumbRef={post.thumb_ref ?? post.media_ref!}
							mediaReady={post.media_ready ?? false}
							{compact}
						/>
					{:else if mediaUrl}
						<img class="post-content-media" src={mediaUrl} alt="" />
					{/if}
					{#if post.body}
						<div class="post-content-caption" class:standalone={!mediaUrl && !isVideo}>
							<p class="caption-line">
								<span class="caption-author">{displayAuthorName}</span>
								<FormattedText text={post.body} class="expand-caption" />
							</p>
						</div>
					{/if}
				</div>
			{/if}

			{#if !draft}
				<section class="comments-section">
					<h3 class="comments-title">Comments</h3>

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
										<span class="comment-author">{comment.author_name}</span>
										<FormattedText text={comment.body} class="comment-text" />
										<span class="comment-time">{timeAgo(comment.created_at)}</span>
									</div>
								</li>
							{/each}
						</ul>
					{/if}
				</section>
			{/if}
		{/if}

		{#if error}
			<p class="error">{error}</p>
		{/if}
	</div>

	{#if post}
		<footer class="comment-composer">
			<input
				bind:value={commentBody}
				placeholder="Add a comment…"
				disabled={posting || disabled || loading}
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
				disabled={posting || disabled || loading || !commentBody.trim()}
			>
				Post
			</button>
		</footer>
	{/if}
</article>

<style>
	.expand-panel {
		display: flex;
		flex-direction: column;
		min-height: 100%;
		border: 1px solid var(--border);
		border-radius: var(--radius-md, 8px);
		background: var(--surface);
		overflow: hidden;
	}

	.expand-panel.compact.draft {
		flex: 1;
		min-height: 0;
	}

	.expand-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		padding: 0.55rem 0.65rem;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
	}

	.expand-panel.compact .expand-header {
		padding: 0.4rem 0.45rem;
	}

	.expand-meta {
		display: flex;
		align-items: center;
		gap: 0.55rem;
		min-width: 0;
	}

	.meta-text {
		display: flex;
		flex-direction: column;
		gap: 0.05rem;
		min-width: 0;
	}

	.author {
		font-size: 0.85rem;
		font-weight: 600;
	}

	.time,
	.loading-label {
		font-size: 0.72rem;
		color: var(--muted);
	}

	.close-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		border: none;
		border-radius: 8px;
		background: transparent;
		color: var(--muted);
		cursor: pointer;
		flex-shrink: 0;
	}

	.close-btn svg {
		width: 1rem;
		height: 1rem;
	}

	.close-btn:hover {
		color: var(--text);
		background: color-mix(in srgb, var(--border) 35%, transparent);
	}

	.expand-body {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: 0.65rem;
	}

	.expand-panel.compact .expand-body {
		padding: 0.35rem;
		min-height: 0;
	}

	.expand-panel.compact.draft .expand-body {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		overflow: hidden;
	}

	.expand-panel.compact.draft .draft-compose {
		display: flex;
		flex: 1;
		flex-direction: column;
		min-height: 0;
		gap: 0.35rem;
	}

	.expand-media {
		width: 100%;
		max-height: min(42vh, 320px);
		object-fit: cover;
		border-radius: var(--radius-sm, 6px);
		display: block;
		margin-bottom: 0.65rem;
		background: var(--bg);
	}

	.expand-panel.compact .expand-media {
		flex: 1 1 auto;
		min-height: 0;
		max-height: none;
		width: 100%;
		margin-bottom: 0;
		border-radius: 4px;
	}

	.expand-panel.compact .caption-draft {
		flex-shrink: 0;
		resize: none;
	}

	.caption-draft {
		width: 100%;
		padding: 0.55rem 0.65rem;
		border: 1px solid var(--border);
		border-radius: 8px;
		background: var(--bg);
		color: var(--text);
		resize: vertical;
		font: inherit;
		font-size: 0.85rem;
		line-height: 1.45;
	}

	.post-content {
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-sm, 6px);
		background: color-mix(in srgb, var(--bg) 35%, var(--surface));
		margin-bottom: 0.75rem;
	}

	.post-content-media {
		width: 100%;
		max-height: min(42vh, 320px);
		object-fit: cover;
		display: block;
		margin: 0;
		background: var(--bg);
	}

	.expand-panel.compact .post-content-media {
		max-height: 120px;
	}

	.post-content-caption {
		padding: 0.8rem 0.85rem 0.9rem;
		border-top: 1px solid var(--border);
	}

	.post-content-caption.standalone {
		border-top: none;
	}

	.caption-line {
		margin: 0;
		font-size: 0.95rem;
		line-height: 1.5;
	}

	.expand-panel.compact .caption-line {
		font-size: 0.88rem;
	}

	.caption-author {
		font-weight: 700;
		color: var(--text);
		margin-right: 0.35rem;
	}

	:global(.expand-caption) {
		display: inline;
		font-size: inherit;
		line-height: inherit;
		margin: 0;
	}

	.comments-section {
		border-top: 1px solid var(--border);
		padding-top: 0.65rem;
	}

	.comments-title {
		margin: 0 0 0.55rem;
		font-size: 0.72rem;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--muted);
	}

	.muted {
		margin: 0;
		font-size: 0.82rem;
		color: var(--muted);
	}

	.comments-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.65rem;
	}

	.comment {
		display: flex;
		gap: 0.45rem;
		align-items: flex-start;
	}

	.comment-body {
		min-width: 0;
		flex: 1;
	}

	.comment-author {
		font-weight: 600;
		font-size: 0.8rem;
		margin-right: 0.25rem;
	}

	:global(.comment-text) {
		display: inline;
		font-size: 0.85rem;
		line-height: 1.4;
	}

	.comment-time {
		display: block;
		margin-top: 0.15rem;
		font-size: 0.68rem;
		color: var(--muted);
	}

	.comment-composer {
		display: flex;
		gap: 0.45rem;
		padding: 0.55rem 0.65rem;
		border-top: 1px solid var(--border);
		background: color-mix(in srgb, var(--bg) 50%, var(--surface));
		flex-shrink: 0;
	}

	.comment-composer input {
		flex: 1;
		min-width: 0;
		padding: 0.5rem 0.7rem;
		border: 1px solid var(--border);
		border-radius: 999px;
		background: var(--bg);
		color: var(--text);
		font: inherit;
		font-size: 0.82rem;
	}

	.send-comment {
		padding: 0.5rem 0.8rem;
		border: none;
		border-radius: 999px;
		background: var(--accent);
		color: #fff;
		font: inherit;
		font-size: 0.82rem;
		font-weight: 600;
		cursor: pointer;
	}

	.send-comment:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.error {
		margin: 0.5rem 0 0;
		font-size: 0.8rem;
	}
</style>
