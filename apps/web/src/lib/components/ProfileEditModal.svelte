<script lang="ts">
  import Avatar from '$lib/components/Avatar.svelte';
  import { prepareImageForUpload } from '$lib/image';

  interface Props {
    open: boolean;
    displayName: string;
    bio: string;
    avatarUrl?: string | null;
    seed: string;
    saving?: boolean;
    uploadingPhoto?: boolean;
    error?: string;
    onclose?: () => void;
    onsave?: (displayName: string, bio: string) => void;
    onphoto?: (file: File) => void;
  }

  let {
    open,
    displayName,
    bio,
    avatarUrl = null,
    seed,
    saving = false,
    uploadingPhoto = false,
    error = '',
    onclose,
    onsave,
    onphoto
  }: Props = $props();

  let nameInput = $state('');
  let bioInput = $state('');
  let fileInput = $state<HTMLInputElement | null>(null);

  const bioLimit = 150;

  $effect(() => {
    if (open) {
      nameInput = displayName;
      bioInput = bio;
    }
  });

  function close() {
    onclose?.();
  }

  function save() {
    onsave?.(nameInput.trim(), bioInput.trim());
  }

  function onBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) close();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) close();
  }

  function pickPhoto() {
    if (saving || uploadingPhoto) return;
    fileInput?.click();
  }

  async function onFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;
    onphoto?.(file);
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="backdrop" role="presentation" onclick={onBackdropClick}>
    <div class="modal" role="dialog" aria-modal="true" aria-labelledby="edit-profile-title">
      <header class="modal-header">
        <button type="button" class="text-btn" onclick={close} disabled={saving}>Cancel</button>
        <h2 id="edit-profile-title">Edit profile</h2>
        <button
          type="button"
          class="text-btn text-btn-accent"
          onclick={save}
          disabled={saving || !nameInput.trim()}
        >
          {saving ? 'Saving…' : 'Done'}
        </button>
      </header>

      <div class="modal-body">
        <div class="avatar-edit">
          <Avatar {seed} alt={nameInput} src={avatarUrl} size={88} />
          <button
            type="button"
            class="change-photo-btn"
            onclick={pickPhoto}
            disabled={saving || uploadingPhoto}
          >
            {uploadingPhoto ? 'Uploading…' : 'Change profile photo'}
          </button>
          <input
            bind:this={fileInput}
            type="file"
            accept="image/jpeg,image/png,image/gif,image/webp,image/*"
            class="file-input"
            onchange={onFileSelect}
          />
        </div>

        <div class="field">
          <label for="edit-name">Name</label>
          <input id="edit-name" bind:value={nameInput} maxlength={64} disabled={saving} />
        </div>

        <div class="field">
          <label for="edit-bio">Bio</label>
          <textarea
            id="edit-bio"
            bind:value={bioInput}
            rows="4"
            maxlength={bioLimit}
            disabled={saving}
            placeholder="Write something about yourself"
          ></textarea>
          <span class="char-count">{bioInput.length}/{bioLimit}</span>
        </div>

        {#if error}
          <p class="error">{error}</p>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding: 0;
    background: rgba(0, 0, 0, 0.45);
  }

  .modal {
    width: 100%;
    max-width: 480px;
    max-height: 92vh;
    overflow: auto;
    border-radius: 16px 16px 0 0;
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: 0 -8px 32px rgba(0, 0, 0, 0.18);
  }

  @media (min-width: 520px) {
    .backdrop {
      align-items: center;
      padding: 1rem;
    }

    .modal {
      border-radius: 14px;
    }
  }

  .modal-header {
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    gap: 0.5rem;
    padding: 0.85rem 1rem;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    text-align: center;
  }

  .text-btn {
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    padding: 0.25rem 0;
    justify-self: start;
  }

  .text-btn:last-child {
    justify-self: end;
  }

  .text-btn-accent {
    color: var(--accent);
  }

  .text-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .modal-body {
    padding: 1.25rem 1rem 1.5rem;
  }

  .avatar-edit {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.85rem;
    margin-bottom: 1.25rem;
  }

  .change-photo-btn {
    border: none;
    background: none;
    color: var(--accent);
    font: inherit;
    font-size: 0.875rem;
    font-weight: 600;
    cursor: pointer;
    padding: 0;
  }

  .change-photo-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .file-input {
    display: none;
  }

  .field {
    margin-bottom: 1rem;
  }

  .field label {
    display: block;
    margin-bottom: 0.35rem;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--muted);
  }

  .field input,
  .field textarea {
    width: 100%;
    padding: 0.65rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text);
    resize: vertical;
  }

  .char-count {
    display: block;
    margin-top: 0.35rem;
    font-size: 0.75rem;
    color: var(--muted);
    text-align: right;
  }

  .error {
    margin: 0;
    font-size: 0.85rem;
  }
</style>
