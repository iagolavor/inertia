<script lang="ts">
  import { blobUrl, type ProfilePhoto } from '$lib/api';

  interface Props {
    photos: ProfilePhoto[];
  }

  let { photos }: Props = $props();
</script>

{#if photos.length > 0}
  <div class="photo-grid">
    {#each photos as photo (photo.id)}
      <div class="photo-cell">
        <img src={blobUrl(photo.blob_hash)} alt={photo.caption ?? 'Foto de perfil'} loading="lazy" />
      </div>
    {/each}
  </div>
{/if}

<style>
  .photo-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 3px;
    margin-bottom: 1rem;
  }

  .photo-cell {
    aspect-ratio: 1;
    overflow: hidden;
    background: var(--bg);
  }

  .photo-cell img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
</style>
