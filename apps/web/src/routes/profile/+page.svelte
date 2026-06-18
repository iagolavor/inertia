<script lang="ts">
  import { onMount } from 'svelte';
  import { api, blobUrl, type FeedItem, type ProfilePhoto } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import PhotoGrid from '$lib/components/PhotoGrid.svelte';
  import ProfileEditModal from '$lib/components/ProfileEditModal.svelte';
  import PostDetailModal from '$lib/components/PostDetailModal.svelte';
  import { identityState, refreshIdentity, setIdentity, startP2pInBackground } from '$lib/identity.svelte';
  import { prepareImageForUpload } from '$lib/image';

  let displayName = $state('');
  let saving = $state(false);
  let error = $state('');
  let photos = $state<ProfilePhoto[]>([]);
  let friendCount = $state(0);
  let editOpen = $state(false);
  let editSaving = $state(false);
  let editUploading = $state(false);
  let editError = $state('');
  let selectedPost = $state<FeedItem | null>(null);
  let detailOpen = $state(false);

  async function loadPhotos() {
    if (!identityState.apiOnline || !identityState.identity) return;
    try {
      photos = await api.listProfilePhotos();
    } catch {
      // non-blocking
    }
  }

  async function loadFriendCount() {
    if (!identityState.apiOnline) return;
    try {
      const contacts = await api.listContacts();
      friendCount = contacts.length;
    } catch {
      friendCount = 0;
    }
  }

  onMount(() => {
    void loadPhotos();
    void loadFriendCount();
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
  const bio = $derived(identityState.identity?.bio ?? '');
  const handle = $derived(
    identityState.identity
      ? `@${identityState.identity.signing_pubkey.slice(0, 8).toLowerCase()}`
      : ''
  );

  function openEdit() {
    editError = '';
    editOpen = true;
  }

  async function saveProfile(name: string, nextBio: string) {
    if (!identityState.identity) return;
    editSaving = true;
    editError = '';
    try {
      const identity = await api.updateProfile(name, nextBio);
      await setIdentity(identity);
      editOpen = false;
    } catch (e) {
      editError = e instanceof Error ? e.message : 'Failed to save profile';
    } finally {
      editSaving = false;
    }
  }

  async function uploadProfilePhoto(file: File) {
    editUploading = true;
    editError = '';
    try {
      const dataBase64 = await prepareImageForUpload(file);
      await api.uploadProfilePhoto(dataBase64);
      await loadPhotos();
    } catch (e) {
      editError = e instanceof Error ? e.message : 'Failed to upload photo';
    } finally {
      editUploading = false;
    }
  }

  async function openPostById(contentId: string) {
    try {
      selectedPost = await api.getPost(contentId);
      detailOpen = true;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to open post';
    }
  }

  async function onCommentAdded() {
    if (selectedPost) {
      try {
        selectedPost = await api.getPost(selectedPost.content_id);
      } catch {
        // ignore refresh errors
      }
    }
  }
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
  <section class="profile-header">
    <div class="avatar-wrap">
      <Avatar
        seed={identityState.identity.signing_pubkey}
        alt={identityState.identity.display_name}
        src={avatarUrl}
        size={72}
      />
    </div>

    <div class="profile-info">
      <div class="name-row">
        <h1 class="profile-name">{identityState.identity.display_name}</h1>
        <button type="button" class="btn-edit-plain" onclick={openEdit}>
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M4 20h4l10.5-10.5a2.1 2.1 0 0 0 0-3L16.5 4.5a2.1 2.1 0 0 0-3 0L3 15v5z"
              fill="none"
              stroke="currentColor"
              stroke-width="1.75"
              stroke-linejoin="round"
            />
            <path d="M13.5 6.5l4 4" fill="none" stroke="currentColor" stroke-width="1.75" />
          </svg>
          <span>Edit</span>
        </button>
      </div>
      <p class="profile-handle">{handle}</p>
      <p class="profile-meta">{photos.length} posts · {friendCount} connections</p>
      {#if bio}
        <p class="profile-bio">{bio}</p>
      {/if}
    </div>
  </section>

  <div class="grid-tabs">
    <div class="grid-tab active" aria-current="page">
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <rect x="3" y="3" width="7" height="7" rx="1" fill="none" stroke="currentColor" stroke-width="1.75" />
        <rect x="14" y="3" width="7" height="7" rx="1" fill="none" stroke="currentColor" stroke-width="1.75" />
        <rect x="3" y="14" width="7" height="7" rx="1" fill="none" stroke="currentColor" stroke-width="1.75" />
        <rect x="14" y="14" width="7" height="7" rx="1" fill="none" stroke="currentColor" stroke-width="1.75" />
      </svg>
      <span>Posts</span>
    </div>
  </div>

  <PhotoGrid
    {photos}
    disabled={!identityState.apiOnline}
    onuploaded={loadPhotos}
    onopenpost={openPostById}
  />

  <ProfileEditModal
    open={editOpen}
    displayName={identityState.identity.display_name}
    {bio}
    {avatarUrl}
    seed={identityState.identity.signing_pubkey}
    saving={editSaving}
    uploadingPhoto={editUploading}
    error={editError}
    onclose={() => (editOpen = false)}
    onsave={saveProfile}
    onphoto={uploadProfilePhoto}
  />

  <PostDetailModal
    open={detailOpen}
    post={selectedPost}
    disabled={!identityState.apiOnline}
    onclose={() => (detailOpen = false)}
    oncomment={onCommentAdded}
  />

  {#if error}<p class="error">{error}</p>{/if}
{/if}

<style>
  .muted {
    color: var(--muted);
    margin: 0;
  }

  .cmd {
    background: var(--bg);
    padding: 1rem;
    border-radius: 8px;
    overflow-x: auto;
  }

  .profile-header {
    display: flex;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .avatar-wrap {
    flex-shrink: 0;
    line-height: 0;
  }

  .profile-info {
    flex: 1;
    min-width: 0;
  }

  .name-row {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    flex-wrap: wrap;
  }

  .profile-name {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 700;
    line-height: 1.35;
  }

  .btn-edit-plain {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
  }

  .btn-edit-plain svg {
    width: 0.9rem;
    height: 0.9rem;
  }

  .btn-edit-plain:hover {
    color: var(--text);
  }

  .profile-handle {
    margin: 0.15rem 0 0;
    font-size: 0.85rem;
    color: var(--muted);
  }

  .profile-meta {
    margin: 0.2rem 0 0;
    font-size: 0.8rem;
    color: var(--muted);
  }

  .profile-bio {
    margin: 0.45rem 0 0;
    font-size: 0.9rem;
    line-height: 1.45;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .grid-tabs {
    display: flex;
    border-top: 1px solid var(--border);
    margin-bottom: 0;
  }

  .grid-tab {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.65rem 0.5rem;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
    border-top: 2px solid transparent;
    margin-top: -1px;
  }

  .grid-tab svg {
    width: 0.95rem;
    height: 0.95rem;
  }

  .grid-tab.active {
    color: var(--text);
    border-top-color: var(--text);
  }
</style>
