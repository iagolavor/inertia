<script lang="ts">
  import { api, type FeedBackup } from '$lib/api';

  interface Props {
    onchanged?: () => void;
  }

  let { onchanged }: Props = $props();

  let enabled = $state(false);
  let loading = $state(true);
  let saving = $state(false);
  let restoring = $state(false);
  let message = $state('');
  let error = $state('');
  let fileInput = $state<HTMLInputElement | null>(null);

  async function loadSettings() {
    loading = true;
    error = '';
    try {
      const settings = await api.getSettings();
      enabled = settings.feed_history_enabled;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load settings';
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void loadSettings();
  });

  async function toggleHistory() {
    saving = true;
    error = '';
    message = '';
    try {
      const settings = await api.setFeedHistoryEnabled(!enabled);
      enabled = settings.feed_history_enabled;
      message = enabled
        ? 'History enabled — new posts are saved locally.'
        : 'Ephemeral mode — only posts from the last 7 days.';
      onchanged?.();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to save';
    } finally {
      saving = false;
    }
  }

  async function exportBackup() {
    error = '';
    message = '';
    try {
      const backup = await api.exportFeedBackup();
      const blob = new Blob([JSON.stringify(backup, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = `inertia-feed-${new Date().toISOString().slice(0, 10)}.json`;
      anchor.click();
      URL.revokeObjectURL(url);
      message = `Backup exported (${backup.items.length} posts).`;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to export';
    }
  }

  function openRestorePicker() {
    fileInput?.click();
  }

  async function onRestoreFile(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = '';
    if (!file) return;

    restoring = true;
    error = '';
    message = '';
    try {
      const text = await file.text();
      const backup = JSON.parse(text) as FeedBackup;
      const report = await api.restoreFeedBackup(backup);
      enabled = true;
      message = `Backup restored — ${report.items_imported} posts, ${report.blobs_imported} new photos.`;
      onchanged?.();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to restore backup';
    } finally {
      restoring = false;
    }
  }
</script>

<div class="history-panel">
  <p class="muted">
    By default the feed is ephemeral (7 days). You can accumulate posts locally and back up to
    continue on another device.
  </p>

  {#if loading}
    <p class="muted">Loading…</p>
  {:else}
    <label class="toggle-row">
      <input
        type="checkbox"
        checked={enabled}
        disabled={saving || restoring}
        onchange={toggleHistory}
      />
      <span>Keep feed history on this device</span>
    </label>

    <div class="actions">
      <button class="btn btn-secondary" type="button" onclick={exportBackup} disabled={restoring}>
        Export backup
      </button>
      <button
        class="btn btn-secondary"
        type="button"
        onclick={openRestorePicker}
        disabled={restoring}
      >
        {restoring ? 'Restoring…' : 'Restore backup'}
      </button>
      <input
        bind:this={fileInput}
        type="file"
        accept="application/json,.json"
        class="file-input"
        onchange={onRestoreFile}
      />
    </div>
  {/if}

  {#if message}
    <p class="success">{message}</p>
  {/if}
  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .muted {
    color: var(--muted);
    font-size: 0.875rem;
    margin: 0 0 0.85rem;
    line-height: 1.45;
  }

  .toggle-row {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    font-size: 0.9rem;
    cursor: pointer;
    margin-bottom: 0.85rem;
  }

  .toggle-row input {
    margin-top: 0.2rem;
  }

  .actions {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.5rem;
  }

  .actions .btn {
    justify-content: center;
    width: 100%;
    padding-inline: 0.75rem;
    font-size: 0.875rem;
  }

  @media (max-width: 420px) {
    .actions {
      grid-template-columns: 1fr;
    }
  }

  .file-input {
    display: none;
  }

  .success {
    margin: 0.65rem 0 0;
    font-size: 0.8rem;
    color: var(--success);
  }

  .error {
    margin: 0.65rem 0 0;
    font-size: 0.8rem;
  }
</style>
