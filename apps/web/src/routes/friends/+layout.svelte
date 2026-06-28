<script lang="ts">
  import { onMount } from 'svelte';
  import { refreshInboxSilently } from '$lib/messages-sync';
  import { startInboxPolling, stopInboxPolling } from '$lib/presence.svelte';

  let { children } = $props();

  onMount(() => {
    void refreshInboxSilently();
    startInboxPolling(refreshInboxSilently);
    return () => stopInboxPolling();
  });
</script>

{@render children()}
