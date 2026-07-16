<script lang="ts">

  import { api, type ArchiveFolder, type FeedItem, type ProfilePhoto } from '$lib/api';

  import Avatar from '$lib/components/Avatar.svelte';

  import FilesPanel from '$lib/components/FilesPanel.svelte';

  import PhotoGrid from '$lib/components/PhotoGrid.svelte';

  import ProfileEditModal from '$lib/components/ProfileEditModal.svelte';

  import { identityState, setIdentity } from '$lib/identity.svelte';

  import {

    buildProfileBlobPreviews,
    readCachedProfile,

    resolveCachedBlobUrl,

    writeCachedProfile

  } from '$lib/local-cache';

  import { prepareImageForUpload } from '$lib/image';

  import { profileItemToFeedItem } from '$lib/profile-photos';



  let error = $state('');

  let photos = $state<ProfilePhoto[]>([]);

  let friendCount = $state(0);

  let blobPreviews = $state<Record<string, string>>({});

  let editOpen = $state(false);

  let editSaving = $state(false);

  let editUploading = $state(false);

  let editError = $state('');

  let selectedItemId = $state<string | null>(null);

  let selectedPost = $state<FeedItem | null>(null);

  let selectedPostLoading = $state(false);

  let photoGrid = $state<{ openPhotoPicker: () => void } | null>(null);

  let archiveFolders = $state<ArchiveFolder[]>([]);

  let profileTab = $state<'posts' | 'files'>('posts');

  let postsDeleteMode = $state(false);
  let postsDeleteIds = $state<Set<string>>(new Set());
  let postsDeleting = $state(false);

  function exitPostsDeleteMode() {
    postsDeleteMode = false;
    postsDeleteIds = new Set();
    postsDeleting = false;
  }

  function enterPostsDeleteMode() {
    if (!identityState.apiOnline || photos.length === 0) return;
    selectItem(null);
    postsDeleteMode = true;
    postsDeleteIds = new Set();
  }

  function togglePostsDeleteId(itemId: string) {
    const next = new Set(postsDeleteIds);
    if (next.has(itemId)) next.delete(itemId);
    else next.add(itemId);
    postsDeleteIds = next;
  }

  async function confirmPostsDelete() {
    if (!postsDeleteMode || postsDeleting || postsDeleteIds.size === 0) return;
    const ids = [...postsDeleteIds];
    postsDeleting = true;
    error = '';
    try {
      for (const id of ids) {
        await api.deleteProfilePhoto(id);
      }
      exitPostsDeleteMode();
      await reloadPhotos();
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to delete posts';
      postsDeleting = false;
    }
  }



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
      await loadArchiveFolders();
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

  // Identity boots async in layout; reload when API + identity become ready.
  $effect(() => {
    if (identityState.loading) return;
    if (!identityState.identity || !identityState.apiOnline) return;
    void loadProfile();
  });

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



  async function selectItem(itemId: string | null) {
    selectedItemId = itemId;
    if (!itemId || !identityState.identity) {
      selectedPost = null;
      selectedPostLoading = false;
      return;
    }

    selectedPostLoading = true;
    const item = photos.find((p) => p.id === itemId);
    selectedPost = item
      ? profileItemToFeedItem(
          item,
          identityState.identity.signing_pubkey,
          identityState.identity.display_name,
          true
        )
      : null;
    selectedPostLoading = false;
  }

  async function onCommentAdded() {
    // Profile comments live on the item; nothing to refresh from feed.
  }

  async function loadArchiveFolders() {
    if (!identityState.apiOnline) return;
    try {
      archiveFolders = await api.listArchiveFolders();
    } catch {
      // best-effort
    }
  }

</script>



