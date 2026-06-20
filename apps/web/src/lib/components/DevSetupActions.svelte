<script lang="ts">
  import {
    API_OFFLINE_HINT,
    copyDevCommand,
    DEV_COMMANDS,
    suggestedWebCommand,
    webUiMode
  } from '$lib/dev-commands';
  import { refreshIdentity } from '$lib/identity.svelte';

  interface Props {
    /** Smaller buttons for the header banner. */
    compact?: boolean;
    /** Show the one-line explanation above actions. */
    showHint?: boolean;
    hint?: string;
    /** Include web UI copy actions (hidden when already on Vite dev). */
    showWeb?: boolean;
  }

  let {
    compact = false,
    showHint = false,
    hint = API_OFFLINE_HINT,
    showWeb = true
  }: Props = $props();

  const uiMode = $derived(webUiMode());
  const showWebAction = $derived(showWeb && uiMode === 'other');

  let retrying = $state(false);
  let feedback = $state('');

  function flash(message: string) {
    feedback = message;
    setTimeout(() => {
      feedback = '';
    }, 4500);
  }

  async function retry() {
    if (retrying) return;
    retrying = true;
    try {
      await refreshIdentity({ silent: true });
    } finally {
      retrying = false;
    }
  }

  async function copyApi() {
    if (await copyDevCommand(DEV_COMMANDS.apiRelease)) {
      flash('Copied — run in a terminal at the project root, then Retry.');
    } else {
      flash('Could not copy — run: npm run api:release');
    }
  }

  async function copyApiWindow() {
    if (await copyDevCommand(DEV_COMMANDS.apiWindow)) {
      flash('Copied — opens the API in its own window (Windows).');
    } else {
      flash('Could not copy — run: npm run api:window');
    }
  }

  async function copyWeb() {
    const cmd = suggestedWebCommand(uiMode);
    if (await copyDevCommand(cmd)) {
      flash(
        uiMode === 'preview'
          ? 'Copied — run web:preview if this tab was opened before the build.'
          : 'Copied — builds then serves the static UI (lighter than dev mode).'
      );
    } else {
      flash(`Could not copy — run: ${cmd}`);
    }
  }
</script>

<div class="dev-setup" class:compact>
  {#if showHint && hint}
    <p class="hint">{hint}</p>
  {/if}

  <div class="actions">
    <button type="button" class="action primary" disabled={retrying} onclick={() => void retry()}>
      {retrying ? 'Checking…' : 'Retry'}
    </button>
    <button type="button" class="action" onclick={() => void copyApi()} title={DEV_COMMANDS.apiRelease}>
      Start API
    </button>
    {#if showWebAction}
      <button
        type="button"
        class="action"
        onclick={() => void copyWeb()}
        title={suggestedWebCommand(uiMode)}
      >
        Start web
      </button>
    {/if}
    <button
      type="button"
      class="action subtle"
      onclick={() => void copyApiWindow()}
      title={DEV_COMMANDS.apiWindow}
    >
      API window
    </button>
  </div>

  {#if feedback}
    <p class="feedback" role="status" aria-live="polite">{feedback}</p>
  {/if}
</div>

<style>
  .dev-setup {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }

  .hint {
    margin: 0;
    font-size: 0.8125rem;
    line-height: 1.45;
    color: var(--muted);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
  }

  .action {
    padding: 0.4rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 0.75rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .compact .action {
    padding: 0.35rem 0.6rem;
    font-size: 0.72rem;
  }

  .action:hover:not(:disabled) {
    background: color-mix(in srgb, var(--border) 35%, var(--bg));
  }

  .action.primary {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    background: color-mix(in srgb, var(--accent) 12%, var(--bg));
    color: var(--accent);
  }

  .action.subtle {
    font-weight: 500;
    color: var(--muted);
  }

  .action:disabled {
    opacity: 0.65;
    cursor: wait;
  }

  .feedback {
    margin: 0;
    font-size: 0.75rem;
    line-height: 1.4;
    color: var(--success);
  }
</style>
