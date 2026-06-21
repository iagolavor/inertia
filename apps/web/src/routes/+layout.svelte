<script lang="ts">
  import '../app.css';

  import { onMount } from 'svelte';
  import { afterNavigate } from '$app/navigation';

  import AppHeader from '$lib/components/AppHeader.svelte';
  import ApiStatusBanner from '$lib/components/ApiStatusBanner.svelte';
  import { refreshIdentity } from '$lib/identity.svelte';
  import { startPresencePolling, stopPresencePolling } from '$lib/presence.svelte';
  import { initTheme } from '$lib/theme.svelte';

  let { children } = $props();

  let navigated = false;

  onMount(() => {
    initTheme();
    refreshIdentity();
    startPresencePolling();

    function onVisible() {
      if (document.visibilityState === 'visible') {
        void refreshIdentity({ silent: true });
      }
    }
    document.addEventListener('visibilitychange', onVisible);

    return () => {
      stopPresencePolling();
      document.removeEventListener('visibilitychange', onVisible);
    };
  });

  afterNavigate(() => {
    if (!navigated) {
      navigated = true;
      return;
    }

    void refreshIdentity({ silent: true });
  });
</script>

<div class="app-shell">
  <AppHeader />

  <div class="banner-wrap">
    <ApiStatusBanner />
  </div>

  <main class="container">
    {@render children()}
  </main>
</div>

<style>
  .app-shell {
    min-height: 100%;
    min-height: 100dvh;
    background: var(--bg);
  }

  .banner-wrap {
    padding: 0 max(1.5rem, var(--safe-right)) 0 max(1.5rem, var(--safe-left));
  }

  @media (max-width: 640px) {
    .banner-wrap {
      padding-left: max(1rem, var(--safe-left));
      padding-right: max(1rem, var(--safe-right));
    }
  }
</style>
