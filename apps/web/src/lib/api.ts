import {
	ApiRequestError,
	classifyFetchFailure,
	classifyHttpFailure,
	type ApiErrorInfo
} from '$lib/api-errors';
import { getApiBase } from '$lib/api-base';

export type { ApiErrorInfo };

const REQUEST_TIMEOUT_MS = 8_000;
/** Relay bootstrap + circuit reservation before issuing an invite. */
const CREATE_INVITE_TIMEOUT_MS = 90_000;
/** Relay circuit handshake + redemption can exceed 60s on mobile. */
const ACCEPT_INVITE_TIMEOUT_MS = 180_000;
const UPLOAD_TIMEOUT_MS = 60_000;

export interface Identity {
  signing_pubkey: string;
  encryption_pubkey: string;
  phone_hash: string | null;
  display_name: string;
  bio?: string;
  avatar_blob_hash?: string | null;
}

export interface Contact {
  id: string;
  phone_hash: string | null;
  display_name: string;
  peer_id: string | null;
  signing_pubkey: string;
  encryption_pubkey: string;
  last_seen: string | null;
  connection_state: 'online' | 'reachable' | 'unreachable' | 'offline';
  multiaddrs?: string[];
}

export interface P2pActivityEvent {
  at: string;
  kind: string;
  detail: string;
  content_type?: 'message' | 'post' | 'comment';
}

export interface P2pActivitySnapshot {
  dial_in_progress: boolean;
  last_activity_at: string | null;
  events: P2pActivityEvent[];
}

export interface P2pLayers {
  node: 'off' | 'running';
  relay: 'not_configured' | 'standby' | 'unreachable' | 'connecting' | 'connected';
  friends: 'offline' | 'connecting' | 'online';
  sync: 'idle' | 'sending';
  friends_online_count: number;
  pending_outbox_count: number;
}

export interface P2pLayerLabels {
  headline: string;
  node: string;
  relay: string;
  friends: string;
  sync: string;
}

export interface P2pStatus {
  running: boolean;
  peer_id: string | null;
  listen_addresses: string[];
  connected_peer_ids: string[];
  relay_configured: boolean;
  relay_multiaddrs: string[];
  relays_connected_count: number;
  relay_connected: boolean;
  relay_tcp_reachable: boolean | null;
  pending_outbox_count: number;
  dial_in_progress: boolean;
  last_activity_at: string | null;
  recent_activity: P2pActivityEvent[];
  layers: P2pLayers;
  labels: P2pLayerLabels;
  /** off | error | warn | idle | online */
  tone: string;
}

export interface InviteResponse {
  link: string;
  payload: string;
  safety_code: string;
  expires_at: string;
  display_name: string;
}

export interface InviteReadiness {
  ready: boolean;
  relay_configured: boolean;
  relay_connected: boolean;
  reachable: boolean;
  message: string;
}

export interface InvitePreview {
  display_name: string;
  signing_pubkey: string;
  safety_code: string;
  expires_at: string;
  peer_id: string | null;
  multiaddrs: string[];
  relay_multiaddr: string;
}

export interface InboxEntry {
  content_id: string;
  sender_id: string;
  received_at: string;
  expires_at: string;
  read_at: string | null;
  body: string;
  media_ref: string | null;
  content_type: 'message' | 'post';
}

export interface ConversationMessage {
  content_id: string;
  body: string;
  at: string;
  expires_at: string;
  is_own: boolean;
  delivery_status: 'pending' | 'sent' | 'failed' | 'delivered' | 'expired' | null;
}

export interface FeedItem {
  content_id: string;
  author_id: string;
  author_name: string;
  body: string;
  media_ref: string | null;
  thumb_ref?: string | null;
  media_kind?: 'photo' | 'video' | null;
  media_ready?: boolean;
  created_at: string;
  expires_at: string;
  is_own: boolean;
  is_archived: boolean;
  comment_count?: number;
}

export interface MediaFetchStatus {
  root_hash: string;
  state: 'idle' | 'fetching' | 'complete' | 'failed';
  chunks_done: number;
  chunks_total: number;
  transport: string;
  error?: string | null;
}

export interface PostComment {
  id: string;
  post_id: string;
  author_id: string;
  author_name: string;
  body: string;
  created_at: string;
}

export interface AppSettings {
  feed_history_enabled: boolean;
  p2p_listen_port: number;
  relay_multiaddrs: string[];
  p2p_announce: string | null;
  web_origin: string | null;
}

