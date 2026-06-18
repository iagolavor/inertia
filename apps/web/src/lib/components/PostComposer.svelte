<script lang="ts">
  import { api } from '$lib/api';
  import { prepareImageForUpload } from '$lib/image';

  interface Props {
    disabled?: boolean;
    onposted?: () => void;
  }

  let { disabled = false, onposted }: Props = $props();

  let body = $state('');
  let mediaPreview = $state<string | null>(null);
  let mediaBase64 = $state<string | null>(null);
  let posting = $state(false);
  let error = $state('');
  let fileInput = $state<HTMLInputElement | null>(null);

  const canSend = $derived(!posting && !disabled && (body.trim().length > 0 || mediaBase64 !== null));

  function onFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    error = '';
    const reader = new FileReader();
    reader.onload = async () => {
      try {
        mediaPreview = reader.result as string;
        mediaBase64 = await prepareImageForUpload(file);
      } catch (e) {
        error = e instanceof Error ? e.message : 'Falha ao processar imagem';
        clearMedia();
      }
    };
    reader.onerror = () => {
      error = 'Falha ao ler imagem';
    };
    reader.readAsDataURL(file);
    input.value = '';
  }

  function clearMedia() {
    mediaPreview = null;
    mediaBase64 = null;
  }

  function openFilePicker() {
    if (posting || disabled) return;
    fileInput?.click();
  }

  async function publish() {
    if (!canSend) return;

    const text = body.trim();
    posting = true;
    error = '';

    try {
      if (mediaBase64) {
        await api.uploadProfilePhoto(mediaBase64, text || undefined);
      } else {
        await api.createPost(text);
      }
      body = '';
      clearMedia();
      onposted?.();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Falha ao publicar';
    } finally {
      posting = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      void publish();
    }
  }
</script>

<div class="composer" class:disabled>
  <div class="composer-box">
    <textarea
      bind:value={body}
      placeholder="Write here"
      rows="2"
      disabled={posting || disabled}
      onkeydown={onKeydown}
    ></textarea>

    {#if mediaPreview}
      <div class="preview-wrap">
        <img class="preview" src={mediaPreview} alt="Pré-visualização" />
        <button
          type="button"
          class="remove-media"
          onclick={clearMedia}
          disabled={posting}
          aria-label="Remover foto"
        >
          ×
        </button>
      </div>
    {/if}

    <div class="composer-footer">
      <button
        type="button"
        class="attach-btn"
        onclick={openFilePicker}
        disabled={posting || disabled}
        aria-label="Adicionar foto"
        title="Adicionar foto"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
            fill="none"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>

      <input
        bind:this={fileInput}
        type="file"
        accept="image/jpeg,image/png,image/gif,image/webp,image/*"
        class="file-input"
        disabled={posting || disabled}
        onchange={onFileSelect}
      />

      <button
        type="button"
        class="send-btn"
        onclick={publish}
        disabled={!canSend}
        aria-label="Publicar"
        title="Publicar (Ctrl+Enter)"
      >
        {#if posting}
          <span class="spinner" aria-hidden="true"></span>
        {:else}
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M5 12h14M13 5l7 7-7 7"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        {/if}
      </button>
    </div>
  </div>

  {#if disabled && !posting}
    <p class="hint">Liga o API bridge para publicar.</p>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .composer {
    width: 100%;
  }

  .composer-box {
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--bg);
    overflow: hidden;
    transition: border-color 0.15s;
  }

  .composer:not(.disabled) .composer-box:focus-within {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
  }

  .composer textarea {
    display: block;
    width: 100%;
    min-height: 3.25rem;
    max-height: 12rem;
    padding: 0.85rem 1rem 0.5rem;
    border: none;
    background: transparent;
    color: var(--text);
    resize: vertical;
    line-height: 1.45;
  }

  .composer textarea:focus {
    outline: none;
  }

  .composer textarea::placeholder {
    color: var(--muted);
  }

  .preview-wrap {
    position: relative;
    margin: 0 1rem 0.5rem;
    display: inline-block;
    max-width: calc(100% - 2rem);
  }

  .preview {
    display: block;
    max-width: 100%;
    max-height: 180px;
    border-radius: 8px;
    border: 1px solid var(--border);
  }

  .remove-media {
    position: absolute;
    top: 0.35rem;
    right: 0.35rem;
    width: 1.6rem;
    height: 1.6rem;
    border: none;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.65);
    color: #fff;
    font-size: 1rem;
    line-height: 1;
    cursor: pointer;
  }

  .composer-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.35rem 0.5rem 0.5rem 0.35rem;
    border-top: 1px solid var(--border);
    background: color-mix(in srgb, var(--surface) 65%, var(--bg));
  }

  .attach-btn,
  .send-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    background: transparent;
    cursor: pointer;
    border-radius: 8px;
    color: var(--muted);
  }

  .attach-btn {
    width: 2.25rem;
    height: 2.25rem;
  }

  .attach-btn svg {
    width: 1.25rem;
    height: 1.25rem;
  }

  .attach-btn:hover:not(:disabled) {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }

  .send-btn {
    width: 2.35rem;
    height: 2.35rem;
    color: var(--accent);
  }

  .send-btn svg {
    width: 1.15rem;
    height: 1.15rem;
  }

  .send-btn:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }

  .send-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 24%, transparent);
  }

  .send-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .attach-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .file-input {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .spinner {
    width: 1rem;
    height: 1rem;
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

  .hint {
    margin: 0.45rem 0 0;
    font-size: 0.8rem;
    color: var(--muted);
  }

  .error {
    margin: 0.45rem 0 0;
    font-size: 0.8rem;
  }
</style>
