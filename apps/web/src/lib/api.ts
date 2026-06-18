const API_BASE = '/api';
const REQUEST_TIMEOUT_MS = 8_000;
const UPLOAD_TIMEOUT_MS = 60_000;

export interface Identity {
  signing_pubkey: string;
  encryption_pubkey: string;
  phone_hash: string | null;
  display_name: string;
  bio?: string;
}

export interface Contact {
  id: string;
  phone_hash: string | null;
  display_name: string;
  peer_id: string | null;
  signing_pubkey: string;
  encryption_pubkey: string;
  last_seen: string | null;
  connection_state: 'online' | 'offline' | 'unreachable';
  multiaddrs?: string[];
}

export interface P2pStatus {
  running: boolean;
  peer_id: string | null;
  listen_addresses: string[];
  connected_peer_ids: string[];
}

export interface InviteResponse {
  link: string;
  payload: string;
  safety_code: string;
  expires_at: string;
  display_name: string;
}

export interface InvitePreview {
  display_name: string;
  signing_pubkey: string;
  safety_code: string;
  expires_at: string;
  peer_id: string | null;
  multiaddrs: string[];
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

export interface FeedItem {
  content_id: string;
  author_id: string;
  author_name: string;
  body: string;
  media_ref: string | null;
  created_at: string;
  expires_at: string;
  is_own: boolean;
  is_archived: boolean;
  comment_count?: number;
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

export interface PublishPhotoResult {
  photo: ProfilePhoto;
  content_id: string;
}

export function blobUrl(hash: string): string {
  return `${API_BASE}/blobs/${hash}`;
}

export interface OutboxEntry {
  content_id: string;
  recipient_id: string;
  status: 'pending' | 'failed' | 'delivered' | 'expired';
  expires_at: string;
  retry_count: number;
  ciphertext: number[];
  content_type: 'message' | 'post';
}

async function fetchWithTimeout(
  path: string,
  init?: RequestInit,
  timeoutMs = REQUEST_TIMEOUT_MS
): Promise<Response> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), timeoutMs);

  try {
    return await fetch(`${API_BASE}${path}`, {
      ...init,
      signal: controller.signal,
      headers: { 'Content-Type': 'application/json', ...init?.headers }
    });
  } catch (error) {
    if (error instanceof DOMException && error.name === 'AbortError') {
      throw new Error('API request timed out — is inertia-api running?');
    }
    throw error;
  } finally {
    clearTimeout(timeout);
  }
}

async function request<T>(
  path: string,
  init?: RequestInit,
  timeoutMs = REQUEST_TIMEOUT_MS
): Promise<T> {
  const res = await fetchWithTimeout(path, init, timeoutMs);
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    const message = err.error ?? res.statusText ?? 'Request failed';
    if (res.status === 409) {
      throw new Error('A profile already exists on this device');
    }
    if (res.status === 413) {
      throw new Error('Imagem demasiado grande para o servidor');
    }
    throw new Error(message);
  }
  if (res.status === 204) return undefined as T;
  return res.json();
}

export const api = {
  health: async () => {
    const res = await fetchWithTimeout('/health');
    if (!res.ok) throw new Error('API offline');
    return res.text();
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
  createInvite: (web_origin?: string) =>
    request<InviteResponse>('/invite', {
      method: 'POST',
      body: JSON.stringify({ web_origin: web_origin ?? window.location.origin })
    }),
  previewInvite: (invite: string) =>
    request<InvitePreview>('/invite/preview', {
      method: 'POST',
      body: JSON.stringify({ invite })
    }),
  acceptInvite: (invite: string) =>
    request<Contact>('/invite/accept', {
      method: 'POST',
      body: JSON.stringify({ invite })
    }),
  listContacts: () => request<Contact[]>('/contacts'),
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
  setFeedHistoryEnabled: (feed_history_enabled: boolean) =>
    request<AppSettings>('/settings', {
      method: 'PATCH',
      body: JSON.stringify({ feed_history_enabled })
    }),
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
  shutdownBridge: () => request<void>('/shutdown', { method: 'POST' })
};
