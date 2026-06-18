<script lang="ts">
  import { api } from '$lib/api';

  interface Props {
    onposted?: () => void;
  }

  let { onposted }: Props = $props();

  let body = $state('');
  let mediaPreview = $state<string | null>(null);
  let mediaBase64 = $state<string | null>(null);
  let posting = $state(false);
  let error = $state('');

  function onFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    if (!file.type.startsWith('image/')) {
      error = 'Apenas imagens são suportadas';
      return;
    }

    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result as string;
      mediaPreview = result;
      mediaBase64 = result.split(',')[1] ?? null;
      error = '';
    };
    reader.readAsDataURL(file);
  }

  function clearMedia() {
    mediaPreview = null;
    mediaBase64 = null;
  }

  async function publish() {
    if (!body.trim() && !mediaBase64) {
      error = 'Escreve algo ou adiciona uma foto';
      return;
    }

    posting = true;
    error = '';

    try {
      await api.createPost(body.trim(), mediaBase64 ?? undefined);
      body = '';
      clearMedia();
      onposted?.();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Falha ao publicar';
    } finally {
      posting = false;
    }
  }
</script>

<div class="compose">
  <textarea
    bind:value={body}
    placeholder="Partilha algo com os teus amigos…"
    rows="3"
    disabled={posting}
  ></textarea>

  {#if mediaPreview}
    <div class="preview-wrap">
      <img class="preview" src={mediaPreview} alt="Pré-visualização" />
      <button type="button" class="remove-media" onclick={clearMedia} aria-label="Remover foto">×</button>
    </div>
  {/if}

  <div class="compose-actions">
    <label class="photo-btn">
      <input type="file" accept="image/*" onchange={onFileSelect} hidden disabled={posting} />
      Foto
    </label>
    <button class="btn" onclick={publish} disabled={posting}>
      {posting ? 'A publicar…' : 'Publicar'}
    </button>
  </div>

  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .compose textarea {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text);
    resize: vertical;
    min-height: 4rem;
  }

  .preview-wrap {
    position: relative;
    margin-top: 0.75rem;
    display: inline-block;
  }

  .preview {
    max-width: 100%;
    max-height: 200px;
    border-radius: 8px;
    display: block;
  }

  .remove-media {
    position: absolute;
    top: 0.35rem;
    right: 0.35rem;
    width: 1.75rem;
    height: 1.75rem;
    border: none;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.6);
    color: #fff;
    font-size: 1.1rem;
    line-height: 1;
    cursor: pointer;
  }

  .compose-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 0.75rem;
  }

  .photo-btn {
    font-size: 0.875rem;
    color: var(--accent);
    font-weight: 500;
    cursor: pointer;
  }

  .photo-btn:hover {
    text-decoration: underline;
  }
</style>