export interface FeedBackup {
  version: number;
  exported_at: string;
  items: Array<{
    content_id: string;
    author_id: string;
    author_name: string;
    body: string;
    media_ref: string | null;
    created_at: string;
    is_own: boolean;
  }>;
  blobs: Record<string, string>;
}

export interface FeedRestoreReport {
  items_imported: number;
  blobs_imported: number;
}

export interface ProfilePhoto {
  id: string;
  blob_hash: string;
  caption: string | null;
  content_id: string | null;
  sort_order: number;
  created_at: string;
}

export type ProfileItem = ProfilePhoto;

export interface ProfileComment {
  id: string;
  profile_item_id: string;
  author_id: string;
  author_name: string;
  body: string;
  created_at: string;
}

export interface ArchiveFolderSummary {
  id: string;
  name: string;
  entry_count: number;
  created_at: string;
}

export interface ArchiveFolder {
  id: string;
  name: string;
  created_at: string;
}

export interface ArchiveEntry {
  id: string;
  folder_id: string;
  name: string;
  root_hash: string;
  total_bytes: number;
  mime: string;
  created_at: string;
}

export interface ArchiveUploadStatus {
  upload_id: string;
  folder_id: string;
  name: string;
  mime: string;
  total_bytes: number;
  chunk_size: number;
  chunks_done: number;
  chunks_total: number;
  missing: number[];
  completed: boolean;
}

/** Soft UI warning for browser zip CPU/memory (not enforced by API). */
export const ARCHIVE_ZIP_SOFT_WARN_BYTES = 200 * 1024 * 1024;
/** Matches inertia-core CHUNK_SIZE. */
export const ARCHIVE_CHUNK_SIZE = 512 * 1024;

export interface ProfileManifest {
  display_name: string;
  bio: string;
  avatar_blob_hash: string | null;
  signing_pubkey: string;
  items: ProfileItem[];
  archive_folders: ArchiveFolderSummary[];
}

export interface PublishPhotoResult {
  photo: ProfilePhoto;
  content_id: string;
}

export function blobUrl(hash: string): string {
  return `${getApiBase()}/blobs/${hash}`;
}

export interface OutboxEntry {
  content_id: string;
  recipient_id: string;
  status: 'pending' | 'sent' | 'failed' | 'delivered' | 'expired';
  expires_at: string;
  retry_count: number;
  ciphertext: number[];
  content_type: 'message' | 'post';
}

const API_HTML_RESPONSE_HINT =
	'Got the app shell instead of the API — reinstall from Android Studio after npm run android:stage-b, or force-stop and reopen the app.';

async function fetchWithTimeout(
  path: string,
  init?: RequestInit,
  timeoutMs = REQUEST_TIMEOUT_MS
): Promise<Response> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);

  try {
    return await fetch(`${getApiBase()}${path}`, {
      ...init,
      signal: controller.signal,
      headers: { 'Content-Type': 'application/json', ...init?.headers }
    });
  } catch (error) {
    throw new ApiRequestError(classifyFetchFailure(error));
  } finally {
    clearTimeout(timeout);
  }
}

async function readJsonBody<T>(res: Response): Promise<T> {
  const text = await res.text();
  if (text.trimStart().startsWith('<')) {
    throw new ApiRequestError({ kind: 'offline', message: API_HTML_RESPONSE_HINT });
  }
  try {
    return JSON.parse(text) as T;
  } catch {
    throw new ApiRequestError({ kind: 'server', message: 'Invalid JSON from API' });
  }
}

async function request<T>(
  path: string,
  init?: RequestInit,
  timeoutMs = REQUEST_TIMEOUT_MS
): Promise<T> {
  const res = await fetchWithTimeout(path, init, timeoutMs);
  if (!res.ok) {
    const err = await readJsonBody<{ error?: string; code?: string }>(res).catch(() => ({
      error: res.statusText
    }));
    const raw = err.error ?? res.statusText ?? 'Request failed';
    const code = 'code' in err && typeof err.code === 'string' ? err.code : undefined;
    if (res.status === 409) {
      throw new ApiRequestError({ kind: 'client', message: 'A profile already exists on this device' });
    }
    if (res.status === 413) {
      throw new ApiRequestError({ kind: 'client', message: 'Imagem demasiado grande para o servidor' });
    }
    if (res.status === 405) {
      throw new ApiRequestError({
        kind: 'offline',
        message:
          'API route not found (405) — rebuild the app (npm run android:stage-b) or paste only the invite code after # in the link'
      });
    }
    throw new ApiRequestError(classifyHttpFailure(res.status, raw, code));
  }
  if (res.status === 204) return undefined as T;
  return readJsonBody<T>(res);
}

