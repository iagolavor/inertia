<script lang="ts">
  import { page } from '$app/state';
  import { api, type Contact, type InvitePreview } from '$lib/api';
  import { ApiRequestError } from '$lib/api-errors';
  import { normalizeInviteInput } from '$lib/invite-input';
  import ProfileHeader from '$lib/components/ProfileHeader.svelte';
  import { identityState } from '$lib/identity.svelte';

  let inviteInput = $state('');
  let preview = $state<InvitePreview | null>(null);
  let accepted = $state<Contact | null>(null);
  let loading = $state(false);
  let accepting = $state(false);
  let error = $state('');

  const relayWarning = $derived(
    identityState.p2pStatus?.relay_configured === true &&
      identityState.p2pStatus?.relay_connected !== true
  );

  function readInviteFromUrl() {
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
    const fromUrl = readInviteFromUrl();
    if (!fromUrl) return;
    inviteInput = fromUrl;
    void loadPreview();
  });

  async function loadPreview() {
    if (!inviteInput.trim()) return;
    loading = true;
    error = '';
    preview = null;
    accepted = null;
    try {
      preview = await api.previewInvite(normalizeInviteInput(inviteInput));
    } catch (e) {
      error = e instanceof ApiRequestError ? e.message : e instanceof Error ? e.message : 'Invalid or expired invite';
    } finally {
      loading = false;
    }
  }

  async function accept() {
    if (!inviteInput.trim()) return;
    accepting = true;
    error = '';
    try {
      accepted = await api.acceptInvite(normalizeInviteInput(inviteInput));
      preview = null;
    } catch (e) {
      error = e instanceof ApiRequestError ? e.message : e instanceof Error ? e.message : 'Failed to accept invite';
    } finally {
      accepting = false;
    }
  }
</script>

<h1>Accept invite</h1>
<p class="subtitle">Verify the safety code. The inviter must be online — each link works once.</p>

<div class="card">
  <div class="field">
    <label for="invite">Invite link or code</label>
    <textarea id="invite" bind:value={inviteInput} rows="4" placeholder="Paste invite here..."></textarea>
  </div>
  {#if error}<p class="error">{error}</p>{/if}
  <button class="btn btn-secondary" onclick={loadPreview} disabled={loading}>
    {loading ? 'Checking...' : 'Preview'}
  </button>
</div>

{#if preview}
  <div class="card">
    <ProfileHeader displayName={preview.display_name} seed={preview.signing_pubkey} size={72}>
      <p style="color: var(--muted); margin: 0.35rem 0 0;">
        Confirm this safety code matches what they told you:
      </p>
      <p style="font-family: monospace; font-size: 1.5rem; letter-spacing: 0.1em; margin: 0.75rem 0 0;">
        {preview.safety_code}
      </p>
      <p style="color: var(--muted); font-size: 0.8rem; margin: 0.75rem 0 0;">
        Single-use · expires {new Date(preview.expires_at).toLocaleString()}
      </p>
      <p style="color: var(--muted); font-size: 0.8rem; margin: 0.75rem 0 0;">
        Accepting contacts them directly over P2P. If they are offline, wait and try again before the link expires.
      </p>
      <p style="color: var(--muted); font-size: 0.8rem; margin: 0.75rem 0 0;">
        This invite includes the shared relay network — accepting will configure it on this device if needed.
      </p>
      {#if relayWarning}
        <p class="relay-warn">
          Relay is configured but not connected yet — wait for <strong>Relay OK</strong> in the header, or try Accept
          anyway (this can take up to a minute).
        </p>
      {/if}
      <button
        class="btn"
        style="margin-top: 1rem;"
        onclick={accept}
        disabled={accepting}
      >
        {accepting ? 'Connecting…' : 'Accept'}
      </button>
    </ProfileHeader>
  </div>
{/if}

{#if accepted}
  <div class="card">
    <ProfileHeader displayName={accepted.display_name} seed={accepted.signing_pubkey} size={72}>
      <p style="color: var(--muted); margin: 0.35rem 0 0;">They are saved locally on your device. No server was involved.</p>
      <p style="margin: 1rem 0 0;">
        <a href="/messages">Send a message</a> · <a href="/friends">Back to messages</a>
      </p>
    </ProfileHeader>
  </div>
{/if}

<style>
  .relay-warn {
    color: var(--warning);
    font-size: 0.8rem;
    margin: 0.75rem 0 0;
  }
</style>
