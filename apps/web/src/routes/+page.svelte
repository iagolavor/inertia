<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type FeedItem } from '$lib/api';
  import { ApiRequestError } from '$lib/api-errors';
  import PostComposer from '$lib/components/PostComposer.svelte';
  import PostCard from '$lib/components/PostCard.svelte';
  import PostDetailModal from '$lib/components/PostDetailModal.svelte';
  import { identityState } from '$lib/identity.svelte';
  import { formatCacheAge, readCachedFeed, writeCachedFeed } from '$lib/local-cache';
  import { startFeedPolling, stopFeedPolling } from '$lib/presence.svelte';

  type FeedRow = FeedItem & {
    local_media_preview?: string;
    delivering?: boolean;
  };

  let feed = $state<FeedRow[]>([]);
  let feedLoading = $state(false);
  let feedError = $state('');
  let showingCached = $state(false);
  let cacheAge = $state<string | null>(null);
  let selectedPost = $state<FeedItem | null>(null);
  let detailOpen = $state(false);

  async function hydrateFromCache() {
    const cached = await readCachedFeed();
    if (!cached) return false;
    feed = cached.items;
    cacheAge = formatCacheAge(cached.saved_at);
    showingCached = true;
    return true;
  }

  async function openPost(post: FeedItem) {
    selectedPost = post;
    detailOpen = true;
  }

  async function onCommentAdded() {
    await loadFeed();
    if (selectedPost) {
      const updated = feed.find((p) => p.content_id === selectedPost!.content_id);
      if (updated) selectedPost = updated;
    }
  }

  async function loadFeed() {
    if (!identityState.identity) return;

    if (!identityState.apiOnline) {
      await hydrateFromCache();
      return;
    }

    feedLoading = true;
    feedError = '';
    try {
      const items = await api.listFeed();
      feed = items;
      showingCached = false;
      cacheAge = null;
      await writeCachedFeed(items);
      if (selectedPost) {
        const updated = feed.find((p) => p.content_id === selectedPost!.content_id);
        if (updated) selectedPost = updated;
      }
    } catch (e) {
      const hadCache = await hydrateFromCache();
      if (hadCache) {
        feedError = '';
      } else if (e instanceof ApiRequestError) {
        feedError = e.message;
      } else {
        feedError = e instanceof Error ? e.message : 'Falha ao carregar feed';
      }
    } finally {
      feedLoading = false;
    }
  }

  function onPosted(result: {
    content_id: string;
    body: string;
    local_media_preview?: string;
    media_kind?: 'photo' | 'video';
    media_ready?: boolean;
  }) {
    if (!identityState.identity) return;

    const now = new Date().toISOString();
    const expires = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString();
    const optimistic: FeedRow = {
      content_id: result.content_id,
      author_id: identityState.identity.signing_pubkey,
      author_name: identityState.identity.display_name,
      body: result.body,
      media_ref: null,
      media_kind: result.media_kind ?? null,
      media_ready: result.media_ready ?? false,
      local_media_preview: result.local_media_preview,
      delivering: true,
      created_at: now,
      expires_at: expires,
      is_own: true,
      is_archived: false
    };

    feed = [optimistic, ...feed.filter((p) => p.content_id !== result.content_id)];
    void loadFeed();
  }

  onMount(() => {
    void hydrateFromCache().then(() => loadFeed());
    startFeedPolling(loadFeed);

    function onVisible() {
      if (document.visibilityState === 'visible') {
        void loadFeed();
      }
    }
    document.addEventListener('visibilitychange', onVisible);
    return () => {
      stopFeedPolling();
      document.removeEventListener('visibilitychange', onVisible);
    };
  });
</script>

{#if identityState.loading || !identityState.identity}
  <p class="empty">Loading...</p>
{:else}
  <h1 class="page-title">Feed</h1>
  <p class="subtitle">Ephemeral P2P social. No tracking, no ads, just your friends.</p>

  {#if !identityState.apiOnline}
    <p class="offline-hint muted">
      You're viewing offline. Posting and comments are paused until the API is back.
    </p>
  {/if}

  <section class="feed-composer">
    <PostComposer
      disabled={!identityState.apiOnline}
      onposted={onPosted}
    />
  </section>

  {#if showingCached && cacheAge}
    <p class="cache-hint muted">Saved · {cacheAge}</p>
  {/if}

  <div class="feed-list">
    {#if feedLoading && feed.length === 0}
      <p class="empty">A carregar feed…</p>
    {:else if feedError}
      <p class="error">{feedError}</p>
    {:else if feed.length === 0}
      <p class="empty">Ainda sem posts. Publica algo ou convida um amigo.</p>
    {:else}
      {#each feed as post (post.content_id)}
        <PostCard {post} onopen={openPost} />
      {/each}
    {/if}
  </div>

  <PostDetailModal
    open={detailOpen}
    post={selectedPost}
    disabled={!identityState.apiOnline}
    onclose={() => (detailOpen = false)}
    oncomment={onCommentAdded}
  />
{/if}

<style>
  .page-title {
    margin: 0 0 0.25rem;
    font-size: 1.35rem;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .muted {
    color: var(--muted);
  }

  .offline-hint {
    margin: 0 0 1rem;
    font-size: 0.875rem;
  }

  .feed-composer {
    margin-bottom: 1.25rem;
  }

  .cache-hint {
    margin: 0 0 0.75rem;
    font-size: 0.8rem;
  }

  .feed-list {
    display: flex;
    flex-direction: column;
  }
</style>
