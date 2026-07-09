<script lang="ts">
	import { zipSync } from 'fflate';
	import {
		api,
		uploadArchiveBlob,
		ARCHIVE_ZIP_SOFT_WARN_BYTES,
		type ArchiveEntry,
		type ArchiveFolder,
		type ArchiveFolderSummary
	} from '$lib/api';

	type Folder = ArchiveFolder | ArchiveFolderSummary;

	interface Props {
		mode: 'owner' | 'friend';
		folders: Folder[];
		disabled?: boolean;
		contactId?: string | null;
		onfolderschange?: () => void | Promise<void>;
		onerror?: (message: string) => void;
	}

	let {
		mode,
		folders,
		disabled = false,
		contactId = null,
		onfolderschange,
		onerror
	}: Props = $props();

	let folderId = $state<string | null>(null);
	let entries = $state<ArchiveEntry[]>([]);
	let loadingEntries = $state(false);
	let nameDraft = $state('');
	let creatingFolder = $state(false);
	let busy = $state(false);
	let progress = $state('');
	let downloadStatus = $state<Record<string, string>>({});
	let dragOver = $state(false);
	let fileInput: HTMLInputElement | null = $state(null);
	let folderInput: HTMLInputElement | null = $state(null);

	const currentFolder = $derived(
		folderId ? (folders.find((f) => f.id === folderId) ?? null) : null
	);
	const atRoot = $derived(folderId == null);

	function entryCount(folder: Folder): number | null {
		return 'entry_count' in folder ? folder.entry_count : null;
	}

	function formatBytes(n: number): string {
		if (n < 1024) return `${n} B`;
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
		if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
		return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
	}

	function fileGlyph(name: string, mime: string): string {
		const lower = name.toLowerCase();
		if (mime.includes('zip') || lower.endsWith('.zip')) return 'zip';
		if (mime.startsWith('image/') || /\.(png|jpe?g|gif|webp|heic)$/i.test(lower)) return 'img';
		if (mime.startsWith('video/') || /\.(mp4|mov|webm|mkv)$/i.test(lower)) return 'vid';
		if (mime.startsWith('audio/') || /\.(mp3|wav|flac|m4a)$/i.test(lower)) return 'aud';
		return 'file';
	}

	function reportError(e: unknown, fallback: string) {
		const msg = e instanceof Error ? e.message : fallback;
		onerror?.(msg);
	}

	async function refreshFolders() {
		await onfolderschange?.();
	}

	function goRoot() {
		folderId = null;
		entries = [];
		creatingFolder = false;
		progress = '';
		dragOver = false;
	}

	async function openFolder(id: string) {
		if (disabled) return;
		folderId = id;
		creatingFolder = false;
		loadingEntries = true;
		entries = [];
		try {
			if (mode === 'owner') {
				entries = await api.listArchiveEntries(id);
			} else if (contactId) {
				const res = await api.fetchFriendArchiveFolder(contactId, id);
				entries = res.entries;
			}
		} catch (e) {
			reportError(e, 'Failed to open folder');
		} finally {
			loadingEntries = false;
		}
	}

	async function createFolder() {
		const name = nameDraft.trim();
		if (!name || busy || mode !== 'owner') return;
		busy = true;
		try {
			const created = await api.createArchiveFolder(name);
			nameDraft = '';
			creatingFolder = false;
			await refreshFolders();
			await openFolder(created.id);
		} catch (e) {
			reportError(e, 'Failed to create folder');
		} finally {
			busy = false;
		}
	}

	async function ingestBlob(name: string, blob: Blob, mime?: string) {
		if (!folderId || mode !== 'owner') return;
		if (blob.size >= ARCHIVE_ZIP_SOFT_WARN_BYTES) {
			const ok = confirm(
				`This is about ${formatBytes(blob.size)}. Zipping or uploading large folders in the browser can use a lot of memory. Continue?`
			);
			if (!ok) return;
		}
		busy = true;
		progress = 'Starting upload…';
		try {
			await uploadArchiveBlob(folderId, name, blob, mime, (done, total, phase) => {
				progress =
					phase === 'complete'
						? 'Finalizing…'
						: `Uploading ${done}/${total} chunks`;
			});
			entries = await api.listArchiveEntries(folderId);
			await refreshFolders();
			progress = '';
		} catch (e) {
			reportError(e, 'Upload failed');
			progress = '';
		} finally {
			busy = false;
		}
	}

	async function onFilesPicked(fileList: FileList | null) {
		if (!fileList || fileList.length === 0 || !folderId) return;
		const files = [...fileList];
		if (files.length === 1) {
			const file = files[0];
			await ingestBlob(file.name, file, file.type || undefined);
			return;
		}
		progress = 'Zipping files…';
		busy = true;
		try {
			const zipBlob = await zipFiles(files);
			const base = folderId.slice(0, 8);
			await ingestBlob(`files-${base}.zip`, zipBlob, 'application/zip');
		} catch (e) {
			reportError(e, 'Failed to zip files');
			busy = false;
			progress = '';
		}
	}

	async function onFolderPicked(fileList: FileList | null) {
		if (!fileList || fileList.length === 0 || !folderId) return;
		const files = [...fileList];
		const rootName = files[0]?.webkitRelativePath?.split('/')[0] || 'folder';
		progress = 'Zipping folder…';
		busy = true;
		try {
			const zipBlob = await zipFiles(files);
			await ingestBlob(`${rootName}.zip`, zipBlob, 'application/zip');
		} catch (e) {
			reportError(e, 'Failed to zip folder');
			busy = false;
			progress = '';
		}
	}

	async function zipFiles(files: File[]): Promise<Blob> {
		const tree: Record<string, Uint8Array> = {};
		let total = 0;
		for (const file of files) {
			const path = file.webkitRelativePath || file.name;
			const data = new Uint8Array(await file.arrayBuffer());
			total += data.byteLength;
			tree[path] = data;
		}
		if (total >= ARCHIVE_ZIP_SOFT_WARN_BYTES) {
			const ok = confirm(`Folder is about ${formatBytes(total)} before zip. Continue?`);
			if (!ok) throw new Error('Cancelled');
		}
		const zipped = zipSync(tree, { level: 6 });
		return new Blob([new Uint8Array(zipped)], { type: 'application/zip' });
	}

	async function onDrop(e: DragEvent) {
		e.preventDefault();
		dragOver = false;
		if (disabled || busy || mode !== 'owner' || !folderId) return;
		await onFilesPicked(e.dataTransfer?.files ?? null);
	}

	async function downloadEntry(entry: ArchiveEntry) {
		if (!contactId || mode !== 'friend') return;
		downloadStatus = { ...downloadStatus, [entry.id]: 'Waiting for direct…' };
		try {
			await api.startArchiveFetch(entry.root_hash, contactId);
			for (let i = 0; i < 180; i++) {
				const status = await api.mediaFetchStatus(entry.root_hash);
				const transport = status.transport ? ` · ${status.transport}` : '';
				downloadStatus = {
					...downloadStatus,
					[entry.id]: `${status.state} ${status.chunks_done}/${status.chunks_total}${transport}`
				};
				if (status.state === 'complete') {
					downloadStatus = { ...downloadStatus, [entry.id]: 'Ready (direct)' };
					return;
				}
				if (status.state === 'failed') {
					const err = status.error ?? 'Failed';
					downloadStatus = {
						...downloadStatus,
						[entry.id]: err.includes('direct_required')
							? 'Needs a direct connection (relay blocked for archives)'
							: err
					};
					return;
				}
				await new Promise((r) => setTimeout(r, 500));
			}
			downloadStatus = { ...downloadStatus, [entry.id]: 'Timed out' };
		} catch (e) {
			downloadStatus = {
				...downloadStatus,
				[entry.id]: e instanceof Error ? e.message : 'Failed'
			};
		}
	}

	async function deleteEntry(entry: ArchiveEntry) {
		if (mode !== 'owner' || busy) return;
		if (!confirm(`Remove ${entry.name}?`)) return;
		busy = true;
		try {
			await api.deleteArchiveEntry(entry.id);
			if (folderId) entries = await api.listArchiveEntries(folderId);
			await refreshFolders();
		} catch (e) {
			reportError(e, 'Failed to delete');
		} finally {
			busy = false;
		}
	}
