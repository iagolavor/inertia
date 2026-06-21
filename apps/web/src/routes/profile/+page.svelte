<script lang="ts">

  import { onMount } from 'svelte';

  import { api, type FeedItem, type ProfilePhoto } from '$lib/api';

  import Avatar from '$lib/components/Avatar.svelte';

  import PhotoGrid from '$lib/components/PhotoGrid.svelte';

  import ProfileEditModal from '$lib/components/ProfileEditModal.svelte';

  import { identityState, refreshIdentity, setIdentity, startP2pInBackground } from '$lib/identity.svelte';

  import {

    buildProfileBlobPreviews,
    readCachedProfile,

    resolveCachedBlobUrl,

    writeCachedProfile

  } from '$lib/local-cache';

  import { prepareImageForUpload } from '$lib/image';



  let displayName = $state('');

  let saving = $state(false);

  let error = $state('');

  let photos = $state<ProfilePhoto[]>([]);

  let friendCount = $state(0);

  let blobPreviews = $state<Record<string, string>>({});

  let editOpen = $state(false);

  let editSaving = $state(false);

  let editUploading = $state(false);

  let editError = $state('');

  let selectedContentId = $state<string | null>(null);

  let selectedPost = $state<FeedItem | null>(null);

  let selectedPostLoading = $state(false);

  let photoGrid = $state<{ openPhotoPicker: () => void } | null>(null);



  function photoUrl(hash: string) {

    return resolveCachedBlobUrl(hash, blobPreviews, identityState.apiOnline);

  }



  async function hydrateFromCache() {

    const cached = await readCachedProfile();

    if (!cached) return false;

    photos = cached.photos;

    friendCount = cached.friend_count;

    blobPreviews = cached.blob_previews;
    return true;

  }



  async function persistProfileCache(nextPhotos: ProfilePhoto[], nextFriendCount: number) {

    const previews =

      identityState.apiOnline && nextPhotos.length > 0

        ? { ...blobPreviews, ...(await buildProfileBlobPreviews(nextPhotos)) }

        : blobPreviews;

    blobPreviews = previews;
    await writeCachedProfile({

      photos: nextPhotos,

      friend_count: nextFriendCount,

      blob_previews: previews

    });

  }



  async function loadProfile() {
    await hydrateFromCache();
    if (!identityState.identity || !identityState.apiOnline) return;

    try {
      const [nextPhotos, contacts] = await Promise.all([
        api.listProfilePhotos(),
        api.listContacts()
      ]);
      photos = nextPhotos;
      friendCount = contacts.length;
      await persistProfileCache(nextPhotos, friendCount);
    } catch {
      await hydrateFromCache();
    }
  }

  async function reloadPhotos() {
    if (!identityState.apiOnline) return;
    try {
      const nextPhotos = await api.listProfilePhotos();
      photos = nextPhotos;
      await persistProfileCache(nextPhotos, friendCount);
    } catch {
      await hydrateFromCache();
    }
  }

  onMount(() => {
    void loadProfile();
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



  const avatarUrl = $derived(
    identityState.identity?.avatar_blob_hash
      ? photoUrl(identityState.identity.avatar_blob_hash)
      : null
  );

  const bio = $derived(identityState.identity?.bio ?? '');

  const handle = $derived(

    identityState.identity

      ? `@${identityState.identity.signing_pubkey.slice(0, 8).toLowerCase()}`

      : ''

  );



  function openEdit() {

    if (!identityState.apiOnline) return;

    editError = '';

    editOpen = true;

  }



  async function saveProfile(name: string, nextBio: string) {

    if (!identityState.identity || !identityState.apiOnline) return;

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



  async function uploadAvatar(file: File) {

    if (!identityState.apiOnline) return;

    editUploading = true;

    editError = '';

    try {

      const dataBase64 = await prepareImageForUpload(file);

      const identity = await api.uploadAvatar(dataBase64);

      await setIdentity(identity);

    } catch (e) {

      editError = e instanceof Error ? e.message : 'Failed to upload profile photo';

    } finally {

      editUploading = false;

    }

  }



  async function selectPost(contentId: string | null) {
    selectedContentId = contentId;
    if (!contentId) {
      selectedPost = null;
      selectedPostLoading = false;
      return;
    }

    if (!identityState.apiOnline) return;

    selectedPostLoading = true;
    selectedPost = null;
    error = '';

    try {
      selectedPost = await api.getPost(contentId);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to open post';
      selectedContentId = null;
    } finally {
      selectedPostLoading = false;
    }
  }



  async function onCommentAdded() {
    if (selectedPost && identityState.apiOnline) {
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

{:else if identityState.identity}

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

        {#if identityState.apiOnline}

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

        {/if}

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

    {#if identityState.apiOnline}
      <button
        type="button"
        class="btn-add-photo"
        onclick={() => photoGrid?.openPhotoPicker()}
      >
        Add photo
      </button>
    {/if}

  </div>



  <PhotoGrid
    bind:this={photoGrid}
    {photos}
    photoUrl={photoUrl}
    disabled={!identityState.apiOnline}
    authorId={identityState.identity.signing_pubkey}
    authorName={identityState.identity.display_name}
    selectedContentId={selectedContentId}
    {selectedPost}
    selectedPostLoading={selectedPostLoading}
    onuploaded={reloadPhotos}
    onselect={selectPost}
    oncomment={onCommentAdded}
  />



  {#if identityState.apiOnline}

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

      onphoto={uploadAvatar}

    />

  {/if}



  {#if error}<p class="error">{error}</p>{/if}

{:else if !identityState.apiOnline}

  <p class="empty muted">Connect the API to create your profile — use the banner above.</p>

{:else}

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

{/if}



<style>

  .muted {

    color: var(--muted);

    margin: 0;

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

    align-items: center;

    justify-content: space-between;

    gap: 0.75rem;

    border-bottom: 1px solid var(--border);

    margin-bottom: 1rem;

    padding-bottom: 0.5rem;

  }



  .btn-add-photo {

    padding: 0.35rem 0.75rem;

    border: 1px solid var(--border);

    border-radius: 8px;

    background: var(--surface);

    color: var(--text);

    font: inherit;

    font-size: 0.78rem;

    font-weight: 600;

    cursor: pointer;

    white-space: nowrap;

    flex-shrink: 0;

  }



  .btn-add-photo:hover {

    background: color-mix(in srgb, var(--border) 25%, var(--surface));

  }



  .grid-tab {

    display: inline-flex;

    align-items: center;

    justify-content: flex-start;

    gap: 0.4rem;

    padding: 0.65rem 0;

    font-size: 0.72rem;

    font-weight: 700;

    letter-spacing: 0.06em;

    text-transform: uppercase;

    color: var(--muted);

    border-bottom: 2px solid transparent;

    margin-bottom: -1px;

  }



  .grid-tab svg {

    width: 0.95rem;

    height: 0.95rem;

  }



  .grid-tab.active {

    color: var(--text);

    border-bottom-color: var(--text);

  }

</style>


