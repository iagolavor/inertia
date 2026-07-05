import type { Contact, ConversationMessage, InboxEntry } from '$lib/api';

const REACHABLE_MS = 24 * 60 * 60 * 1000;

/** Overlay live libp2p session state without refetching the full contact roster. */
export function contactWithLivePresence(
  contact: Contact,
  connectedPeerIds: string[] | undefined
): Contact {
  const connected = new Set(connectedPeerIds ?? []);
  let connection_state: Contact['connection_state'];

  if (contact.peer_id && connected.has(contact.peer_id)) {
    connection_state = 'online';
  } else if (!contact.peer_id && !(contact.multiaddrs?.length ?? 0)) {
    connection_state = 'unreachable';
  } else if (
    contact.last_seen != null &&
    Date.now() - new Date(contact.last_seen).getTime() <= REACHABLE_MS
  ) {
    connection_state = 'reachable';
  } else {
    connection_state = 'unreachable';
  }

  return { ...contact, connection_state };
}

export interface DmThread {
  contact: Contact;
  lastMessage: InboxEntry | null;
  /** ISO timestamp used for sorting (most recent activity first). */
  lastActivity: string;
}

export function buildDmThreads(contacts: Contact[], inbox: InboxEntry[]): DmThread[] {
  const messages = inbox.filter((entry) => entry.content_type === 'message');

  const latestBySender = new Map<string, InboxEntry>();
  for (const msg of messages) {
    const existing = latestBySender.get(msg.sender_id);
    if (!existing || new Date(msg.received_at) > new Date(existing.received_at)) {
      latestBySender.set(msg.sender_id, msg);
    }
  }

  const threads: DmThread[] = contacts.map((contact) => {
    const lastMessage =
      latestBySender.get(contact.id) ?? latestBySender.get(contact.signing_pubkey) ?? null;
    return {
      contact,
      lastMessage,
      lastActivity: lastMessage?.received_at ?? contact.last_seen ?? '1970-01-01T00:00:00.000Z'
    };
  });

  threads.sort((a, b) => {
    const aTime = new Date(a.lastActivity).getTime();
    const bTime = new Date(b.lastActivity).getTime();
    if (bTime !== aTime) return bTime - aTime;
    return a.contact.display_name.localeCompare(b.contact.display_name);
  });

  return threads;
}

