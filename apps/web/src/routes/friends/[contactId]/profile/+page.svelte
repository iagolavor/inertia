<script lang="ts">
	import { page } from '$app/state';
	import {
		api,
		type ArchiveFolderSummary,
		type Contact,
		type FeedItem,
		type ProfileManifest,
		type ProfilePhoto
	} from '$lib/api';
	import { ApiRequestError } from '$lib/api-errors';
	import FilesPanel from '$lib/components/FilesPanel.svelte';
	import FriendPresenceHeader from '$lib/components/FriendPresenceHeader.svelte';
	import PhotoGrid from '$lib/components/PhotoGrid.svelte';
	import { identityState } from '$lib/identity.svelte';
	import { profileItemToFeedItem } from '$lib/profile-photos';
	import { formatCacheAge, readCachedMessages } from '$lib/local-cache';

	let contacts = $state<Contact[]>([]);
	let manifest = $state<ProfileManifest | null>(null);
	let loading = $state(true);
	let error = $state('');
	let offlineFriend = $state(false);
	let showingCached = $state(false);
	let cacheAge = $state<string | null>(null);
	let selectedItemId = $state<string | null>(null);
	let selectedPost = $state<FeedItem | null>(null);
	let selectedPostLoading = $state(false);
	let profileTab = $state<'posts' | 'files'>('posts');

	const contactId = $derived(page.params.contactId);
	const contact = $derived(contacts.find((c) => c.id === contactId) ?? null);
	const photos = $derived(manifest?.items ?? ([] as ProfilePhoto[]));
	const archiveFolders = $derived(manifest?.archive_folders ?? ([] as ArchiveFolderSummary[]));

	async function hydrateRoster() {
		const rosterCache = await readCachedMessages();
		if (rosterCache) {
			contacts = rosterCache.contacts;
			cacheAge = formatCacheAge(rosterCache.saved_at);
			showingCached = true;
			return true;
		}
		return false;
	}

	async function load() {
		if (!contactId) return;

		if (!identityState.apiOnline) {
			loading = true;
			error = '';
			await hydrateRoster();
			offlineFriend = true;
			loading = false;
			return;
		}

		loading = true;
		error = '';
		offlineFriend = false;
		try {
			contacts = await api.listContacts();
			showingCached = false;
			cacheAge = null;
			manifest = await api.fetchFriendProfile(contactId);
		} catch (e) {
			await hydrateRoster();
			const msg = e instanceof ApiRequestError ? e.message : e instanceof Error ? e.message : 'Failed to load profile';
			if (/offline|no peer|p2p not started|timed out/i.test(msg)) {
				offlineFriend = true;
				error = 'Friend is offline. Profile photos load when they are online.';
			} else {
				error = msg;
			}
			manifest = null;
		} finally {
			loading = false;
		}
	}

	async function selectItem(itemId: string | null) {
		selectedItemId = itemId;
		if (!itemId || !manifest || !contact) {
			selectedPost = null;
			selectedPostLoading = false;
			return;
		}
		selectedPostLoading = true;
		const item = manifest.items.find((p) => p.id === itemId);
		selectedPost = item
			? profileItemToFeedItem(item, manifest.signing_pubkey, manifest.display_name)
			: null;
		selectedPostLoading = false;
	}

	// Identity boots async in layout; reload when API + identity become ready.
	$effect(() => {
		const id = contactId;
		if (!id) return;
		if (identityState.loading) return;
		// Re-run when API comes online after a cold start.
		void identityState.apiOnline;
		void load();
	});
</script>

<a class="chat-back-link" href="/messages">← Messages</a>

