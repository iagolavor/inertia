<script lang="ts">
  import { onMount } from 'svelte';
  import { api, type FeedItem } from '$lib/api';
  import PostComposer from '$lib/components/PostComposer.svelte';
  import PostCard from '$lib/components/PostCard.svelte';
  import { identityState } from '$lib/identity.svelte';

  let feed = $state<FeedItem[]>([]);
  let feedLoading = $state(false);
  let feedError = $state('');

  async function loadFeed() {
    if (!identityState.apiOnline || !identityState.identity) return;
    feedLoading = true;
    feedError = '';
    try {
      feed = await api.listFeed();
    } catch (e) {
      feedError = e instanceof Error ? e.message : 'Falha ao carregar feed';
    } finally {
      feedLoading = false;
    }
  }

  onMount(() => {
    void loadFeed();
  });
</script>

<h1>Inertia</h1>
<p class="subtitle">Ephemeral P2P social — zero tracking, zero ads, your circle only.</p>

{#if identityState.loading}
  <p class="empty">Loading...</p>
{:else if !identityState.apiOnline}
  <div class="card">
    <h2>API offline</h2>
    <p>Start the Rust API bridge before using the app:</p>
    <pre class="cmd">cargo run -p inertia-api</pre>
  </div>
{:else if identityState.identity}
  <div class="card feed-composer">
    <PostComposer
      disabled={!identityState.apiOnline}
      onposted={loadFeed}
    />
  </div>

  <div class="card feed-list">
    {#if feedLoading && feed.length === 0}
      <p class="empty">A carregar feed…</p>
    {:else if feedError}
      <p class="error">{feedError}</p>
    {:else if feed.length === 0}
      <p class="empty">Ainda sem posts. Publica algo ou convida um amigo.</p>
    {:else}
      {#each feed as post (post.content_id)}
        <PostCard {post} />
      {/each}
    {/if}
  </div>
{:else}
  <div class="card">
    <h2>Get started</h2>
    <p class="muted">Create a local profile to connect with people you trust.</p>
    <p style="margin-top: 1rem;">
      <a class="btn" href="/profile">Create your profile</a>
    </p>
  </div>
{/if}

<div class="card">
  <h3>How it works</h3>
  <ul class="muted list">
    <li>Invite links expire in 15 minutes and work only once</li>
    <li>Messages expire after 7 days; posts after 48 hours</li>
    <li>Delivery is direct P2P when both of you are online</li>
    <li>No ads, no algorithms, no doomscrolling</li>
  </ul>
</div>

<style>
  .muted {
    color: var(--muted);
  }

  .feed-composer {
    padding: 0.85rem;
  }

  .feed-list {
    padding-top: 0.5rem;
  }

  .list {
    padding-left: 1.25rem;
    margin: 0;
  }

  .cmd {
    background: var(--bg);
    padding: 1rem;
    border-radius: 8px;
    overflow-x: auto;
  }
</style>
