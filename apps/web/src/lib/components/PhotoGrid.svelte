<script lang="ts">
  import { api, blobUrl, type ProfilePhoto } from '$lib/api';
  import { prepareImageForUpload } from '$lib/image';

  interface Props {
    photos: ProfilePhoto[];
    disabled?: boolean;
    /** Override blob URL resolution (e.g. offline cached previews). */
    photoUrl?: (hash: string) => string;
    onuploaded?: () => void;
    onopenpost?: (contentId: string) => void;
  }

  let { photos, disabled = false, photoUrl, onuploaded, onopenpost }: Props = $props();

  const urlFor = (hash: string) => (photoUrl ? photoUrl(hash) : blobUrl(hash));

  let uploading = $state(false);
  let error = $state('');
  let fileInput = $state<HTMLInputElement | null>(null);
  let pendingBase64 = $state<string | null>(null);
  let captionDraft = $state('');
  let captionOpen = $state(false);

  function openPicker() {
    if (uploading || disabled) return;
    fileInput?.click();
  }

  async function onFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;

    error = '';
    try {
      pendingBase64 = await prepareImageForUpload(file);
      captionDraft = '';
      captionOpen = true;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to process image';
    }
  }

  function cancelCaption() {
    captionOpen = false;
    pendingBase64 = null;
    captionDraft = '';
  }

  async function publishPhoto() {
    if (!pendingBase64 || uploading) return;
    uploading = true;
    error = '';
    try {
      await api.uploadProfilePhoto(
        pendingBase64,
        captionDraft.trim() || undefined
      );
      captionOpen = false;
      pendingBase64 = null;
      captionDraft = '';
      onuploaded?.();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to upload photo';
    } finally {
      uploading = false;
    }
  }

  function openPhoto(photo: ProfilePhoto) {
    if (photo.content_id) {
      onopenpost?.(photo.content_id);
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
      aria-label="Add photo"
      title="Add photo"
    >
      {#if uploading}
        <span class="spinner" aria-hidden="true"></span>
      {:else}
        <span class="plus">+</span>
      {/if}
    </button>

    {#each photos as photo (photo.id)}
      <button
        type="button"
        class="photo-cell photo-btn"
        onclick={() => openPhoto(photo)}
        disabled={!photo.content_id}
        aria-label={photo.caption ?? 'Open post'}
      >
        <img src={urlFor(photo.blob_hash)} alt={photo.caption ?? 'Profile photo'} loading="lazy" />
      </button>
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

{#if captionOpen}
  <div class="caption-backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && cancelCaption()}>
    <div class="caption-modal" role="dialog" aria-modal="true" aria-labelledby="caption-title">
      <h2 id="caption-title">New post</h2>
      <p class="caption-hint">Add a description for your photo (optional).</p>
      <textarea
        bind:value={captionDraft}
        rows="3"
        placeholder="Write a caption…"
        disabled={uploading}
      ></textarea>
      <div class="caption-actions">
        <button type="button" class="btn-secondary" onclick={cancelCaption} disabled={uploading}>
          Cancel
        </button>
        <button type="button" class="btn-primary" onclick={publishPhoto} disabled={uploading}>
          {uploading ? 'Posting…' : 'Share'}
        </button>
      </div>
    </div>
  </div>
{/if}

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

  .photo-btn {
    padding: 0;
    border: none;
    cursor: pointer;
  }

  .photo-btn:disabled {
    cursor: default;
  }

  .photo-btn:not(:disabled):hover img {
    opacity: 0.9;
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

  .caption-backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    background: rgba(0, 0, 0, 0.45);
  }

  .caption-modal {
    width: 100%;
    max-width: 400px;
    padding: 1.25rem;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--surface);
  }

  .caption-modal h2 {
    margin: 0 0 0.35rem;
    font-size: 1.05rem;
  }

  .caption-hint {
    margin: 0 0 0.85rem;
    font-size: 0.85rem;
    color: var(--muted);
  }

  .caption-modal textarea {
    width: 100%;
    padding: 0.65rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text);
    resize: vertical;
    font: inherit;
    margin-bottom: 0.85rem;
  }

  .caption-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .btn-secondary,
  .btn-primary {
    padding: 0.5rem 1rem;
    border-radius: 8px;
    font: inherit;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-secondary {
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text);
  }

  .btn-primary {
    border: none;
    background: var(--accent);
    color: #fff;
  }

  .btn-primary:disabled,
  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
