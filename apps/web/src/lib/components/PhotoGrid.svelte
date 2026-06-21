<script lang="ts">
	import { api, blobUrl, type FeedItem, type ProfilePhoto } from '$lib/api';
	import ProfilePostExpand from '$lib/components/ProfilePostExpand.svelte';
	import { prepareImageForUpload } from '$lib/image';
	import {
		computeProfileGridLayout,
		computeProfileGridLayoutAtIndex,
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
		selectedContentId?: string | null;
		selectedPost?: FeedItem | null;
		selectedPostLoading?: boolean;
		onuploaded?: () => void;
		onselect?: (contentId: string | null) => void;
		oncomment?: () => void;
	}

	let {
		photos,
		disabled = false,
		readonly = false,
		emptyLabel = 'No posts yet — add your first photo.',
		photoUrl,
		authorId = '',
		authorName = '',
		selectedContentId = null,
		selectedPost = null,
		selectedPostLoading = false,
		onuploaded,
		onselect,
		oncomment
	}: Props = $props();

	const urlFor = (hash: string) => (photoUrl ? photoUrl(hash) : blobUrl(hash));

	const gridCells = $derived(
		sortProfileGridCells(computeProfileGridLayout(photos, selectedContentId))
	);

	const previewGridCells = $derived(
		sortProfileGridCells(computeProfileGridLayoutAtIndex(photos, photos.length))
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
		if (!photo.content_id || disabled) return;
		if (selectedContentId === photo.content_id) {
			onselect?.(null);
			return;
		}
		onselect?.(photo.content_id);
	}

	let expandEl = $state<HTMLElement | null>(null);

	$effect(() => {
		if (!selectedContentId || !expandEl) return;
		queueMicrotask(() => {
			expandEl?.scrollIntoView({ behavior: 'smooth', block: 'start' });
		});
	});

	const displayCells = $derived.by(() => {
		if (selectedContentId == null) return gridCells;
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
	{#if photos.length === 0 && !selectedContentId}
		<p class="empty-grid muted">{emptyLabel}</p>
	{/if}

	<div class="photo-grid" class:has-selection={selectedContentId != null}>
		{#each displayCells as cell (cellKey(cell))}
			{#if cell.kind === 'expand'}
				<div class="expand-cell" bind:this={expandEl} style={gridCellStyle(cell)}>
					<ProfilePostExpand
						post={selectedPost}
						loading={selectedPostLoading}
						{disabled}
						onclose={() => onselect?.(null)}
						oncomment={oncomment}
					/>
				</div>
			{:else}
				<button
					type="button"
					class="photo-cell photo-btn"
					style={gridCellStyle(cell)}
					onclick={() => togglePhoto(cell.photo)}
					disabled={!cell.photo.content_id || disabled}
					aria-label={cell.photo.caption ?? 'Open post'}
					aria-pressed={selectedContentId === cell.photo.content_id}
				>
					<img src={urlFor(cell.photo.blob_hash)} alt={cell.photo.caption ?? 'Profile photo'} loading="lazy" />
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

	{#if error}
		<p class="error">{error}</p>
	{/if}
</div>

{#if captionOpen && pendingPreviewUrl && !readonly}
	<div class="caption-backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && cancelCaption()}>
		<div class="caption-modal" role="dialog" aria-modal="true" aria-labelledby="caption-title">
			<h2 id="caption-title">New post</h2>
			<p class="preview-label">Preview on your profile grid</p>

			<div class="preview-frame" aria-label="Expanded grid preview">
				<div class="preview-grid has-selection">
					{#each previewGridCells as cell (`preview-${cellKey(cell)}`)}
						{#if cell.kind === 'expand'}
							<div class="expand-cell preview-expand" style={gridCellStyle(cell)}>
								<ProfilePostExpand
									draft
									compact
									showClose={false}
									previewImageUrl={pendingPreviewUrl}
									caption={captionDraft}
									oncaptionchange={(value) => (captionDraft = value)}
									authorId={authorId}
									authorName={authorName}
									disabled={uploading}
								/>
							</div>
						{:else}
							<div class="preview-thumb" style={gridCellStyle(cell)}>
								<img src={urlFor(cell.photo.blob_hash)} alt="" loading="lazy" />
							</div>
						{/if}
					{/each}
				</div>
			</div>

			<div class="caption-actions">
				<button type="button" class="btn-secondary" onclick={cancelCaption} disabled={uploading}>
					Cancel
				</button>
				<button type="button" class="btn-primary" onclick={publishPhoto} disabled={uploading}>
					{uploading ? 'Posting…' : 'Share'}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.empty-grid {
		margin: 0 0 0.65rem;
		font-size: 0.85rem;
	}

	.photo-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 3px;
		margin-bottom: 0.5rem;
		grid-auto-flow: row dense;
	}

	.photo-grid.has-selection {
		grid-auto-rows: minmax(96px, auto);
	}

	.photo-cell {
		aspect-ratio: 1;
		overflow: hidden;
		background: var(--bg);
	}

	.expand-cell {
		min-height: 280px;
		animation: expand-in 0.18s ease-out;
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

	.caption-backdrop {
		position: fixed;
		inset: 0;
		z-index: 50;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
		background: rgba(0, 0, 0, 0.45);
	}

	.caption-modal {
		width: 100%;
		max-width: 520px;
		padding: 1.25rem;
		border-radius: 12px;
		border: 1px solid var(--border);
		background: var(--surface);
	}

	.caption-modal h2 {
		margin: 0 0 0.35rem;
		font-size: 1.05rem;
	}

	.preview-label {
		margin: 0 0 0.65rem;
		font-size: 0.82rem;
		color: var(--muted);
	}

	.preview-frame {
		max-height: 360px;
		overflow: auto;
		margin-bottom: 0.85rem;
		padding: 3px;
		border: 1px solid var(--border);
		border-radius: 8px;
		background: var(--bg);
		scrollbar-width: thin;
	}

	.preview-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 3px;
		grid-auto-flow: row dense;
		grid-auto-rows: minmax(104px, auto);
		min-height: 220px;
	}

	.preview-thumb {
		aspect-ratio: 1;
		overflow: hidden;
		background: var(--bg);
		opacity: 0.72;
	}

	.preview-thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}

	.preview-expand {
		display: flex;
		min-height: 212px;
		height: 100%;
		animation: none;
	}

	.preview-expand :global(.expand-panel) {
		flex: 1;
		width: 100%;
		min-height: 0;
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

		.preview-grid.has-selection {
			grid-template-columns: 1fr;
		}

		.preview-grid.has-selection .preview-thumb,
		.preview-grid.has-selection .preview-expand {
			grid-column: 1 / -1 !important;
			grid-row: auto !important;
		}
	}
</style>
