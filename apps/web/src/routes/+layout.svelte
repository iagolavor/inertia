<script lang="ts">
  import '../app.css';

  import { onMount } from 'svelte';
  import { afterNavigate } from '$app/navigation';

  import AppHeader from '$lib/components/AppHeader.svelte';
  import { refreshIdentity } from '$lib/identity.svelte';
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

<AppHeader />

<main class="container">
  {@render children()}
</main>