{#if identityState.loading || !identityState.identity}

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
    <div class="tab-row">
      <button
        type="button"
        class="grid-tab"
        class:active={profileTab === 'posts'}
        aria-current={profileTab === 'posts' ? 'page' : undefined}
        onclick={() => (profileTab = 'posts')}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <rect x="3" y="3" width="7" height="7" rx="1" fill="none" stroke="currentColor" stroke-width="1.75" />
          <rect x="14" y="3" width="7" height="7" rx="1" fill="none" stroke="currentColor" stroke-width="1.75" />
          <rect x="3" y="14" width="7" height="7" rx="1" fill="none" stroke="currentColor" stroke-width="1.75" />
          <rect x="14" y="14" width="7" height="7" rx="1" fill="none" stroke="currentColor" stroke-width="1.75" />
        </svg>
        <span>Posts</span>
      </button>
      <button
        type="button"
        class="grid-tab"
        class:active={profileTab === 'files'}
        aria-current={profileTab === 'files' ? 'page' : undefined}
        onclick={() => {
          exitPostsDeleteMode();
          profileTab = 'files';
        }}
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M10 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"
            fill="none"
            stroke="currentColor"
            stroke-width="1.75"
          />
        </svg>
        <span>Files</span>
      </button>
    </div>
  </div>

  {#if profileTab === 'posts'}
    <p class="tab-blurb muted">
      Photos stay on your profile, only accessible when you are online(or have the app open). Publishing also sends a 7-day post to friends' feeds.
    </p>
    <div class="tab-panel">
      <header class="panel-chrome">
        <span class="panel-title">Posts</span>
        {#if identityState.apiOnline}
          <div class="panel-tools">
            {#if postsDeleteMode}
              <button
                type="button"
                class="panel-tool danger"
                disabled={postsDeleting || postsDeleteIds.size === 0}
                onclick={() => void confirmPostsDelete()}
              >
                Confirm delete{postsDeleteIds.size > 0 ? ` (${postsDeleteIds.size})` : ''}
              </button>
              <button
                type="button"
                class="panel-tool ghost"
                disabled={postsDeleting}
                onclick={exitPostsDeleteMode}
              >
                Cancel
              </button>
            {:else}
              <button
                type="button"
                class="panel-tool"
                disabled={postsDeleting}
                onclick={() => photoGrid?.openPhotoPicker()}
              >
                Add photo
              </button>
              {#if photos.length > 0}
                <button
                  type="button"
                  class="panel-tool ghost danger"
                  disabled={postsDeleting}
                  onclick={enterPostsDeleteMode}
                >
                  Delete
                </button>
              {/if}
            {/if}
          </div>
        {/if}
      </header>
      <div class="panel-body">
        <PhotoGrid
          bind:this={photoGrid}
          {photos}
          photoUrl={photoUrl}
          disabled={!identityState.apiOnline || postsDeleting}
          authorId={identityState.identity.signing_pubkey}
          authorName={identityState.identity.display_name}
          selectedItemId={postsDeleteMode ? null : selectedItemId}
          {selectedPost}
          selectedPostLoading={selectedPostLoading}
          deleteMode={postsDeleteMode}
          selectedDeleteIds={postsDeleteIds}
          onuploaded={reloadPhotos}
          onselect={selectItem}
          oncomment={onCommentAdded}
          ontoggledelete={togglePostsDeleteId}
        />
      </div>
    </div>
  {:else if identityState.apiOnline}
    <p class="tab-blurb muted">
      Folders stay on your device. Friends can browse and download only when you are both online,
      over a direct (hole-punched) connection. Transfers can resume if interrupted.
    </p>
    <FilesPanel
      mode="owner"
      folders={archiveFolders}
      disabled={!identityState.apiOnline}
      onfolderschange={loadArchiveFolders}
      onerror={(msg) => (error = msg)}
    />
  {:else}
    <p class="tab-blurb muted">Reconnect to manage files.</p>
  {/if}

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

    justify-content: flex-start;

    gap: 0.75rem;

    border-bottom: 1px solid var(--border);

    margin-bottom: 0.75rem;

    padding-bottom: 0.5rem;

  }

  .tab-row {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .tab-blurb {
    margin: 0 0 0.75rem;
    font-size: 0.8rem;
    line-height: 1.4;
    max-width: 40rem;
  }

  .tab-panel {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    overflow: hidden;
  }

  .panel-chrome {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.55rem 0.85rem;
    padding: 0.55rem 0.75rem;
    border-bottom: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg) 55%, var(--surface));
  }

  .panel-title {
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--text);
  }

  .panel-tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
  }

  .panel-tool {
    padding: 0.3rem 0.65rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .panel-tool:hover:not(:disabled) {
    background: color-mix(in srgb, var(--border) 22%, var(--surface));
  }

  .panel-tool:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .panel-tool.ghost {
    background: transparent;
  }

  .panel-tool.danger {
    color: var(--danger);
    border-color: color-mix(in srgb, var(--danger) 35%, var(--border));
  }

  .panel-tool.danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--danger) 12%, var(--surface));
  }

  .panel-body {
    padding: 0.75rem;
  }



  .grid-tab {

    display: inline-flex;

    align-items: center;

    justify-content: flex-start;

    gap: 0.35rem;

    padding: 0.45rem 0;

    /* font shorthand must come before size/weight overrides */
    font: inherit;

    font-size: 0.8rem;

    font-weight: 600;

    letter-spacing: 0;

    text-transform: none;

    color: var(--muted);

    border: none;

    border-bottom: 2px solid transparent;

    margin-bottom: -1px;

    background: transparent;

    cursor: pointer;

  }



  .grid-tab svg {

    width: 0.85rem;

    height: 0.85rem;

  }



  .grid-tab.active {

    color: var(--text);

    border-bottom-color: var(--text);

  }

  .muted {
    color: var(--muted);
    font-size: 0.85rem;
  }

</style>


