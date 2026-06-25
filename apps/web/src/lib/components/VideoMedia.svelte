<script lang="ts">
	import { api, blobUrl } from '$lib/api';

	interface Props {
		rootHash: string;
		thumbRef: string;
		mediaReady?: boolean;
		compact?: boolean;
	}

	let { rootHash, thumbRef, mediaReady = false, compact = false }: Props = $props();

	let fetching = $state(false);
	let progress = $state<{ done: number; total: number; transport: string } | null>(null);
	let error = $state('');
	let ready = $state(false);
	let pollTimer = $state<ReturnType<typeof setInterval> | null>(null);

	$effect(() => {
		ready = mediaReady;
	});

	$effect(() => {
		return () => {
			if (pollTimer) clearInterval(pollTimer);
		};
	});

	function stopPolling() {
		if (pollTimer) {
			clearInterval(pollTimer);
			pollTimer = null;
		}
	}

	async function pollStatus() {
		try {
			const status = await api.mediaFetchStatus(rootHash);
			progress = {
				done: status.chunks_done,
				total: status.chunks_total,
				transport: status.transport
			};
			if (status.state === 'complete') {
				ready = true;
				fetching = false;
				stopPolling();
			} else if (status.state === 'failed') {
				error = status.error ?? 'Download failed';
				fetching = false;
				stopPolling();
			}
		} catch (e) {
			error = e instanceof Error ? e.message : 'Status check failed';
			fetching = false;
			stopPolling();
		}
	}

	async function startFetch() {
		if (ready || fetching) return;
		fetching = true;
		error = '';
		try {
			const status = await api.startMediaFetch(rootHash);
			progress = {
				done: status.chunks_done,
				total: status.chunks_total,
				transport: status.transport
			};
			if (status.state === 'complete') {
				ready = true;
				fetching = false;
				return;
			}
			stopPolling();
			pollTimer = setInterval(() => void pollStatus(), 800);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Could not start download';
			fetching = false;
		}
	}

	async function onTap() {
		if (ready) return;
		await startFetch();
	}

	const thumbUrl = $derived(blobUrl(thumbRef));
	const videoUrl = $derived(ready ? blobUrl(rootHash) : null);
	const progressLabel = $derived(
		progress && progress.total > 0
			? `${progress.done}/${progress.total}${progress.transport !== 'unknown' ? ` · ${progress.transport}` : ''}`
			: 'Loading…'
	);
</script>

<div class="video-media" class:compact>
	{#if ready && videoUrl}
		<!-- svelte-ignore a11y_media_has_caption -->
		<video class="video-player" src={videoUrl} controls playsinline preload="metadata"></video>
	{:else}
		<button type="button" class="thumb-btn" onclick={() => void onTap()} disabled={fetching}>
			<img class="thumb" src={thumbUrl} alt="" loading="lazy" />
			<span class="overlay" aria-hidden="true">
				{#if fetching}
					<span class="badge progress">{progressLabel}</span>
				{:else}
					<span class="play-icon">▶</span>
				{/if}
			</span>
		</button>
		{#if error}
			<p class="error">{error}</p>
		{/if}
	{/if}
</div>

<style>
	.video-media {
		width: 100%;
	}

	.thumb-btn {
		display: block;
		position: relative;
		width: 100%;
		padding: 0;
		border: none;
		background: none;
		cursor: pointer;
		border-radius: var(--radius-lg, 8px);
		overflow: hidden;
	}

	.thumb-btn:disabled {
		cursor: wait;
	}

	.thumb {
		width: 100%;
		max-height: 420px;
		object-fit: cover;
		display: block;
		background: var(--bg);
	}

	.compact .thumb {
		max-height: none;
		height: 100%;
	}

	.overlay {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(0, 0, 0, 0.25);
	}

	.play-icon {
		font-size: 2.5rem;
		color: #fff;
		text-shadow: 0 2px 8px rgba(0, 0, 0, 0.5);
	}

	.badge.progress {
		padding: 0.35rem 0.65rem;
		border-radius: 999px;
		background: rgba(0, 0, 0, 0.65);
		color: #fff;
		font-size: 0.75rem;
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}

	.video-player {
		width: 100%;
		max-height: 420px;
		border-radius: var(--radius-lg, 8px);
		display: block;
		background: #000;
	}

	.compact .video-player {
		max-height: none;
		height: 100%;
		object-fit: cover;
	}

	.error {
		margin: 0.35rem 0 0;
		font-size: 0.8rem;
		color: var(--danger, #c0392b);
	}
</style>
