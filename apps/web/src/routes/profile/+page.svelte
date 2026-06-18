<script lang="ts">
  import { api } from '$lib/api';
  import ProfileHeader from '$lib/components/ProfileHeader.svelte';
  import { identityState, refreshIdentity, setIdentity, startP2pInBackground } from '$lib/identity.svelte';

  let displayName = $state('');
  let saving = $state(false);
  let error = $state('');

  async function createIdentity() {
    if (!displayName.trim()) {
      error = 'Display name is required';
      return;
    }

    if (identityState.identity) {
      error = 'A profile already exists on this device';
      return;
    }

    saving = true;
    error = '';

    try {
      const identity = await api.initIdentity(displayName.trim());
      await setIdentity(identity);
      void startP2pInBackground();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to create identity';
      await refreshIdentity();
    } finally {
      saving = false;
    }
  }
</script>

<h1>Profile</h1>
<p class="subtitle">One profile per device, stored in your local database.</p>

{#if identityState.loading}
  <p class="empty">Loading...</p>
{:else if !identityState.apiOnline}
  <div class="card">
    <h2>API offline</h2>
    <p>Start the Rust API bridge before using the app:</p>
    <pre class="cmd">cargo run -p inertia-api</pre>
    <button class="btn btn-secondary" onclick={() => refreshIdentity()}>Retry</button>
  </div>
{:else if !identityState.identity}
  <div class="card">
    <h2>Create your profile</h2>
    <p class="muted">
      Your cryptographic identity is generated on this device and saved locally. Each install gets
      exactly one profile — it cannot be replaced without resetting local data.
    </p>
    <div class="field">
      <label for="name">Display name</label>
      <input id="name" bind:value={displayName} placeholder="Your name" />
    </div>
    {#if error}<p class="error">{error}</p>{/if}
    <button class="btn" onclick={createIdentity} disabled={saving}>
      {saving ? 'Creating...' : 'Create identity'}
    </button>
  </div>
{:else}
  <div class="card">
    <ProfileHeader
      displayName={identityState.identity.display_name}
      seed={identityState.identity.signing_pubkey}
      size={96}
    >
      <p class="badge-local">Stored on this device</p>
      <p class="muted detail">
        Signing key:
        <strong class="mono wrap">{identityState.identity.signing_pubkey}</strong>
      </p>
      <p class="muted detail">
        Encryption key:
        <strong class="mono wrap">{identityState.identity.encryption_pubkey}</strong>
      </p>
      <p class="muted detail">
        Safety code:
        <strong class="mono">{identityState.identity.signing_pubkey.slice(0, 8)}</strong>
      </p>
      {#if identityState.p2pInfo?.peer_id}
        <p class="muted detail peer-id">Peer ID: {identityState.p2pInfo.peer_id}</p>
      {:else}
        <p class="muted detail">P2P starting in background…</p>
      {/if}
      <p class="links">
        <a href="/friends">Invite a friend</a> ·
        <a href="/messages">Messages</a> ·
        <a href="/outbox">Outbox</a>
      </p>
    </ProfileHeader>
  </div>
{/if}

<style>
  .muted {
    color: var(--muted);
    margin: 0;
  }

  .detail {
    font-size: 0.875rem;
    margin-top: 0.35rem;
  }

  .peer-id {
    word-break: break-all;
  }

  .mono {
    font-family: monospace;
  }

  .wrap {
    word-break: break-all;
    font-size: 0.75rem;
    font-weight: 400;
  }

  .links {
    margin: 1rem 0 0;
  }

  .badge-local {
    display: inline-block;
    margin: 0.5rem 0 0;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--success);
    background: #1b4332;
  }

  .cmd {
    background: var(--bg);
    padding: 1rem;
    border-radius: 8px;
    overflow-x: auto;
  }
</style>