export const api = {
  health: async () => {
    try {
      const res = await fetchWithTimeout('/health');
      if (!res.ok) {
        throw new ApiRequestError(classifyHttpFailure(res.status, res.statusText));
      }
      const body = (await res.text()).trim();
      if (body !== 'ok') {
        throw new ApiRequestError({ kind: 'offline', message: API_HTML_RESPONSE_HINT });
      }
      return body;
    } catch (error) {
      if (error instanceof ApiRequestError) throw error;
      throw new ApiRequestError(classifyFetchFailure(error));
    }
  },
  getIdentity: () => request<Identity>('/identity'),
  initIdentity: (display_name: string) =>
    request<Identity>('/identity', {
      method: 'POST',
      body: JSON.stringify({ display_name })
    }),
  updateProfile: (display_name: string, bio?: string) =>
    request<Identity>('/identity/update', {
      method: 'POST',
      body: JSON.stringify({ display_name, bio: bio ?? '' })
    }),
  uploadAvatar: (data_base64: string) =>
    request<Identity>(
      '/identity/avatar',
      {
        method: 'POST',
        body: JSON.stringify({ data_base64 })
      },
      UPLOAD_TIMEOUT_MS
    ),
  createInvite: () =>
    request<InviteResponse>(
      '/invite',
      {
        method: 'POST',
        body: JSON.stringify({})
      },
      CREATE_INVITE_TIMEOUT_MS
    ),
  inviteReadiness: () => request<InviteReadiness>('/invite/readiness'),
  previewInvite: (invite: string) =>
    request<InvitePreview>('/invite/preview', {
      method: 'POST',
      body: JSON.stringify({ invite })
    }),
  acceptInvite: (invite: string) =>
    request<Contact>(
      '/invite/accept',
      {
        method: 'POST',
        body: JSON.stringify({ invite })
      },
      ACCEPT_INVITE_TIMEOUT_MS
    ),
  listContacts: () => request<Contact[]>('/contacts'),
  getContact: (contactId: string) =>
    request<Contact>(`/contacts/${encodeURIComponent(contactId)}`),
  listConversationMessages: (contactId: string) =>
    request<ConversationMessage[]>(`/contacts/${encodeURIComponent(contactId)}/messages`),
  listInbox: () => request<InboxEntry[]>('/inbox'),
  listOutbox: () => request<OutboxEntry[]>('/outbox'),
  sendMessage: (recipient_id: string, body: string) =>
    request<{ content_id: string }>('/messages', {
      method: 'POST',
      body: JSON.stringify({ recipient_id, body })
    }),
  startP2p: (listen_port?: number) =>
    request<{ peer_id: string; addresses: string[] }>('/p2p/start', {
      method: 'POST',
      body: JSON.stringify({ listen_port })
    }),
  p2pAddresses: () =>
    request<{ peer_id: string | null; addresses: string[] }>('/p2p/addresses'),
  p2pStatus: () => request<P2pStatus>('/p2p/status'),
  p2pActivity: () => request<P2pActivitySnapshot>('/p2p/activity'),
  retryOutbox: (content_id: string, recipient_id: string) =>
    request<void>('/outbox/retry', {
      method: 'POST',
      body: JSON.stringify({ content_id, recipient_id })
    }),
  listFeed: () => request<FeedItem[]>('/feed'),
  getPost: (content_id: string) => request<FeedItem>(`/posts/${content_id}`),
  listPostComments: (post_id: string) =>
    request<PostComment[]>(`/posts/${post_id}/comments`),
  addPostComment: (post_id: string, body: string) =>
    request<PostComment>(`/posts/${post_id}/comments`, {
      method: 'POST',
      body: JSON.stringify({ body })
    }),
  getSettings: () => request<AppSettings>('/settings'),
  updateSettings: (settings: {
    feed_history_enabled?: boolean;
    p2p_listen_port?: number;
    relay_multiaddrs?: string[];
    p2p_announce?: string;
    web_origin?: string;
  }) =>
    request<AppSettings>('/settings', {
      method: 'PATCH',
      body: JSON.stringify(settings)
    }),
  setFeedHistoryEnabled: (feed_history_enabled: boolean) =>
    request<AppSettings>('/settings', {
      method: 'PATCH',
      body: JSON.stringify({ feed_history_enabled })
    }),
  p2pShareAddress: () => request<{ multiaddr: string | null }>('/p2p/share-address'),
  exportFeedBackup: () => request<FeedBackup>('/feed/backup'),
  restoreFeedBackup: (backup: FeedBackup) =>
    request<FeedRestoreReport>(
      '/feed/restore',
      {
        method: 'POST',
        body: JSON.stringify(backup)
      },
      UPLOAD_TIMEOUT_MS
    ),
  createPost: (body: string, media_base64?: string) =>
    request<{ content_id: string }>(
      '/posts',
      {
        method: 'POST',
        body: JSON.stringify({ body, media_base64: media_base64 ?? null })
      },
      UPLOAD_TIMEOUT_MS
    ),
  createVideoPost: (
    body: string,
    video_base64: string,
    thumb_base64: string,
    duration_ms: number
  ) =>
    request<{ content_id: string }>(
      '/posts/video',
      {
        method: 'POST',
        body: JSON.stringify({ body, video_base64, thumb_base64, duration_ms })
      },
      UPLOAD_TIMEOUT_MS
    ),
  startMediaFetch: (root_hash: string, author_contact_id?: string, direct_required = false) => {
    const params = new URLSearchParams();
    if (author_contact_id) params.set('author_contact_id', author_contact_id);
    if (direct_required) params.set('direct_required', 'true');
    const q = params.toString() ? `?${params}` : '';
    return request<MediaFetchStatus>(
      `/media/${encodeURIComponent(root_hash)}/fetch${q}`,
      { method: 'POST' },
      UPLOAD_TIMEOUT_MS
    );
  },
  /** Archive peer download: DCUtR/direct only (never relay bulk). */
  startArchiveFetch: (root_hash: string, author_contact_id: string) =>
    api.startMediaFetch(root_hash, author_contact_id, true),
  mediaFetchStatus: (root_hash: string) =>
    request<MediaFetchStatus>(`/media/${encodeURIComponent(root_hash)}/status`),
  listProfilePhotos: () => request<ProfilePhoto[]>('/profile/photos'),
  uploadProfilePhoto: (data_base64: string, caption?: string) =>
    request<PublishPhotoResult>(
      '/profile/photos',
      {
        method: 'POST',
        body: JSON.stringify({ data_base64, caption: caption ?? null })
      },
      UPLOAD_TIMEOUT_MS
    ),
  fetchFriendProfile: (contactId: string) =>
    request<ProfileManifest>(`/contacts/${encodeURIComponent(contactId)}/profile`),
  listOwnProfileComments: (itemId: string) =>
    request<ProfileComment[]>(`/profile/items/${encodeURIComponent(itemId)}/comments`),
  addOwnProfileComment: (itemId: string, body: string) =>
    request<ProfileComment>(`/profile/items/${encodeURIComponent(itemId)}/comments`, {
      method: 'POST',
      body: JSON.stringify({ body })
    }),
  listFriendProfileComments: (contactId: string, itemId: string) =>
    request<ProfileComment[]>(
      `/contacts/${encodeURIComponent(contactId)}/profile/items/${encodeURIComponent(itemId)}/comments`
    ),
  addFriendProfileComment: (contactId: string, itemId: string, body: string) =>
    request<ProfileComment>(
      `/contacts/${encodeURIComponent(contactId)}/profile/items/${encodeURIComponent(itemId)}/comments`,
      {
        method: 'POST',
        body: JSON.stringify({ body })
      }
    ),
  listArchiveFolders: () => request<ArchiveFolder[]>('/archive/folders'),
  createArchiveFolder: (name: string) =>
    request<ArchiveFolder>('/archive/folders', {
      method: 'POST',
      body: JSON.stringify({ name })
    }),
  deleteArchiveFolder: (folderId: string) =>
    request<void>(`/archive/folders/${encodeURIComponent(folderId)}`, { method: 'DELETE' }),
  listArchiveEntries: (folderId: string) =>
    request<ArchiveEntry[]>(`/archive/folders/${encodeURIComponent(folderId)}/entries`),
  addArchiveEntry: (folderId: string, name: string, data_base64: string, mime?: string) =>
    request<ArchiveEntry>(
      `/archive/folders/${encodeURIComponent(folderId)}/entries`,
      {
        method: 'POST',
        body: JSON.stringify({ name, data_base64, mime: mime ?? null })
      },
      UPLOAD_TIMEOUT_MS
    ),
  beginArchiveUpload: (folderId: string, name: string, total_bytes: number, mime?: string) =>
    request<ArchiveUploadStatus>(`/archive/folders/${encodeURIComponent(folderId)}/uploads`, {
      method: 'POST',
      body: JSON.stringify({ name, total_bytes, mime: mime ?? null })
    }),
  archiveUploadStatus: (uploadId: string) =>
    request<ArchiveUploadStatus>(`/archive/uploads/${encodeURIComponent(uploadId)}`),
  putArchiveUploadChunk: async (
    uploadId: string,
    index: number,
    chunk: ArrayBuffer | Uint8Array,
    chunkHash: string
  ): Promise<ArchiveUploadStatus> => {
    const src = chunk instanceof Uint8Array ? chunk : new Uint8Array(chunk);
    const copy = new Uint8Array(src.byteLength);
    copy.set(src);
    const body = new Blob([copy]);
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), UPLOAD_TIMEOUT_MS);
    try {
      const res = await fetch(
        `${getApiBase()}/archive/uploads/${encodeURIComponent(uploadId)}/chunks/${index}`,
        {
          method: 'PUT',
          body,
          signal: controller.signal,
          headers: {
            'Content-Type': 'application/octet-stream',
            'x-chunk-hash': chunkHash
          }
        }
      );
      if (!res.ok) {
        const err = await readJsonBody<{ error?: string }>(res).catch(() => ({
          error: res.statusText
        }));
        throw new ApiRequestError(
          classifyHttpFailure(res.status, err.error ?? res.statusText ?? 'Chunk upload failed')
        );
      }
      return readJsonBody<ArchiveUploadStatus>(res);
    } catch (error) {
      if (error instanceof ApiRequestError) throw error;
      throw new ApiRequestError(classifyFetchFailure(error));
    } finally {
      clearTimeout(timeout);
    }
  },
  completeArchiveUpload: (uploadId: string) =>
    request<ArchiveEntry>(`/archive/uploads/${encodeURIComponent(uploadId)}/complete`, {
      method: 'POST'
    }, UPLOAD_TIMEOUT_MS),
  deleteArchiveEntry: (entryId: string) =>
    request<void>(`/archive/entries/${encodeURIComponent(entryId)}`, { method: 'DELETE' }),
  fetchFriendArchiveFolder: (contactId: string, folderId: string) =>
    request<{ folder: ArchiveFolderSummary; entries: ArchiveEntry[] }>(
      `/contacts/${encodeURIComponent(contactId)}/archive/folders/${encodeURIComponent(folderId)}`
    ),
  shutdownBridge: () => request<void>('/shutdown', { method: 'POST' })
};

