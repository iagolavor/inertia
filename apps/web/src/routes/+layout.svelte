<script lang="ts">
  import '../app.css';

  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';

  import AppHeader from '$lib/components/AppHeader.svelte';
  import ApiStatusBanner from '$lib/components/ApiStatusBanner.svelte';
  import { identityState, refreshIdentity } from '$lib/identity.svelte';
  import {
    refreshMessagesOnVisible,
    refreshP2pLive,
    refreshP2pOnAppOpen,
    stopP2pLiveRecovery
  } from '$lib/presence.svelte';
  import { startP2pEventStream, stopP2pEventStream } from '$lib/p2p-events.svelte';
  import { initTheme } from '$lib/theme.svelte';

  let { children } = $props();

  const isWelcome = $derived(page.url.pathname.startsWith('/welcome'));
  const showAppChrome = $derived(!!identityState.identity && !isWelcome);

  $effect(() => {
    if (identityState.loading) return;

    const path = page.url.pathname;
    const suffix = `${page.url.search}${page.url.hash}`;

    if (!identityState.identity && !path.startsWith('/welcome')) {
      void goto(`/welcome${suffix}`, { replaceState: true });
    }
  });

  onMount(() => {
    initTheme();
    refreshIdentity();
    refreshP2pOnAppOpen();
    startP2pEventStream();

    function onVisible() {
      if (document.visibilityState === 'visible') {
        void refreshIdentity({ silent: true });
        refreshP2pOnAppOpen();
        refreshMessagesOnVisible();
      }
    }
    document.addEventListener('visibilitychange', onVisible);

    return () => {
      stopP2pLiveRecovery();
      stopP2pEventStream();
      document.removeEventListener('visibilitychange', onVisible);
    };
  });
</script>

<div class="app-shell">
  {#if showAppChrome}
    <AppHeader />

    <div class="banner-wrap">
      <ApiStatusBanner />
    </div>
  {/if}

  <div class="main-grow" class:welcome-grow={isWelcome && !showAppChrome}>
    <main class="container" class:welcome-container={isWelcome && !showAppChrome}>
      {@render children()}
    </main>
  </div>
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    min-height: 100%;
    min-height: 100dvh;
    background: var(--bg);
  }

  .banner-wrap {
    padding: 0 max(1.5rem, var(--safe-right)) 0 max(1.5rem, var(--safe-left));
  }

  /* Grows vertically; keeps .container at full 720px width (no flex shrink). */
  .main-grow {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    width: 100%;
  }

  .main-grow > main.container {
    width: 100%;
    flex: 1 1 auto;
    min-height: 0;
  }

  .main-grow > main.container:has(:global(.chat-fill)) {
    display: flex;
    flex-direction: column;
  }

  .welcome-grow {
    flex: 1;
  }

  .welcome-container {
    max-width: none;
    padding: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  @media (max-width: 640px) {
    .banner-wrap {
      padding-left: max(1rem, var(--safe-left));
      padding-right: max(1rem, var(--safe-right));
    }
  }
</style>
