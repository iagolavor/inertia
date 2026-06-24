<script lang="ts">
  import { api } from '$lib/api';
  import { prepareImageForUpload } from '$lib/image';
  import {
    assertVideoUploadAllowed,
    isVideoUploadFile,
    prepareVideoForUpload,
    formatVideoDuration,
    processingLabel,
    type PreparedVideo,
    type VideoPrepareStage
  } from '$lib/video';
  import {
    applyInlineFormat,
    prefixSelectedLines,
    type InlineFormat
  } from '$lib/textFormat';

  interface Props {
    disabled?: boolean;
    onposted?: (result: {
      content_id: string;
      body: string;
      local_media_preview?: string;
      media_kind?: 'photo' | 'video';
      media_ready?: boolean;
    }) => void;
  }

  let { disabled = false, onposted }: Props = $props();

  let body = $state('');
  let mediaPreview = $state<string | null>(null);
  let mediaBase64 = $state<string | null>(null);
  let pendingVideo = $state<PreparedVideo | null>(null);
  let mediaKind = $state<'photo' | 'video' | null>(null);
  let processingStage = $state<VideoPrepareStage | null>(null);
  let pendingDurationMs = $state<number | null>(null);
  let posting = $state(false);
  let error = $state('');
  let fileInput = $state<HTMLInputElement | null>(null);
  let textareaEl = $state<HTMLTextAreaElement | null>(null);

  const canSend = $derived(
    !posting &&
      !processingStage &&
      !disabled &&
      (body.trim().length > 0 || mediaBase64 !== null || pendingVideo !== null)
  );
  const hasDraft = $derived(
    body.trim().length > 0 ||
      mediaPreview !== null ||
      pendingVideo !== null ||
      posting ||
      processingStage !== null
  );

  const videoDurationLabel = $derived(
    pendingVideo
      ? formatVideoDuration(pendingVideo.durationMs)
      : pendingDurationMs
        ? formatVideoDuration(pendingDurationMs)
        : null
  );

  const processingMedia = $derived(processingStage !== null);
  const previewBusy = $derived(processingMedia || (posting && mediaKind === 'video'));
  const previewStatus = $derived(
    processingStage
      ? processingLabel(processingStage)
      : posting && mediaKind === 'video'
        ? 'Uploading video…'
        : null
  );

  let composerFocused = $state(false);
  const toolbarVisible = $derived(composerFocused || hasDraft);

  function onComposerFocusIn() {
    composerFocused = true;
  }

  function onComposerFocusOut(e: FocusEvent) {
    const root = e.currentTarget as HTMLElement;
    if (!root.contains(e.relatedTarget as Node | null)) {
      composerFocused = false;
    }
  }

  function applyFormat(format: InlineFormat) {
    if (!textareaEl || posting || disabled) return;
    body = applyInlineFormat(textareaEl, format);
  }

  function applyBulletList() {
    if (!textareaEl || posting || disabled) return;
    body = prefixSelectedLines(textareaEl, '- ');
  }

  async function onFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;

    error = '';
    clearMedia();
    try {
      if (isVideoUploadFile(file)) {
        assertVideoUploadAllowed(file);
        processingStage = 'loading';
        const prepared = await prepareVideoForUpload(file, (progress) => {
          processingStage = progress.stage;
          if (progress.previewUrl) {
            mediaPreview = progress.previewUrl;
            mediaKind = 'video';
          }
          if (progress.durationMs) {
            pendingDurationMs = progress.durationMs;
          }
        });
        pendingVideo = prepared;
        mediaKind = 'video';
        mediaPreview = prepared.previewUrl;
        pendingDurationMs = prepared.durationMs;
      } else {
        const reader = new FileReader();
        const dataUrl = await new Promise<string>((resolve, reject) => {
          reader.onload = () => resolve(reader.result as string);
          reader.onerror = () => reject(new Error('Failed to read image'));
          reader.readAsDataURL(file);
        });
        mediaPreview = dataUrl;
        mediaBase64 = await prepareImageForUpload(file);
        mediaKind = 'photo';
      }
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to process media';
      clearMedia();
    } finally {
      processingStage = null;
    }
  }

  function clearMedia() {
    mediaPreview = null;
    mediaBase64 = null;
    pendingVideo = null;
    pendingDurationMs = null;
    mediaKind = null;
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
      let content_id: string;
      if (pendingVideo) {
        ({ content_id } = await api.createVideoPost(
          text,
          pendingVideo.videoBase64,
          pendingVideo.thumbBase64,
          pendingVideo.durationMs
        ));
      } else if (mediaBase64) {
        ({ content_id } = await api.createPost(text, mediaBase64));
      } else {
        ({ content_id } = await api.createPost(text));
      }
      body = '';
      const preview = mediaPreview;
      const kind = mediaKind;
      clearMedia();
      onposted?.({
        content_id,
        body: text,
        local_media_preview: preview ?? undefined,
        media_kind: kind ?? undefined,
        media_ready: kind === 'video' ? true : undefined
      });
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
      return;
    }

    if (!(e.ctrlKey || e.metaKey) || posting || disabled) return;

    const key = e.key.toLowerCase();
    if (key === 'b' && !e.shiftKey) {
      e.preventDefault();
      applyFormat('bold');
    } else if (key === 'i' && !e.shiftKey) {
      e.preventDefault();
      applyFormat('italic');
    } else if (key === 'k' && !e.shiftKey) {
      e.preventDefault();
      applyFormat('link');
    } else if (key === 'e' && e.shiftKey) {
      e.preventDefault();
      applyFormat('code');
    } else if (key === 'x' && e.shiftKey) {
      e.preventDefault();
      applyFormat('strike');
    }
  }