</script>

<div class="finder">
	<header class="chrome">
		<nav class="crumbs" aria-label="Files location">
			<button
				type="button"
				class="crumb"
				class:current={atRoot}
				disabled={disabled || atRoot}
				onclick={goRoot}
			>
				Files
			</button>
			{#if currentFolder}
				<span class="sep" aria-hidden="true">/</span>
				<span class="crumb current" aria-current="page">{currentFolder.name}</span>
			{/if}
		</nav>

		<div class="toolbar">
			{#if mode === 'owner' && atRoot}
				{#if creatingFolder}
					<form
						class="new-folder"
						onsubmit={(e) => {
							e.preventDefault();
							void createFolder();
						}}
					>
						<input
							bind:value={nameDraft}
							placeholder="Folder name"
							disabled={disabled || busy}
							autofocus
						/>
						<button type="submit" class="tool" disabled={disabled || busy || !nameDraft.trim()}>
							Create
						</button>
						<button
							type="button"
							class="tool ghost"
							disabled={busy}
							onclick={() => {
								creatingFolder = false;
								nameDraft = '';
							}}
						>
							Cancel
						</button>
					</form>
				{:else}
					<button
						type="button"
						class="tool"
						disabled={disabled || busy}
						onclick={() => (creatingFolder = true)}
					>
						New folder
					</button>
				{/if}
			{:else if mode === 'owner' && !atRoot}
				<button
					type="button"
					class="tool"
					disabled={disabled || busy}
					onclick={() => fileInput?.click()}
				>
					Add files
				</button>
				<button
					type="button"
					class="tool"
					disabled={disabled || busy}
					onclick={() => folderInput?.click()}
				>
					Add folder
				</button>
			{/if}
		</div>
	</header>

	{#if progress}
		<p class="status muted">{progress}</p>
	{/if}

	<div
		class="pane"
		class:drop-target={!atRoot && mode === 'owner'}
		class:drag-over={dragOver}
		ondragover={(e) => {
			if (atRoot || mode !== 'owner') return;
			e.preventDefault();
			dragOver = true;
		}}
		ondragleave={() => (dragOver = false)}
		ondrop={onDrop}
		role="region"
		aria-label={atRoot ? 'Folders' : 'Folder contents'}
	>
		{#if atRoot}
			{#if folders.length === 0}
				<div class="empty-state">
					<svg class="empty-icon" viewBox="0 0 24 24" aria-hidden="true">
						<path
							fill="currentColor"
							d="M10 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"
						/>
					</svg>
					<p class="empty-title">No folders yet</p>
					<p class="empty-copy muted">
						{#if mode === 'owner'}
							Create a folder to share files with friends. They stay on your device until someone downloads them.
						{:else}
							This friend has not shared any folders yet.
						{/if}
					</p>
				</div>
			{:else}
				<ul class="icon-grid">
					{#each folders as folder (folder.id)}
						<li>
							<button
								type="button"
								class="icon-item"
								disabled={disabled}
								ondblclick={() => openFolder(folder.id)}
								onclick={() => openFolder(folder.id)}
							>
								<svg class="glyph folder" viewBox="0 0 24 24" aria-hidden="true">
									<path
										fill="currentColor"
										d="M10 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"
									/>
								</svg>
								<span class="label">{folder.name}</span>
								{#if entryCount(folder) != null}
									<span class="meta muted">{entryCount(folder)} items</span>
								{/if}
							</button>
						</li>
					{/each}
				</ul>
			{/if}
		{:else if loadingEntries}
			<p class="pane-msg muted">Loading…</p>
		{:else if entries.length === 0}
			<div class="empty-state">
				<p class="empty-title">Empty folder</p>
				<p class="empty-copy muted">
					{#if mode === 'owner'}
						Drop files here, or use Add files / Add folder in the toolbar above.
					{:else}
						Nothing in this folder yet.
					{/if}
				</p>
			</div>
		{:else}
			<ul class="file-table" role="list">
				{#each entries as entry (entry.id)}
					<li class="file-row">
						<span class="file-icon" data-kind={fileGlyph(entry.name, entry.mime)} aria-hidden="true"></span>
						<div class="file-main">
							<span class="file-name">{entry.name}</span>
							<span class="file-size muted">{formatBytes(entry.total_bytes)}</span>
							{#if downloadStatus[entry.id]}
								<span class="file-status muted">{downloadStatus[entry.id]}</span>
							{/if}
						</div>
						<div class="file-actions">
							{#if mode === 'friend'}
								<button
									type="button"
									class="tool accent"
									disabled={disabled}
									onclick={() => downloadEntry(entry)}
								>
									Download
								</button>
							{:else}
								<button
									type="button"
									class="tool danger"
									disabled={disabled || busy}
									onclick={() => deleteEntry(entry)}
								>
									Remove
								</button>
							{/if}
						</div>
					</li>
				{/each}
			</ul>
		{/if}
	</div>

	{#if mode === 'owner'}
		<input
			bind:this={fileInput}
			type="file"
			class="hidden"
			multiple
			disabled={disabled || busy}
			onchange={(e) => {
				const input = e.currentTarget;
				void onFilesPicked(input.files);
				input.value = '';
			}}
		/>
		<input
			bind:this={folderInput}
			type="file"
			class="hidden"
			multiple
			disabled={disabled || busy}
			{...{ webkitdirectory: true, directory: true }}
			onchange={(e) => {
				const input = e.currentTarget;
				void onFolderPicked(input.files);
				input.value = '';
			}}
		/>
	{/if}
</div>

<style>
	.finder {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		overflow: hidden;
		min-height: 16rem;
	}

	.chrome {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 0.55rem 0.85rem;
		padding: 0.55rem 0.75rem;
		border-bottom: 1px solid var(--border);
		background: color-mix(in srgb, var(--bg) 55%, var(--surface));
		position: sticky;
		top: 0;
		z-index: 1;
	}

	.crumbs {
		display: flex;
		align-items: center;
		gap: 0.35rem;
		min-width: 0;
		flex: 1;
		font-size: 0.82rem;
	}

	.crumb {
		font: inherit;
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--accent);
		background: none;
		border: none;
		padding: 0;
		cursor: pointer;
		max-width: 12rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.crumb:disabled {
		cursor: default;
	}

	.crumb.current {
		color: var(--text);
		font-weight: 600;
	}

	.sep {
		color: var(--muted);
		user-select: none;
	}

	.toolbar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: flex-end;
		gap: 0.35rem;
	}

	.new-folder {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.35rem;
	}

	.new-folder input {
		width: 9.5rem;
		max-width: 40vw;
		padding: 0.3rem 0.55rem;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--bg);
		color: var(--text);
		font: inherit;
		font-size: 0.8rem;
	}

	.tool {
		padding: 0.3rem 0.65rem;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: 0.78rem;
		font-weight: 600;
		cursor: pointer;
		white-space: nowrap;
	}

	.tool:hover:not(:disabled) {
		background: color-mix(in srgb, var(--border) 22%, var(--surface));
	}

	.tool:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.tool.ghost {
		background: transparent;
		font-weight: 500;
		color: var(--muted);
	}

	.tool.accent {
		border: none;
		background: var(--accent);
		color: #fff;
	}

	.tool.danger {
		border-color: color-mix(in srgb, #c44 40%, var(--border));
		color: #c44;
	}

	.status {
		margin: 0;
		padding: 0.4rem 0.75rem;
		border-bottom: 1px solid var(--border);
		font-size: 0.78rem;
		background: color-mix(in srgb, var(--accent) 6%, var(--surface));
	}

	.pane {
		flex: 1;
		min-height: 12rem;
		padding: 0.75rem;
		transition: background 0.15s ease, outline-color 0.15s ease;
	}

	.pane.drop-target.drag-over {
		background: color-mix(in srgb, var(--accent) 8%, var(--surface));
		outline: 2px dashed color-mix(in srgb, var(--accent) 55%, var(--border));
		outline-offset: -6px;
	}

	.icon-grid {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(6.5rem, 1fr));
		gap: 0.35rem;
	}

	.icon-item {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.3rem;
		width: 100%;
		padding: 0.7rem 0.4rem 0.55rem;
		border: 1px solid transparent;
		border-radius: 10px;
		background: transparent;
		color: var(--text);
		font: inherit;
		cursor: pointer;
	}

	.icon-item:hover:not(:disabled),
	.icon-item:focus-visible {
		background: color-mix(in srgb, var(--border) 28%, transparent);
		border-color: color-mix(in srgb, var(--border) 70%, transparent);
		outline: none;
	}

	.icon-item:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.glyph {
		width: 2.35rem;
		height: 2.35rem;
	}

	.glyph.folder {
		color: color-mix(in srgb, var(--accent) 72%, var(--warning));
		opacity: 0.95;
	}

	.label {
		font-size: 0.78rem;
		font-weight: 600;
		text-align: center;
		word-break: break-word;
		line-height: 1.25;
	}

	.meta {
		font-size: 0.68rem;
	}

	.file-table {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
	}

	.file-row {
		display: flex;
		align-items: center;
		gap: 0.65rem;
		padding: 0.55rem 0.35rem;
		border-bottom: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
	}

	.file-row:last-child {
		border-bottom: none;
	}

	.file-icon {
		width: 1.55rem;
		height: 1.85rem;
		flex-shrink: 0;
		border-radius: 3px;
		background: color-mix(in srgb, var(--border) 55%, var(--surface));
		position: relative;
	}

	.file-icon::after {
		content: '';
		position: absolute;
		inset: auto 0 0.2rem;
		height: 0.35rem;
		background: color-mix(in srgb, var(--muted) 35%, transparent);
	}

	.file-icon[data-kind='zip'] {
		background: color-mix(in srgb, var(--warning) 28%, var(--surface));
	}

	.file-icon[data-kind='img'] {
		background: color-mix(in srgb, var(--accent) 22%, var(--surface));
	}

	.file-icon[data-kind='vid'] {
		background: color-mix(in srgb, var(--success) 22%, var(--surface));
	}

	.file-main {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.08rem;
	}

	.file-name {
		font-size: 0.85rem;
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.file-size,
	.file-status {
		font-size: 0.72rem;
	}

	.file-actions {
		flex-shrink: 0;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
		gap: 0.35rem;
		min-height: 10rem;
		padding: 1.25rem 1rem;
	}

	.empty-icon {
		width: 2.5rem;
		height: 2.5rem;
		color: var(--muted);
		opacity: 0.55;
		margin-bottom: 0.25rem;
	}

	.empty-title {
		margin: 0;
		font-size: 0.88rem;
		font-weight: 600;
	}

	.empty-copy {
		margin: 0;
		max-width: 22rem;
		font-size: 0.8rem;
		line-height: 1.4;
	}

	.pane-msg {
		margin: 1rem 0;
		font-size: 0.82rem;
	}

	.hidden {
		display: none;
	}

	.muted {
		color: var(--muted);
	}
</style>
