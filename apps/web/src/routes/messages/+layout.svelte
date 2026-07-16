<script lang="ts">
  import { onMount } from 'svelte';
  import { refreshInboxSilently } from '$lib/messages-sync';
  import { registerInboxRefresh } from '$lib/presence.svelte';

  let { children } = $props();

  onMount(() => {
    void refreshInboxSilently();
    registerInboxRefresh(refreshInboxSilently);
    return () => registerInboxRefresh(null);
  });
</script>

{@render children()}
