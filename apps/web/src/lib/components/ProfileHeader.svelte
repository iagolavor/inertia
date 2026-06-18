<script lang="ts">
  import Avatar from './Avatar.svelte';

  interface Props {
    displayName: string;
    seed: string;
    avatarUrl?: string | null;
    size?: number;
    children?: import('svelte').Snippet;
  }

  let {
    displayName,
    seed,
    avatarUrl = null,
    size = 72,
    children
  }: Props = $props();
</script>

<div class="profile-header">
  <Avatar {seed} alt={displayName} src={avatarUrl} {size} />
  <div class="profile-meta">
    <h2 class="profile-name">{displayName}</h2>
    {#if children}
      {@render children()}
    {/if}
  </div>
</div>

<style>
  .profile-header {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .profile-meta {
    min-width: 0;
    flex: 1;
  }

  .profile-name {
    margin: 0;
    font-size: 1.35rem;
    line-height: 1.3;
  }
</style>
