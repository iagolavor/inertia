<script lang="ts">
  import { api, blobUrl, type ProfilePhoto } from '$lib/api';
  import { prepareImageForUpload } from '$lib/image';

  interface Props {
    photos: ProfilePhoto[];
    disabled?: boolean;
    onuploaded?: () => void;
  }

  let { photos, disabled = false, onuploaded }: Props = $props();

  let uploading = $state(false);
  let error = $state('');
  let fileInput = $state<HTMLInputElement | null>(null);

  function openPicker() {
    if (uploading || disabled) return;
    fileInput?.click();
  }

  async function onFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;

    uploading = true;
    error = '';
    try {
      const dataBase64 = await prepareImageForUpload(file);
      await api.uploadProfilePhoto(dataBase64);
      onuploaded?.();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Falha ao carregar foto';
    } finally {
      uploading = false;
    }
  }
</script>

<div class="photo-section">
  <div class="photo-grid">
    <button
      type="button"
      class="photo-cell add-cell"
      onclick={openPicker}
      disabled={uploading || disabled}
      aria-label="Adicionar foto"
      title="Adicionar foto"
    >
      {#if uploading}
        <span class="spinner" aria-hidden="true"></span>
      {:else}
        <span class="plus">+</span>
      {/if}
    </button>

    {#each photos as photo (photo.id)}
      <div class="photo-cell">
        <img src={blobUrl(photo.blob_hash)} alt={photo.caption ?? 'Foto de perfil'} loading="lazy" />
      </div>
    {/each}
  </div>

  <input
    bind:this={fileInput}
    type="file"
    accept="image/jpeg,image/png,image/gif,image/webp,image/*"
    class="file-input"
    disabled={uploading || disabled}
    onchange={onFileSelect}
  />

  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .photo-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 3px;
    margin-bottom: 0.5rem;
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

  .add-cell {
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px dashed var(--border);
    background: var(--surface);
    cursor: pointer;
    color: var(--muted);
  }

  .add-cell:hover:not(:disabled) {
    color: var(--accent);
    border-color: var(--accent);
  }

  .add-cell:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .plus {
    font-size: 1.75rem;
    line-height: 1;
    font-weight: 300;
  }

  .spinner {
    width: 1.25rem;
    height: 1.25rem;
    border: 2px solid color-mix(in srgb, var(--accent) 30%, transparent);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .file-input {
    display: none;
  }

  .error {
    margin: 0;
    font-size: 0.8rem;
  }
</style>
