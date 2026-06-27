<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';

  $effect(() => {
    const suffix = `${page.url.search}${page.url.hash}`;
    void goto(`/welcome${suffix}`, { replaceState: true });
  });
</script>

<p class="empty">Redirecting…</p>
