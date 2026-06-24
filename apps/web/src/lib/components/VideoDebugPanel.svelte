<script lang="ts">
	import { isVideoDebugEnabled } from '$lib/video-debug.svelte';
	import {
		clearVideoDebugLog,
		formatVideoDebugDetail,
		videoDebugLog
	} from '$lib/video-debug.svelte';

	let listEl = $state<HTMLElement | null>(null);

	const visible = $derived(isVideoDebugEnabled());
	const lines = $derived(videoDebugLog.lines);

	$effect(() => {
		if (!listEl || lines.length === 0) return;
		listEl.scrollTop = listEl.scrollHeight;
	});
</script>

{#if visible}
	<div class="video-debug" class:collapsed={!videoDebugLog.open}>
		<header class="video-debug-head">
			<button
				type="button"
				class="video-debug-toggle"
				onclick={() => (videoDebugLog.open = !videoDebugLog.open)}
			>
				Video debug ({lines.length})
			</button>
			{#if videoDebugLog.open}
				<button type="button" class="video-debug-clear" onclick={clearVideoDebugLog}>Clear</button>
			{/if}
		</header>
		{#if videoDebugLog.open}
			<div class="video-debug-list" bind:this={listEl}>
				{#if lines.length === 0}
					<p class="video-debug-empty">Pick a video in the composer — logs appear here.</p>
				{:else}
					{#each lines as line (line.id)}
						<div class="video-debug-line">
							<span class="video-debug-ms">+{line.ms}ms</span>
							<span class="video-debug-step">{line.step}</span>
							{#if Object.keys(line.detail).length > 0}
								<code class="video-debug-detail">{formatVideoDebugDetail(line.detail)}</code>
							{/if}
						</div>
					{/each}
				{/if}
			</div>
		{/if}
	</div>
{/if}

<style>
	.video-debug {
		position: fixed;
		right: max(0.75rem, var(--safe-right));
		bottom: max(0.75rem, var(--safe-bottom));
		z-index: 9999;
		width: min(420px, calc(100vw - 1.5rem));
		max-height: min(42vh, 360px);
		display: flex;
		flex-direction: column;
		border: 1px solid color-mix(in srgb, var(--accent) 45%, var(--border));
		border-radius: 8px;
		background: color-mix(in srgb, var(--surface) 92%, #000);
		box-shadow: 0 8px 28px rgba(0, 0, 0, 0.35);
		font-size: 0.7rem;
		line-height: 1.35;
		pointer-events: auto;
	}

	.video-debug.collapsed {
		max-height: none;
	}

	.video-debug-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		padding: 0.35rem 0.5rem;
		border-bottom: 1px solid var(--border);
		background: color-mix(in srgb, var(--accent) 8%, var(--surface));
	}

	.video-debug-toggle,
	.video-debug-clear {
		border: none;
		background: none;
		color: var(--text);
		font: inherit;
		font-size: 0.72rem;
		font-weight: 600;
		cursor: pointer;
		padding: 0.2rem 0.35rem;
		border-radius: 4px;
	}

	.video-debug-clear {
		color: var(--muted);
		font-weight: 500;
	}

	.video-debug-toggle:hover,
	.video-debug-clear:hover {
		background: color-mix(in srgb, var(--border) 40%, transparent);
	}

	.video-debug-list {
		overflow: auto;
		padding: 0.4rem 0.5rem;
		flex: 1;
		min-height: 0;
	}

	.video-debug-empty {
		margin: 0;
		color: var(--muted);
	}

	.video-debug-line {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 0.15rem 0.45rem;
		margin-bottom: 0.45rem;
	}

	.video-debug-ms {
		color: var(--accent);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}

	.video-debug-step {
		font-weight: 600;
		word-break: break-word;
	}

	.video-debug-detail {
		grid-column: 1 / -1;
		display: block;
		padding: 0.25rem 0.35rem;
		border-radius: 4px;
		background: color-mix(in srgb, var(--border) 25%, transparent);
		color: var(--muted);
		font-family: ui-monospace, 'Cascadia Code', monospace;
		font-size: 0.65rem;
		word-break: break-all;
		white-space: pre-wrap;
	}
</style>
