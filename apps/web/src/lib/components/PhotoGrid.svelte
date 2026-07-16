<script lang="ts">
	import { api, blobUrl, type FeedItem, type ProfilePhoto } from '$lib/api';
	import Avatar from '$lib/components/Avatar.svelte';
	import ProfilePostExpand from '$lib/components/ProfilePostExpand.svelte';
	import { prepareImageForUpload } from '$lib/image';
	import {
		computeProfileGridLayout,
		gridCellStyle,
		sortProfileGridCells
	} from '$lib/profile-grid-layout';

	interface Props {
		photos: ProfilePhoto[];
		disabled?: boolean;
		readonly?: boolean;
		emptyLabel?: string;
		photoUrl?: (hash: string) => string;
		authorId?: string;
		authorName?: string;
		/** Selected durable profile item id (not feed content_id). */
		selectedItemId?: string | null;
		selectedPost?: FeedItem | null;
		selectedPostLoading?: boolean;
		/** When set, comments use profile-item APIs against this contact (friend profile). */
		ownerContactId?: string | null;
		/** Owner Posts tab: select cells for delete instead of expanding. */
		deleteMode?: boolean;
		selectedDeleteIds?: Set<string>;
		onuploaded?: () => void;
		onselect?: (itemId: string | null) => void;
		oncomment?: () => void;
		ontoggledelete?: (itemId: string) => void;
	}

	let {
		photos,
		disabled = false,
		readonly = false,
		emptyLabel = 'No photos yet. Add your first photo.',
		photoUrl,
		authorId = '',
		authorName = '',
		selectedItemId = null,
		selectedPost = null,
		selectedPostLoading = false,
		ownerContactId = null,
		deleteMode = false,
		selectedDeleteIds = new Set<string>(),
		onuploaded,
		onselect,
		oncomment,
		ontoggledelete
	}: Props = $props();

	const urlFor = (hash: string) => (photoUrl ? photoUrl(hash) : blobUrl(hash));

	let blobReady = $state<Record<string, boolean>>({});
	const pendingBlobFetches = new Map<string, Promise<boolean>>();

	async function ensureFriendBlob(hash: string): Promise<boolean> {
		if (!ownerContactId || disabled) return true;
		if (blobReady[hash]) return true;
		const pending = pendingBlobFetches.get(hash);
		if (pending) return pending;

		const task = (async () => {
			try {
				await api.fetchFriendBlob(ownerContactId!, hash);
				blobReady = { ...blobReady, [hash]: true };
				return true;
			} catch {
				return false;
			} finally {
				pendingBlobFetches.delete(hash);
			}
		})();
		pendingBlobFetches.set(hash, task);
		return task;
	}

	function friendPhotoReady(hash: string): boolean {
		return !ownerContactId || !!blobReady[hash];
	}

	$effect(() => {
		if (!ownerContactId || disabled) return;
		for (const photo of photos) {
			void ensureFriendBlob(photo.blob_hash);
		}
	});

	const gridCells = $derived(
		sortProfileGridCells(computeProfileGridLayout(photos, selectedItemId))
	);

	let uploading = $state(false);
	let error = $state('');
	let fileInput = $state<HTMLInputElement | null>(null);
	let pendingBase64 = $state<string | null>(null);
	let captionDraft = $state('');
	let captionOpen = $state(false);

	const pendingPreviewUrl = $derived(
		pendingBase64 ? `data:image/jpeg;base64,${pendingBase64}` : null
	);

	$effect(() => {
		if (!captionOpen) return;
		const prev = document.body.style.overflow;
		document.body.style.overflow = 'hidden';
		return () => {
			document.body.style.overflow = prev;
		};
	});

	export function openPhotoPicker() {
		if (uploading || disabled || readonly) return;
		fileInput?.click();
	}

	async function onFileSelect(e: Event) {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file) return;

		error = '';
		try {
			pendingBase64 = await prepareImageForUpload(file);
			captionDraft = '';
			captionOpen = true;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to process image';
		}
	}

	function cancelCaption() {
		captionOpen = false;
		pendingBase64 = null;
		captionDraft = '';
		error = '';
	}

	async function publishPhoto() {
		if (!pendingBase64 || uploading) return;
		uploading = true;
		error = '';
		try {
			await api.uploadProfilePhoto(pendingBase64, captionDraft.trim() || undefined);
			captionOpen = false;
			pendingBase64 = null;
			captionDraft = '';
			onuploaded?.();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to upload photo';
		} finally {
			uploading = false;
		}
	}

	function togglePhoto(photo: ProfilePhoto) {
		if (disabled) return;
		if (deleteMode) {
			ontoggledelete?.(photo.id);
			return;
		}
		if (selectedItemId === photo.id) {
			onselect?.(null);
			return;
		}
		onselect?.(photo.id);
	}

	let expandEl = $state<HTMLElement | null>(null);

	$effect(() => {
		if (!selectedItemId || !expandEl) return;
		queueMicrotask(() => {
			expandEl?.scrollIntoView({ behavior: 'smooth', block: 'start' });
		});
	});

	const displayCells = $derived.by(() => {
		if (selectedItemId == null) return gridCells;
		const expand = gridCells.find((cell) => cell.kind === 'expand');
		if (!expand) return gridCells;
		return [expand, ...gridCells.filter((cell) => cell.kind !== 'expand')];
	});

	function cellKey(cell: (typeof gridCells)[number]): string {
		if (cell.kind === 'expand') {
			return `expand-${cell.photo?.id ?? `idx-${cell.photoIndex}`}`;
		}
		return `thumb-${cell.photo.id}`;
	}
