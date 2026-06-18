<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { afterNavigate } from '$app/navigation';
  import { page } from '$app/state';
  import Avatar from '$lib/components/Avatar.svelte';
  import { identityState, refreshIdentity } from '$lib/identity.svelte';

  let { children } = $props();
  let navigated = false;

  onMount(() => refreshIdentity());

  afterNavigate(() => {
    if (!navigated) {
      navigated = true;
      return;
    }
    void refreshIdentity({ silent: true });
  });
</script>

<nav>
  <a href="/" class:active={page.url.pathname === '/'}>Home</a>
  <a href="/profile" class:active={page.url.pathname === '/profile'}>Profile</a>
  <a href="/friends" class:active={page.url.pathname === '/friends'}>Friends</a>
  <a href="/invite" class:active={page.url.pathname === '/invite'}>Accept invite</a>
  <a href="/messages" class:active={page.url.pathname === '/messages'}>Messages</a>
  <a href="/outbox" class:active={page.url.pathname === '/outbox'}>Outbox</a>
  {#if identityState.identity}
    <a href="/profile" class="nav-profile" aria-label="Your profile">
      <Avatar
        seed={identityState.identity.signing_pubkey}
        alt={identityState.identity.display_name}
        size={28}
      />
    </a>
  {/if}
</nav>

<main class="container">
  {@render children()}
</main>

<style>
  nav {
    align-items: center;
  }

  .nav-profile {
    margin-left: auto;
    display: flex;
    line-height: 0;
  }

  .nav-profile:hover {
    text-decoration: none;
  }
</style>
