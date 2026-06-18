<script lang="ts">
  import '../app.css';

  import { onMount } from 'svelte';
  import { afterNavigate } from '$app/navigation';

  import NavDrawer from '$lib/components/NavDrawer.svelte';
  import { refreshIdentity } from '$lib/identity.svelte';
  import { initTheme } from '$lib/theme.svelte';

  let { children } = $props();

  let drawerOpen = $state(false);
  let navigated = false;

  onMount(() => {
    initTheme();
    refreshIdentity();
  });

  afterNavigate(() => {
    drawerOpen = false;

    if (!navigated) {
      navigated = true;
      return;
    }

    void refreshIdentity({ silent: true });
  });
</script>

<NavDrawer bind:open={drawerOpen} />

<main class="container">
  {@render children()}
</main>
