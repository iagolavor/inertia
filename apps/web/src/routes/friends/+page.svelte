<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  onMount(() => {
    void goto('/connections', { replaceState: true });
  });
</script>

<p class="empty">Redirecting…</p>
