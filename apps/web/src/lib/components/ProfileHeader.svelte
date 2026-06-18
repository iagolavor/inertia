<script lang="ts">
  import Avatar from './Avatar.svelte';
  import StatusDot from './StatusDot.svelte';

  interface Props {
    displayName: string;
    seed: string;
    avatarUrl?: string | null;
    size?: number;
    online?: boolean;
    statusLoading?: boolean;
    children?: import('svelte').Snippet;
  }

  let {
    displayName,
    seed,
    avatarUrl = null,
    size = 72,
    online = false,
    statusLoading = false,
    children
  }: Props = $props();
</script>

<div class="profile-header">
  <div class="avatar-wrap">
    <Avatar {seed} alt={displayName} src={avatarUrl} {size} />
    <span class="avatar-status">
      <StatusDot {online} loading={statusLoading} size={12} bordered />
    </span>
  </div>
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

  .avatar-wrap {
    position: relative;
    flex-shrink: 0;
    line-height: 0;
  }

  .avatar-wrap .avatar-status {
    position: absolute;
    right: -2px;
    bottom: -2px;
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