</script>

<div class="composer" class:disabled>
  <div
    class="composer-box"
    class:has-draft={hasDraft}
    onfocusin={onComposerFocusIn}
    onfocusout={onComposerFocusOut}
  >
    <textarea
      bind:this={textareaEl}
      bind:value={body}
      placeholder="Share something…"
      rows="1"
      disabled={posting || disabled}
      onkeydown={onKeydown}
    ></textarea>

    {#if processingMedia && !mediaPreview}
      <div class="preview-wrap preview-placeholder" aria-busy="true" aria-label="Processing video">
        <div class="preview-overlay">
          <span class="overlay-spinner" aria-hidden="true"></span>
          <span class="overlay-label">Processing video…</span>
        </div>
      </div>
    {:else if mediaPreview}
      <div class="preview-wrap">
        <img class="preview" src={mediaPreview} alt="Video preview" />
        {#if mediaKind === 'video'}
          <span class="video-badge" aria-hidden="true">
            {#if videoDurationLabel}{videoDurationLabel}{:else}Video{/if}
          </span>
        {/if}
        {#if previewBusy && previewStatus}
          <div class="preview-overlay" aria-live="polite">
            <span class="overlay-spinner" aria-hidden="true"></span>
            <span class="overlay-label">{previewStatus}</span>
          </div>
        {/if}
        <button
          type="button"
          class="remove-media"
          onclick={clearMedia}
          disabled={posting || processingMedia}
          aria-label="Remove media"
        >
          ×
        </button>
      </div>
    {/if}

    <div class="composer-toolbar" aria-hidden={!toolbarVisible}>
      <div class="toolbar-start" role="toolbar" aria-label="Text formatting">
        <button
          type="button"
          class="format-btn"
          disabled={posting || disabled}
          title="Bold (Ctrl+B)"
          aria-label="Bold"
          onclick={() => applyFormat('bold')}
        >
          <strong>B</strong>
        </button>
        <button
          type="button"
          class="format-btn"
          disabled={posting || disabled}
          title="Italic (Ctrl+I)"
          aria-label="Italic"
          onclick={() => applyFormat('italic')}
        >
          <em>I</em>
        </button>
        <button
          type="button"
          class="format-btn"
          disabled={posting || disabled}
          title="Strikethrough (Ctrl+Shift+X)"
          aria-label="Strikethrough"
          onclick={() => applyFormat('strike')}
        >
          <s>S</s>
        </button>
        <button
          type="button"
          class="format-btn format-btn-mono"
          disabled={posting || disabled}
          title="Code (Ctrl+Shift+E)"
          aria-label="Code"
          onclick={() => applyFormat('code')}
        >
          &lt;/&gt;
        </button>
        <button
          type="button"
          class="format-btn"
          disabled={posting || disabled}
          title="Link (Ctrl+K)"
          aria-label="Link"
          onclick={() => applyFormat('link')}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M10 13a5 5 0 007.54.54l2.5-2.5a5 5 0 00-7.07-7.07l-1.72 1.71"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
            />
            <path
              d="M14 11a5 5 0 00-7.54-.54l-2.5 2.5a5 5 0 007.07 7.07l1.71-1.71"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
            />
          </svg>
        </button>
        <span class="format-sep" aria-hidden="true"></span>
        <button
          type="button"
          class="format-btn"
          disabled={posting || disabled}
          title="Bullet list"
          aria-label="Bullet list"
          onclick={applyBulletList}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="5" cy="7" r="1.5" fill="currentColor" />
            <circle cx="5" cy="12" r="1.5" fill="currentColor" />
            <circle cx="5" cy="17" r="1.5" fill="currentColor" />
            <path
              d="M9 7h11M9 12h11M9 17h11"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
            />
          </svg>
        </button>
        <span class="format-sep" aria-hidden="true"></span>
        <button
          type="button"
          class="format-btn attach-btn"
          onclick={openFilePicker}
          disabled={posting || disabled}
          aria-label="Add photo or video"
          title="Add photo or video"
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
          accept="image/jpeg,image/png,image/gif,image/webp,image/*,video/mp4,video/webm,video/quicktime,video/*"
          class="file-input"
          disabled={posting || disabled}
          onchange={onFileSelect}
        />
      </div>

      <button
        type="button"
        class="send-btn"
        onclick={publish}
        disabled={!canSend}
        aria-label="Publish"
        title="Publish (Ctrl+Enter)"
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
    border: var(--compose-border, 1px solid var(--border));
    border-radius: var(--radius-md, 8px);
    background: var(--compose-bg, var(--surface));
    overflow: hidden;
    transition: border-color 0.15s;
  }

  .composer:not(.disabled) .composer-box:focus-within {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
  }

  .composer textarea {
    display: block;
    width: 100%;
    min-height: 2.35rem;
    max-height: 12rem;
    padding: 0.45rem 0.75rem;
    border: none;
    border-radius: 0;
    background: transparent;
    color: var(--text);
    resize: vertical;
    line-height: 1.4;
  }

  .composer textarea:focus {
    outline: none;
  }

  .composer textarea::placeholder {
    color: var(--muted);
  }

  .composer-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
    max-height: 0;
    opacity: 0;
    overflow: hidden;
    padding: 0 0.65rem;
    border-top: 0 solid transparent;
    pointer-events: none;
    transition:
      max-height 0.2s ease,
      opacity 0.15s ease,
      padding 0.15s ease,
      border-color 0.15s ease;
  }

  .composer-box:focus-within .composer-toolbar,
  .composer-box.has-draft .composer-toolbar {
    max-height: 2.5rem;
    opacity: 1;
    padding: 0.3rem 0.65rem 0.45rem;
    border-top-color: var(--border);
    border-top-width: 1px;
    pointer-events: auto;
  }

  .toolbar-start {
    display: flex;
    align-items: center;
    gap: 0.1rem;
    flex: 1;
    min-width: 0;
    flex-wrap: nowrap;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .toolbar-start::-webkit-scrollbar {
    display: none;
  }

  .format-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 1.35rem;
    height: 1.35rem;
    padding: 0 0.2rem;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--muted);
    font-size: 0.8rem;
    line-height: 1;
    cursor: pointer;
  }

  .format-btn strong,
  .format-btn em,
  .format-btn s {
    font: inherit;
    color: inherit;
  }

  .format-btn-mono {
    font-family: ui-monospace, 'Cascadia Code', monospace;
    font-size: 0.72rem;
    font-weight: 600;
  }

  .format-btn svg {
    width: 0.9rem;
    height: 0.9rem;
  }

  .format-btn:hover:not(:disabled) {
    color: var(--text);
    background: color-mix(in srgb, var(--border) 35%, transparent);
  }

  .format-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .format-sep {
    width: 1px;
    height: 0.85rem;
    margin: 0 0.1rem;
    background: var(--border);
  }

  .preview-wrap {
    position: relative;
    margin: 0 0.75rem 0.5rem;
    display: inline-block;
    max-width: calc(100% - 1.5rem);
  }

  .preview-placeholder {
    width: min(100%, 280px);
    aspect-ratio: 16 / 9;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: color-mix(in srgb, var(--border) 35%, var(--bg));
  }

  .preview-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.45rem;
    background: rgba(0, 0, 0, 0.55);
    color: #fff;
    border-radius: inherit;
  }

  .overlay-spinner {
    width: 1.35rem;
    height: 1.35rem;
    border: 2px solid rgba(255, 255, 255, 0.35);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  .overlay-label {
    font-size: 0.75rem;
    font-weight: 600;
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

  .video-badge {
    position: absolute;
    left: 0.45rem;
    bottom: 0.45rem;
    padding: 0.15rem 0.45rem;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.65);
    color: #fff;
    font-size: 0.65rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    font-variant-numeric: tabular-nums;
  }

  .attach-btn:hover:not(:disabled) {
    color: var(--accent);
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }

  .send-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    border: none;
    background: transparent;
    cursor: pointer;
    border-radius: 6px;
    color: var(--accent);
    width: 1.75rem;
    height: 1.75rem;
    padding: 0;
  }

  .send-btn svg {
    width: 1rem;
    height: 1rem;
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

  .format-btn:disabled,
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
