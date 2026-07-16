<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { api, type Contact, type InvitePreview } from '$lib/api';
  import { ApiRequestError } from '$lib/api-errors';
  import ProfileHeader from '$lib/components/ProfileHeader.svelte';
  import { normalizeInviteInput } from '$lib/invite-input';
  import { identityState } from '$lib/identity.svelte';

  let inviteInput = $state('');
  let preview = $state<InvitePreview | null>(null);
  let accepted = $state<Contact | null>(null);
  let inviteLoading = $state(false);
  let accepting = $state(false);
  let inviteError = $state('');

  const relayOk = $derived(identityState.p2pStatus?.relay_connected === true);
  const relayConfigured = $derived(identityState.p2pStatus?.relay_configured === true);
  const relayWarning = $derived(relayConfigured && !relayOk);

  function readInviteFromUrl(): string {
    const params = page.url.searchParams;
    const d = params.get('d');
    const hash = page.url.hash.slice(1);
    if (d) return normalizeInviteInput(decodeURIComponent(d));
    if (hash) return normalizeInviteInput(hash);
    return '';
  }

  $effect(() => {
    page.url.hash;
    page.url.search;
    if (!identityState.identity || identityState.loading) return;
    const fromUrl = readInviteFromUrl();
    if (!fromUrl) return;
    inviteInput = fromUrl;
    if (identityState.apiOnline && !preview && !accepted) {
      void loadPreview();
    }
  });

  async function loadPreview() {
    if (!inviteInput.trim()) {
      inviteError = 'Paste an invite link or code first';
      return;
    }
    inviteLoading = true;
    inviteError = '';
    preview = null;
    accepted = null;
    try {
      preview = await api.previewInvite(normalizeInviteInput(inviteInput));
    } catch (e) {
      inviteError =
        e instanceof ApiRequestError
          ? e.message
          : e instanceof Error
            ? e.message
            : 'Invalid or expired invite';
    } finally {
      inviteLoading = false;
    }
  }

  async function acceptInvite() {
    if (!inviteInput.trim()) return;
    accepting = true;
    inviteError = '';
    try {
      accepted = await api.acceptInvite(normalizeInviteInput(inviteInput));
      preview = null;
    } catch (e) {
      inviteError =
        e instanceof ApiRequestError
          ? e.message
          : e instanceof Error
            ? e.message
            : 'Failed to accept invite';
    } finally {
      accepting = false;
    }
  }
</script>

{#if accepted}
  <h1>You're connected</h1>
  <p class="subtitle"><a href="/connections">← Back to Connections</a></p>

  <div class="card action-card">
    <ProfileHeader displayName={accepted.display_name} seed={accepted.signing_pubkey} size={56}>
      <p class="muted lead">
        {accepted.display_name} is saved on this device. No central server was involved.
      </p>
    </ProfileHeader>
    <div class="btn-row">
      <a class="btn" href="/friends/{accepted.id}">Open chat</a>
      <a class="btn btn-secondary" href="/connections">Connections</a>
      <a class="btn btn-secondary" href="/messages">Messages</a>
    </div>
  </div>
{:else}
  <h1>Accept invite</h1>
  <p class="subtitle">
    <a href="/connections">← Back to Connections</a> · Paste a single-use invite to add a connection.
    Links expire in 15 minutes.
  </p>

  <div class="card action-card">
    <h3 class="section-title">Invite link or code</h3>
    <p class="muted lead">
      Your friend should stay online while you accept. Confirm the safety code before accepting.
    </p>

    <div class="field">
      <label for="invite-input">Paste invite</label>
      <textarea
        id="invite-input"
        bind:value={inviteInput}
        rows="4"
        placeholder="Paste invite here…"
        disabled={!identityState.apiOnline || inviteLoading || accepting}
      ></textarea>
    </div>

    {#if inviteError}<p class="error">{inviteError}</p>{/if}

    <button
      type="button"
      class="btn btn-secondary"
      onclick={() => void loadPreview()}
      disabled={!identityState.apiOnline || inviteLoading || accepting}
    >
      {inviteLoading ? 'Checking…' : 'Preview'}
    </button>

    {#if preview}
      <div class="preview-card">
        <ProfileHeader displayName={preview.display_name} seed={preview.signing_pubkey} size={56}>
          <p class="muted lead">Confirm this safety code matches what they told you:</p>
          <p class="safety-code">{preview.safety_code}</p>
          <p class="muted meta">Expires {new Date(preview.expires_at).toLocaleString()}</p>
          <p class="muted meta">
            This invite includes their relay. It will be configured on your device when you accept.
          </p>
          {#if relayWarning}
            <p class="relay-warn">
              Relay not connected yet. Wait a moment or try Accept anyway (can take up to a minute).
            </p>
          {/if}
          <button
            type="button"
            class="btn accept-btn"
            onclick={() => void acceptInvite()}
            disabled={accepting || !identityState.apiOnline}
          >
            {accepting ? 'Connecting via relay (up to 2 min)…' : 'Accept invite'}
          </button>
        </ProfileHeader>
      </div>
    {/if}
  </div>
{/if}

<style>
  .action-card {
    padding: 0.75rem 1.25rem 1.1rem;
  }

  .action-card :global(.btn) {
    padding: 0.35rem 0.75rem;
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
    border-radius: 6px;
    gap: 0.35rem;
  }

  .lead {
    margin: 0 0 0.85rem;
    font-size: var(--font-size-md);
    line-height: 1.45;
  }

  .btn-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 1rem;
  }

  .preview-card {
    margin-top: 1.1rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border);
  }

  .safety-code {
    margin: 0.35rem 0 0.65rem;
    font-family: monospace;
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-bold);
    letter-spacing: 0.04em;
  }

  .meta {
    margin: 0.25rem 0 0;
    font-size: var(--font-size-sm);
  }

  .relay-warn {
    margin: 0.65rem 0 0;
    font-size: var(--font-size-sm);
    color: var(--warning);
  }

  .accept-btn {
    margin-top: 0.85rem;
  }

  .field {
    margin-bottom: 0.85rem;
  }

  .field label {
    display: block;
    margin-bottom: 0.35rem;
    font-size: var(--font-size-sm);
    color: var(--muted);
  }

  .field textarea {
    width: 100%;
    box-sizing: border-box;
    padding: 0.55rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--input-bg, var(--bg));
    color: var(--text);
    font: inherit;
    font-size: var(--font-size-md);
    resize: vertical;
  }
</style>
