import type { Contact, ConversationMessage, InboxEntry } from '$lib/api';

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

export function isContactOnline(state: Contact['connection_state']): boolean {
  return state === 'online';
}

export function connectionLabel(state: Contact['connection_state']): string {
  if (state === 'online') return 'connected';
  if (state === 'unreachable') return 'not reachable';
  return 'offline';
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