{#if loading}
	<p class="empty">Loading…</p>
{:else if !contact}
	<p class="error">Friend not found.</p>
{:else}
	<FriendPresenceHeader
		{contact}
		messageHref="/friends/{contact.id}"
		detail="{photos.length} photos"
		cacheAge={showingCached ? cacheAge : null}
	/>

	{#if offlineFriend || !identityState.apiOnline}
		<p class="offline-hint muted">
			{#if !identityState.apiOnline}
				API offline. Reconnect to load this profile.
			{:else}
				Friend is offline. Profile and files are available when they are online.
			{/if}
		</p>
	{/if}

	{#if manifest}
		{#if manifest.bio}
			<p class="friend-bio">{manifest.bio}</p>
		{/if}

		<div class="grid-tabs">
			<div class="tab-row">
				<button
					type="button"
					class="grid-tab"
					class:active={profileTab === 'posts'}
					aria-current={profileTab === 'posts' ? 'page' : undefined}
					onclick={() => (profileTab = 'posts')}
				>
					<svg viewBox="0 0 24 24" aria-hidden="true">
						<rect x="3" y="3" width="7" height="7" rx="1" fill="none" stroke="currentColor" stroke-width="1.75" />
						<rect x="14" y="3" width="7" height="7" rx="1" fill="none" stroke="currentColor" stroke-width="1.75" />
						<rect x="3" y="14" width="7" height="7" rx="1" fill="none" stroke="currentColor" stroke-width="1.75" />
						<rect x="14" y="14" width="7" height="7" rx="1" fill="none" stroke="currentColor" stroke-width="1.75" />
					</svg>
					<span>Posts</span>
				</button>
				<button
					type="button"
					class="grid-tab"
					class:active={profileTab === 'files'}
					aria-current={profileTab === 'files' ? 'page' : undefined}
					onclick={() => (profileTab = 'files')}
				>
					<svg viewBox="0 0 24 24" aria-hidden="true">
						<path
							d="M10 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"
							fill="none"
							stroke="currentColor"
							stroke-width="1.75"
						/>
					</svg>
					<span>Files</span>
				</button>
			</div>
		</div>

		{#if profileTab === 'posts'}
			<p class="tab-blurb muted">
				Photos load from your friend's device when they are online.
			</p>
			<div class="tab-panel">
				<header class="panel-chrome">
					<span class="panel-title">Posts</span>
				</header>
				<div class="panel-body">
					<PhotoGrid
						{photos}
						readonly
						emptyLabel="No photos yet from {contact.display_name}."
						disabled={!identityState.apiOnline || offlineFriend}
						selectedItemId={selectedItemId}
						{selectedPost}
						selectedPostLoading={selectedPostLoading}
						ownerContactId={contactId}
						onselect={selectItem}
					/>
				</div>
			</div>
		{:else}
			<p class="tab-blurb muted">
				Browse when your friend is online. Downloads need a direct (hole-punched) connection -
				not the relay - and can resume after interruptions.
			</p>
			<FilesPanel
				mode="friend"
				folders={archiveFolders}
				contactId={contactId}
				disabled={!identityState.apiOnline || offlineFriend}
				onerror={(msg) => (error = msg)}
			/>
		{/if}
	{/if}

	{#if error}
		<p class="error">{error}</p>
	{/if}
{/if}

<style>
	.offline-hint {
		margin: 0.65rem 0 0;
		font-size: 0.85rem;
	}

	.friend-bio {
		margin: 0.75rem 0 0;
		font-size: 0.9rem;
		line-height: 1.45;
		color: var(--text);
	}

	.grid-tabs {
		display: flex;
		align-items: center;
		justify-content: flex-start;
		gap: 0.75rem;
		border-bottom: 1px solid var(--border);
		margin: 1rem 0 0.75rem;
		padding-bottom: 0.5rem;
	}

	.tab-row {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.tab-blurb {
		margin: 0 0 0.75rem;
		font-size: 0.8rem;
		line-height: 1.4;
		max-width: 40rem;
	}

	.tab-panel {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		overflow: hidden;
	}

	.panel-chrome {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		justify-content: space-between;
		gap: 0.55rem 0.85rem;
		padding: 0.55rem 0.75rem;
		border-bottom: 1px solid var(--border);
		background: color-mix(in srgb, var(--bg) 55%, var(--surface));
	}

	.panel-title {
		font-size: 0.82rem;
		font-weight: 600;
		color: var(--text);
	}

	.panel-body {
		padding: 0.75rem;
	}

	.grid-tab {
		display: inline-flex;
		align-items: center;
		gap: 0.35rem;
		padding: 0.45rem 0;
		/* font shorthand must come before size/weight overrides */
		font: inherit;
		font-size: 0.8rem;
		font-weight: 600;
		letter-spacing: 0;
		text-transform: none;
		color: var(--muted);
		border: none;
		border-bottom: 2px solid transparent;
		margin-bottom: -1px;
		background: transparent;
		cursor: pointer;
	}

	.grid-tab svg {
		width: 0.85rem;
		height: 0.85rem;
	}

	.grid-tab.active {
		color: var(--text);
		border-bottom-color: var(--text);
	}

	.muted {
		color: var(--muted);
		font-size: 0.85rem;
	}
</style>
