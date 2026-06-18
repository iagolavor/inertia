const API_BASE = '/api';
const REQUEST_TIMEOUT_MS = 8_000;

export interface Identity {
  signing_pubkey: string;
  encryption_pubkey: string;
  phone_hash: string | null;
  display_name: string;
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
}

export interface ProfilePhoto {
  id: string;
  blob_hash: string;
  caption: string | null;
  sort_order: number;
  created_at: string;
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

async function fetchWithTimeout(path: string, init?: RequestInit): Promise<Response> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);

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

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetchWithTimeout(path, init);
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    const message = err.error ?? 'Request failed';
    if (res.status === 409) {
      throw new Error('A profile already exists on this device');
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
  retryOutbox: (content_id: string, recipient_id: string) =>
    request<void>('/outbox/retry', {
      method: 'POST',
      body: JSON.stringify({ content_id, recipient_id })
    }),
  listFeed: () => request<FeedItem[]>('/feed'),
  createPost: (body: string, media_base64?: string) =>
    request<{ content_id: string }>('/posts', {
      method: 'POST',
      body: JSON.stringify({ body, media_base64: media_base64 ?? null })
    }),
  listProfilePhotos: () => request<ProfilePhoto[]>('/profile/photos'),
  uploadProfilePhoto: (data_base64: string, caption?: string) =>
    request<ProfilePhoto>('/profile/photos', {
      method: 'POST',
      body: JSON.stringify({ data_base64, caption: caption ?? null })
    }),
  shutdownBridge: () => request<void>('/shutdown', { method: 'POST' })
};
