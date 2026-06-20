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

<AppHeader />

<div class="banner-wrap">
  <ApiStatusBanner />
</div>

<main class="container">
  {@render children()}
</main>

<style>
  .banner-wrap {
    padding: 0 1.5rem;
  }
</style>