export function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return 'now';
  if (mins < 60) return `${mins}m`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d`;
  return new Date(iso).toLocaleDateString();
}

export function previewText(body: string, max = 72): string {
  const flat = body.replace(/\s+/g, ' ').trim();
  if (flat.length <= max) return flat;
  return `${flat.slice(0, max - 1)}…`;
}

export type PresenceTier = 'connected' | 'reachable';

/** API overlay state, with client fallback when the API still returns legacy `offline`. */
export function effectiveConnectionState(
  contact: Pick<Contact, 'connection_state' | 'last_seen' | 'peer_id' | 'multiaddrs'>
): Contact['connection_state'] {
  const { connection_state: state } = contact;
  if (state === 'online' || state === 'reachable' || state === 'unreachable') return state;
  const seenRecently =
    contact.last_seen != null &&
    Date.now() - new Date(contact.last_seen).getTime() <= REACHABLE_MS;
  if (seenRecently) return 'reachable';
  return 'unreachable';
}

export function hasContactRoute(contact: Pick<Contact, 'peer_id' | 'multiaddrs'>): boolean {
  return Boolean(contact.peer_id) || (contact.multiaddrs?.length ?? 0) > 0;
}

export function presenceTier(
  contact: Pick<Contact, 'connection_state' | 'last_seen' | 'peer_id' | 'multiaddrs'>
): PresenceTier | null {
  const state = effectiveConnectionState(contact);
  if (state === 'online') return 'connected';
  if (state === 'reachable') return 'reachable';
  return null;
}

export function isContactOnline(
  contact: Pick<Contact, 'connection_state' | 'last_seen' | 'peer_id' | 'multiaddrs'>
): boolean {
  return effectiveConnectionState(contact) === 'online';
}

export function connectionLabel(
  contact: Pick<Contact, 'connection_state' | 'last_seen' | 'peer_id' | 'multiaddrs'>
): string {
  const state = effectiveConnectionState(contact);
  if (state === 'online') return 'connected';
  if (state === 'reachable') return 'reachable';
  if (state === 'unreachable' && !hasContactRoute(contact)) return 'no route yet';
  return '';
}

export function presenceIndicator(
  contact: Pick<Contact, 'connection_state' | 'last_seen' | 'peer_id' | 'multiaddrs'>
): string {
  const state = effectiveConnectionState(contact);
  if (state === 'online') return '●';
  if (state === 'reachable') return '◐';
  return '○';
}

export function showsConnectionStatus(
  contact: Pick<Contact, 'connection_state' | 'last_seen' | 'peer_id' | 'multiaddrs'>
): boolean {
  const tier = presenceTier(contact);
  if (tier) return true;
  return effectiveConnectionState(contact) === 'unreachable' && !hasContactRoute(contact);
}

export function groupDmThreads(threads: DmThread[]): {
  connected: DmThread[];
  reachable: DmThread[];
  other: DmThread[];
} {
  const connected: DmThread[] = [];
  const reachable: DmThread[] = [];
  const other: DmThread[] = [];
  for (const thread of threads) {
    const tier = presenceTier(thread.contact);
    if (tier === 'connected') connected.push(thread);
    else if (tier === 'reachable') reachable.push(thread);
    else other.push(thread);
  }
  return { connected, reachable, other };
}

export type DeliveryTickState = 'sending' | 'sent' | 'delivered' | 'failed';

/** Map server delivery status to tick UI (own messages only). */
export function deliveryTickState(
  status: ConversationMessage['delivery_status'],
  optimistic = false
): DeliveryTickState | null {
  if (status == null) return null;
  if (optimistic && status === 'pending') return 'sending';
  if (status === 'pending') return 'sending';
  if (status === 'sent') return 'sent';
  if (status === 'delivered') return 'delivered';
  if (status === 'failed' || status === 'expired') return 'failed';
  return null;
}

export function messageTtlLabel(expiresAt: string): string {
  const ms = new Date(expiresAt).getTime() - Date.now();
  if (ms <= 0) return 'expired';
  const days = Math.ceil(ms / 86_400_000);
  return days <= 1 ? '<1d left' : `${days}d left`;
}

const PENDING_MESSAGE_PREFIX = 'pending-';

export function isOptimisticMessageId(contentId: string): boolean {
  return contentId.startsWith(PENDING_MESSAGE_PREFIX);
}

export function createOptimisticMessage(body: string): ConversationMessage {
  const at = new Date().toISOString();
  return {
    content_id: `${PENDING_MESSAGE_PREFIX}${crypto.randomUUID()}`,
    body,
    at,
    expires_at: new Date(Date.now() + 7 * 86_400_000).toISOString(),
    is_own: true,
    delivery_status: 'pending'
  };
}

/** Keep in-flight optimistic sends visible until the server returns a matching row. */
export function mergeConversationMessages(
  server: ConversationMessage[],
  optimistic: ConversationMessage[]
): ConversationMessage[] {
  const merged = [...server];
  for (const opt of optimistic) {
    if (!isOptimisticMessageId(opt.content_id)) continue;
    const duplicate = server.some(
      (row) =>
        row.is_own &&
        row.body === opt.body &&
        Math.abs(new Date(row.at).getTime() - new Date(opt.at).getTime()) < 60_000
    );
    if (!duplicate) merged.push(opt);
  }
  merged.sort((a, b) => new Date(a.at).getTime() - new Date(b.at).getTime());
  return merged;
}
