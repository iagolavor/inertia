<script lang="ts">
  import { isDuplicateRelay } from '$lib/relay-multiaddr';

  let {
    relays = $bindable<string[]>([]),
    disabled = false,
    inputId = 'relay-multiaddr',
    addError = $bindable('')
  }: {
    relays?: string[];
    disabled?: boolean;
    inputId?: string;
    addError?: string;
  } = $props();

  let draft = $state('');

  function addRelay() {
    const value = draft.trim();
    addError = '';
    if (!value) {
      addError = 'Paste a full relay multiaddr first.';
      return;
    }
    if (isDuplicateRelay(value, relays)) {
      addError = 'That relay is already in the list.';
      return;
    }
    relays = [...relays, value];
    draft = '';
  }

  function removeRelay(index: number) {
    relays = relays.filter((_, i) => i !== index);
    addError = '';
  }

  function onInputKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault();
      addRelay();
    }
  }
</script>

<div class="relay-list-editor">
  <div class="relay-add-row">
    <input
      id={inputId}
      type="text"
      class="relay-input"
      placeholder="/ip4/203.0.113.10/tcp/9000/p2p/12D3KooW…"
      bind:value={draft}
      {disabled}
      onkeydown={onInputKeydown}
    />
    <button
      type="button"
      class="btn btn-secondary btn-compact"
      {disabled}
      onclick={addRelay}
    >
      Add
    </button>
  </div>

  {#if relays.length > 0}
    <ul class="relay-list" aria-label="Configured relay addresses">
      {#each relays as relay, index (relay)}
        <li class="relay-row">
          <div class="relay-row-main">
            {#if index === 0}
              <span class="relay-badge">Primary</span>
            {/if}
            <code class="relay-value">{relay}</code>
          </div>
          <button
            type="button"
            class="relay-remove"
            {disabled}
            aria-label="Remove relay"
            onclick={() => removeRelay(index)}
          >
            Remove
          </button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="relay-empty muted">No relays yet. Add one from your VPS relay logs.</p>
  {/if}

  {#if addError}
    <p class="relay-add-error">{addError}</p>
  {/if}
</div>

<style>
  .relay-list-editor {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .relay-add-row {
    display: flex;
    gap: 0.5rem;
    align-items: stretch;
  }

  .relay-input {
    flex: 1;
    min-width: 0;
    padding: 0.55rem 0.65rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 0.875rem;
  }

  .relay-input:disabled {
    opacity: 0.65;
  }

  .btn-compact {
    padding: 0.4rem 0.75rem;
    font-size: 0.8125rem;
    font-weight: 500;
    white-space: nowrap;
  }

  .relay-list {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
  }

  .relay-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.55rem 0.65rem;
    background: var(--bg);
    border-top: 1px solid var(--border);
  }

  .relay-row:first-child {
    border-top: none;
  }

  .relay-row-main {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 0;
    flex: 1;
  }

  .relay-badge {
    align-self: flex-start;
    font-size: 0.6875rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }

  .relay-value {
    font-family: monospace;
    font-size: 0.8125rem;
    word-break: break-all;
    line-height: 1.4;
  }

  .relay-remove {
    flex-shrink: 0;
    padding: 0.2rem 0;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.8125rem;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .relay-remove:hover:not(:disabled) {
    color: var(--danger);
  }

  .relay-remove:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .relay-empty {
    margin: 0;
    font-size: 0.8125rem;
    line-height: 1.45;
  }

  .relay-add-error {
    margin: 0;
    font-size: 0.8125rem;
    color: var(--danger);
  }

  .muted {
    color: var(--muted);
  }
</style>