</script>

<div class="photo-section">
	{#if photos.length === 0 && !selectedItemId}
		<p class="empty-grid muted">{emptyLabel}</p>
	{/if}

	<div
		class="photo-grid"
		class:has-selection={selectedItemId != null && !deleteMode}
		class:delete-mode={deleteMode}
	>
		{#each displayCells as cell (cellKey(cell))}
			{#if cell.kind === 'expand' && !deleteMode}
				<div class="expand-cell" bind:this={expandEl} style={gridCellStyle(cell)}>
					<ProfilePostExpand
						post={selectedPost}
						profileItemId={selectedItemId}
						{ownerContactId}
						loading={selectedPostLoading}
						{disabled}
						onclose={() => onselect?.(null)}
						oncomment={oncomment}
					/>
				</div>
			{:else if cell.kind !== 'expand'}
				{@const marked = selectedDeleteIds.has(cell.photo.id)}
				<button
					type="button"
					class="photo-cell photo-btn"
					class:marked={deleteMode && marked}
					style={gridCellStyle(cell)}
					onclick={() => togglePhoto(cell.photo)}
					disabled={disabled}
					aria-label={deleteMode
						? marked
							? `Deselect ${cell.photo.caption ?? 'photo'}`
							: `Select ${cell.photo.caption ?? 'photo'} for delete`
						: (cell.photo.caption ?? 'Open photo')}
					aria-pressed={deleteMode ? marked : selectedItemId === cell.photo.id}
				>
					{#if friendPhotoReady(cell.photo.blob_hash)}
						{#key cell.photo.blob_hash}
							<img
								src={urlFor(cell.photo.blob_hash)}
								alt={cell.photo.caption ?? 'Profile photo'}
								loading={ownerContactId ? 'eager' : 'lazy'}
								decoding="async"
							/>
						{/key}
					{:else}
						<span class="photo-placeholder" aria-hidden="true"></span>
					{/if}
					{#if deleteMode && marked}
						<span class="mark" aria-hidden="true">✓</span>
					{/if}
				</button>
			{/if}
		{/each}
	</div>

	<input
		bind:this={fileInput}
		type="file"
		accept="image/jpeg,image/png,image/gif,image/webp,image/*"
		class="file-input"
		disabled={uploading || disabled}
		onchange={onFileSelect}
	/>

	{#if error && !captionOpen}
		<p class="error">{error}</p>
	{/if}
</div>

{#if captionOpen && pendingPreviewUrl && !readonly}
	<div
		class="new-post-overlay"
		role="dialog"
		aria-modal="true"
		aria-labelledby="caption-title"
		onclick={(e) => e.target === e.currentTarget && cancelCaption()}
	>
		<div class="new-post-panel">
			<header class="new-post-header">
				<h2 id="caption-title">New post</h2>
			</header>

			<div class="new-post-photo">
				<img src={pendingPreviewUrl} alt="New post preview" />
			</div>

			<footer class="new-post-footer">
				<div class="new-post-compose">
					<Avatar seed={authorId} alt={authorName} size={28} />
					<div class="new-post-fields">
						<span class="new-post-name">{authorName}</span>
						<textarea
							class="new-post-caption"
							bind:value={captionDraft}
							placeholder="Write a caption…"
							rows="2"
							disabled={uploading}
						></textarea>
					</div>
				</div>
				{#if error}
					<p class="error">{error}</p>
				{/if}
				<div class="caption-actions">
					<button type="button" class="btn-secondary" onclick={cancelCaption} disabled={uploading}>
						Cancel
					</button>
					<button type="button" class="btn-primary" onclick={publishPhoto} disabled={uploading}>
						{uploading ? 'Posting…' : 'Share'}
					</button>
				</div>
			</footer>
		</div>
	</div>
{/if}

<style>
	.photo-section {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		width: 100%;
	}

	.empty-grid {
		margin: 0 0 0.65rem;
		font-size: 0.85rem;
	}

	.photo-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 6px;
		margin-bottom: 0;
		grid-auto-flow: row dense;
	}

	.photo-grid.has-selection {
		grid-auto-rows: minmax(96px, auto);
	}

	.photo-cell {
		position: relative;
		aspect-ratio: 1;
		overflow: hidden;
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm, 6px);
	}

	.photo-grid.delete-mode .photo-cell.marked {
		outline: 2px solid var(--danger);
		outline-offset: -2px;
	}

	.photo-cell .mark {
		position: absolute;
		top: 0.35rem;
		right: 0.35rem;
		width: 1.25rem;
		height: 1.25rem;
		border-radius: 999px;
		background: var(--danger);
		color: #fff;
		font-size: 0.7rem;
		font-weight: 700;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		line-height: 1;
	}

	.expand-cell {
		min-height: 280px;
		animation: expand-in 0.18s ease-out;
		border-radius: var(--radius-md, 8px);
		overflow: hidden;
	}

	@keyframes expand-in {
		from {
			opacity: 0.6;
		}
		to {
			opacity: 1;
		}
	}

	.photo-cell img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
		transition: opacity 0.15s ease;
	}

	.photo-placeholder {
		display: block;
		width: 100%;
		height: 100%;
		background: color-mix(in srgb, var(--border) 35%, var(--bg));
		animation: photo-pulse 1.1s ease-in-out infinite;
	}

	@keyframes photo-pulse {
		0%,
		100% {
			opacity: 0.55;
		}
		50% {
			opacity: 0.9;
		}
	}

	.photo-btn {
		padding: 0;
		border: none;
		cursor: pointer;
	}

	.photo-btn:disabled {
		cursor: default;
	}

	.photo-btn:not(:disabled):hover img {
		opacity: 0.88;
	}

	.file-input {
		display: none;
	}

	.error {
		margin: 0;
		font-size: 0.8rem;
	}

	.new-post-overlay {
		position: fixed;
		inset: 0;
		z-index: 50;
		display: flex;
		flex-direction: column;
		background: var(--bg);
	}

	.new-post-panel {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		width: 100%;
		background: var(--bg);
	}

	.new-post-header {
		flex-shrink: 0;
		padding: calc(0.65rem + var(--safe-top)) 1rem 0.65rem;
		border-bottom: 1px solid var(--border);
		background: var(--surface);
		text-align: center;
	}

	.new-post-header h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 700;
	}

	.new-post-photo {
		flex: 1;
		min-height: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #000;
	}

	.new-post-photo img {
		width: 100%;
		height: 100%;
		object-fit: contain;
		display: block;
	}

	.new-post-footer {
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		gap: 0.55rem;
		padding: 0.85rem 1rem calc(0.85rem + var(--safe-bottom));
		border-top: 1px solid var(--border);
		background: var(--surface);
	}

	.new-post-compose {
		display: flex;
		align-items: flex-start;
		gap: 0.55rem;
	}

	.new-post-fields {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}

	.new-post-name {
		font-size: 0.88rem;
		font-weight: 600;
		line-height: 1.35;
	}

	.new-post-caption {
		width: 100%;
		min-height: 2.5rem;
		max-height: 6rem;
		padding: 0;
		border: none;
		background: transparent;
		color: var(--text);
		font: inherit;
		font-size: 0.88rem;
		line-height: 1.4;
		resize: none;
	}

	.new-post-caption:focus {
		outline: none;
	}

	.new-post-caption::placeholder {
		color: var(--muted);
	}

	.new-post-caption:disabled {
		opacity: 0.6;
	}

	@media (min-width: 641px) {
		.new-post-overlay {
			align-items: center;
			justify-content: center;
			padding: 1.25rem;
			background: rgba(0, 0, 0, 0.5);
		}

		.new-post-panel {
			flex: none;
			width: clamp(520px, 72vmin, 820px);
			max-width: min(820px, 94vw);
			max-height: 94vh;
			border: 1px solid var(--border);
			border-radius: var(--radius-lg, 12px);
			background: var(--surface);
			overflow: hidden;
		}

		.new-post-header {
			padding: 0.85rem 1.15rem;
			text-align: left;
		}

		.new-post-header h2 {
			font-size: 1.05rem;
		}

		.new-post-photo {
			flex: none;
			width: 100%;
			height: min(72vmin, calc(94vh - 11rem), 780px);
			min-height: 320px;
		}

		.new-post-footer {
			padding: 0.95rem 1.15rem 1.15rem;
		}

		.new-post-name {
			font-size: 0.95rem;
		}

		.new-post-caption {
			flex: 1;
			min-height: 3rem;
			font-size: 0.95rem;
		}
	}

	.caption-actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}

	.btn-secondary,
	.btn-primary {
		padding: 0.5rem 1rem;
		border-radius: 8px;
		font: inherit;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
	}

	.btn-secondary {
		border: 1px solid var(--border);
		background: transparent;
		color: var(--text);
	}

	.btn-primary {
		border: none;
		background: var(--accent);
		color: #fff;
	}

	.btn-primary:disabled,
	.btn-secondary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	@media (max-width: 640px) {
		.photo-grid.has-selection {
			grid-template-columns: 1fr;
		}

		.photo-grid.has-selection .photo-cell,
		.photo-grid.has-selection .expand-cell {
			grid-column: 1 / -1 !important;
			grid-row: auto !important;
		}

		.expand-cell {
			min-height: 360px;
		}
	}
</style>
