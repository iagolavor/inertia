<script lang="ts">

  import '../app.css';

  import { onMount } from 'svelte';

  import { afterNavigate } from '$app/navigation';

  import { page } from '$app/state';

  import Avatar from '$lib/components/Avatar.svelte';
  import OnlineStatus from '$lib/components/OnlineStatus.svelte';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';

  import { identityState, refreshIdentity } from '$lib/identity.svelte';

  import { initTheme } from '$lib/theme.svelte';



  let { children } = $props();

  let navigated = false;



  onMount(() => {

    initTheme();

    refreshIdentity();

  });



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



  <div class="nav-actions">

    <ThemeToggle />

    {#if identityState.identity}

      <div class="nav-profile">
        <OnlineStatus online={identityState.apiOnline} loading={identityState.loading} compact />
        <a href="/profile" class="nav-profile-link" aria-label="Your profile">
          <Avatar
            seed={identityState.identity.signing_pubkey}
            alt={identityState.identity.display_name}
            size={28}
          />
        </a>
      </div>

    {:else}

      <span class="nav-status-only" title="API connection status">
        <OnlineStatus online={identityState.apiOnline} loading={identityState.loading} compact />
      </span>

    {/if}

  </div>

</nav>



<main class="container">

  {@render children()}

</main>



<style>

  nav {

    align-items: center;

  }



  .nav-actions {

    margin-left: auto;

    display: flex;

    align-items: center;

    gap: 0.75rem;

  }



  .nav-profile {
    display: flex;
    align-items: center;
    gap: 0.45rem;
  }

  .nav-profile-link {
    display: flex;
    line-height: 0;
  }

  .nav-profile-link:hover {
    text-decoration: none;
  }



  .nav-status-only {

    display: flex;

    align-items: center;

  }

</style>


