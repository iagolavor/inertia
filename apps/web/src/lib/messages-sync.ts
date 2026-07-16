import { api, type Contact, type InboxEntry } from '$lib/api';
import { getOpenConversationId } from '$lib/conversation-sync';
import { markDmThreadRead } from '$lib/dm-unread';
import { identityState } from '$lib/identity.svelte';
import { writeCachedMessages } from '$lib/local-cache';
import type { P2pUiEvent } from '$lib/p2p-event-handlers';

export type InboxSnapshot = {
	contacts: Contact[];
	inbox: InboxEntry[];
};

type InboxListener = (snapshot: InboxSnapshot) => void;

const listeners = new Set<InboxListener>();
let lastSnapshot: InboxSnapshot | null = null;

export function subscribeInboxSync(listener: InboxListener): () => void {
	listeners.add(listener);
	if (lastSnapshot) listener(lastSnapshot);
	return () => listeners.delete(listener);
}

export function seedInboxSnapshot(snapshot: InboxSnapshot): void {
	void writeCachedMessages(snapshot.contacts, snapshot.inbox);
	emit(snapshot);
}

function emit(snapshot: InboxSnapshot) {
	lastSnapshot = snapshot;
	for (const listener of listeners) listener(snapshot);
}

/** Background fetch used by SSE fallback while Messages or chat routes are open. */
export async function refreshInboxSilently(): Promise<void> {
	if (!identityState.identity || !identityState.apiOnline) return;
	try {
		const [contacts, inbox] = await Promise.all([api.listContacts(), api.listInbox()]);
		await writeCachedMessages(contacts, inbox);
		emit({ contacts, inbox });
	} catch {
		// background refresh — keep last good snapshot
	}
}

/** Apply an SSE row to the thread list without waiting for a full inbox fetch. */
export function patchInboxFromEvent(event: P2pUiEvent): boolean {
	if (event.kind !== 'message_received' || event.content_type !== 'message') return false;
	if (!event.content_id || !event.body || !event.sender_id || !lastSnapshot) return false;

	const senderId = event.contact_id ?? event.sender_id;
	if (lastSnapshot.inbox.some((row) => row.content_id === event.content_id)) {
		return true;
	}

	const entry: InboxEntry = {
		content_id: event.content_id,
		sender_id: senderId,
		received_at: event.at,
		expires_at: event.expires_at ?? new Date(Date.now() + 7 * 86_400_000).toISOString(),
		read_at: null,
		body: event.body,
		media_ref: null,
		content_type: 'message'
	};

	const inbox = [entry, ...lastSnapshot.inbox];
	void writeCachedMessages(lastSnapshot.contacts, inbox);
	emit({ contacts: lastSnapshot.contacts, inbox });

	const openId = getOpenConversationId();
	if (openId && (openId === senderId || openId === event.contact_id)) {
		markDmThreadRead(openId, entry.received_at);
	}
	return true;
}
