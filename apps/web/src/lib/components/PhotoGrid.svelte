<script lang="ts">
  import { blobUrl, type ProfilePhoto } from '$lib/api';

  interface Props {
    photos: ProfilePhoto[];
    onupload?: (file: File) => void;
    uploading?: boolean;
  }

  let { photos, onupload, uploading = false }: Props = $props();

  function onFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (file) onupload?.(file);
    input.value = '';
  }
</script>

<label class="add-btn" class:disabled={uploading}>
  <input type="file" accept="image/*" onchange={onFileSelect} hidden disabled={uploading} />
  {uploading ? '…' : '+ adicionar'}
</label>

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
  .add-btn {
    display: inline-block;
    margin-bottom: 0.75rem;
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--accent);
    cursor: pointer;
  }

  .add-btn:hover:not(.disabled) {
    text-decoration: underline;
  }

  .add-btn.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .photo-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 3px;
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