export async function sha256Hex(data: ArrayBuffer | Uint8Array): Promise<string> {
  const buf = data instanceof Uint8Array ? data : new Uint8Array(data);
  // Copy into a fresh ArrayBuffer so TS accepts BufferSource under strict DOM typings.
  const copy = new Uint8Array(buf.byteLength);
  copy.set(buf);
  const digest = await crypto.subtle.digest('SHA-256', copy.buffer);
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

/** Stream a Blob/File into the local chunked archive upload API with resume. */
export async function uploadArchiveBlob(
  folderId: string,
  name: string,
  blob: Blob,
  mime: string | undefined,
  onProgress?: (done: number, total: number, phase: 'upload' | 'complete') => void
): Promise<ArchiveEntry> {
  let status = await api.beginArchiveUpload(folderId, name, blob.size, mime);
  const chunkSize = status.chunk_size || ARCHIVE_CHUNK_SIZE;
  const total = status.chunks_total;

  const uploadMissing = async (missing: number[]) => {
    for (const index of missing) {
      const start = index * chunkSize;
      const end = Math.min(start + chunkSize, blob.size);
      const slice = blob.slice(start, end);
      const buf = new Uint8Array(await slice.arrayBuffer());
      const hash = await sha256Hex(buf);
      status = await api.putArchiveUploadChunk(status.upload_id, index, buf, hash);
      onProgress?.(status.chunks_done, status.chunks_total, 'upload');
    }
  };

  // First pass: all missing; then re-check status for resume gaps.
  let missing =
    status.missing.length > 0
      ? status.missing
      : Array.from({ length: total }, (_, i) => i);
  await uploadMissing(missing);
  status = await api.archiveUploadStatus(status.upload_id);
  if (status.missing.length > 0) {
    await uploadMissing(status.missing);
  }

  onProgress?.(status.chunks_total, status.chunks_total, 'complete');
  return api.completeArchiveUpload(status.upload_id);
}
