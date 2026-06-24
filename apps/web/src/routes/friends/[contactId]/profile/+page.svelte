<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { api, type Contact, type FeedItem } from '$lib/api';
	import { ApiRequestError } from '$lib/api-errors';
	import FriendPresenceHeader from '$lib/components/FriendPresenceHeader.svelte';
	import PhotoGrid from '$lib/components/PhotoGrid.svelte';
	import { identityState } from '$lib/identity.svelte';
	import { feedPostsToProfilePhotos } from '$lib/profile-photos';
	import { formatCacheAge, readCachedFeed, readCachedMessages } from '$lib/local-cache';

	let contacts = $state<Contact[]>([]);
	let feed = $state<FeedItem[]>([]);
	let loading = $state(true);
	let error = $state('');
	let showingCached = $state(false);
	let cacheAge = $state<string | null>(null);
	let selectedContentId = $state<string | null>(null);
	let selectedPost = $state<FeedItem | null>(null);
	let selectedPostLoading = $state(false);

	const contactId = $derived(page.params.contactId);
	const contact = $derived(contacts.find((c) => c.id === contactId) ?? null);
	const photos = $derived(
		contact ? feedPostsToProfilePhotos(feed, contact.signing_pubkey) : []
	);

	async function hydrateFromCache() {
		if (!contactId) return false;
		const [feedCache, rosterCache] = await Promise.all([readCachedFeed(), readCachedMessages()]);
		if (rosterCache) contacts = rosterCache.contacts;
		if (!feedCache) return false;
		feed = feedCache.items;
		cacheAge = formatCacheAge(feedCache.saved_at);
		showingCached = true;
		return true;
	}

	async function loadFeed() {
		if (!identityState.apiOnline) return;
		feed = await api.listFeed();
		showingCached = false;
		cacheAge = null;
	}

	async function load() {
		if (!contactId) return;

		if (!identityState.apiOnline) {
			loading = true;
			error = '';
			await hydrateFromCache();
			loading = false;
			return;
		}

		loading = true;
		error = '';
		try {
			contacts = await api.listContacts();
			await loadFeed();
		} catch (e) {
			const hadCache = await hydrateFromCache();
			if (!hadCache) {
				error = e instanceof ApiRequestError ? e.message : 'Failed to load posts';
			}
		} finally {
			loading = false;
		}
	}

	async function selectPost(contentId: string | null) {
		selectedContentId = contentId;
		if (!contentId) {
			selectedPost = null;
			selectedPostLoading = false;
			return;
		}

		if (!identityState.apiOnline) return;

		selectedPostLoading = true;
		selectedPost = null;
		error = '';

		try {
			selectedPost = await api.getPost(contentId);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to open post';
			selectedContentId = null;
		} finally {
			selectedPostLoading = false;
		}
	}

	async function onCommentAdded() {
		if (selectedPost && identityState.apiOnline) {
			try {
				selectedPost = await api.getPost(selectedPost.content_id);
				await loadFeed();
			} catch {
				// ignore refresh errors
			}
		}
	}

	onMount(() => {
		void hydrateFromCache().then(() => load());
	});
</script>

<a class="chat-back-link" href="/friends">← Messages</a>

{#if loading}
	<p class="empty">Loading…</p>
{:else if !contact}
	<p class="error">Friend not found.</p>
{:else}
	<FriendPresenceHeader
		{contact}
		messageHref="/friends/{contact.id}"
		detail="{photos.length} posts"
		cacheAge={showingCached ? cacheAge : null}
	/>

	{#if !identityState.apiOnline}
		<p class="offline-hint muted">Showing cached posts — reconnect the API to comment.</p>
	{/if}

	<PhotoGrid
		{photos}
		readonly
		emptyLabel="No photo posts yet from {contact.display_name}."
		disabled={!identityState.apiOnline}
		selectedContentId={selectedContentId}
		{selectedPost}
		selectedPostLoading={selectedPostLoading}
		onselect={selectPost}
		oncomment={onCommentAdded}
	/>

	{#if error}
		<p class="error">{error}</p>
	{/if}
{/if}

<style>
	.offline-hint {
		margin: 0.65rem 0 0;
		font-size: 0.85rem;
	}
</style>
