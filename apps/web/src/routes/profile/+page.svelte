<script lang="ts">
  import { onMount } from 'svelte';
  import { api, blobUrl, type ProfilePhoto } from '$lib/api';
  import PhotoGrid from '$lib/components/PhotoGrid.svelte';
  import ProfileHeader from '$lib/components/ProfileHeader.svelte';
  import { identityState, refreshIdentity, setIdentity, startP2pInBackground } from '$lib/identity.svelte';

  let displayName = $state('');
  let saving = $state(false);
  let error = $state('');
  let photos = $state<ProfilePhoto[]>([]);

  async function loadPhotos() {
    if (!identityState.apiOnline || !identityState.identity) return;
    try {
      photos = await api.listProfilePhotos();
    } catch {
      // non-blocking
    }
  }

  onMount(() => {
    void loadPhotos();
  });

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

  const avatarUrl = $derived(photos.length > 0 ? blobUrl(photos[0].blob_hash) : null);
</script>

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
  <h1 class="page-title">Perfil</h1>

  <div class="card profile-card">
    <ProfileHeader
      displayName={identityState.identity.display_name}
      seed={identityState.identity.signing_pubkey}
      {avatarUrl}
      size={96}
      online={identityState.apiOnline}
      statusLoading={identityState.loading}
    >
      <p class="badge-local">Guardado neste dispositivo</p>
    </ProfileHeader>
  </div>

  <PhotoGrid
    {photos}
    disabled={!identityState.apiOnline}
    onuploaded={loadPhotos}
  />

  {#if error}<p class="error">{error}</p>{/if}
{/if}

<style>
  .page-title {
    margin: 0 0 1rem;
    font-size: 1.35rem;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .muted {
    color: var(--muted);
    margin: 0;
  }

  .badge-local {
    display: inline-block;
    margin: 0.5rem 0 0;
    padding: 0.2rem 0.55rem;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--success);
    background: var(--badge-success-bg);
  }

  .cmd {
    background: var(--bg);
    padding: 1rem;
    border-radius: 8px;
    overflow-x: auto;
  }

  .profile-card {
    margin-bottom: 1rem;
  }
</style>
