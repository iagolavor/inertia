import { api, type ConversationMessage } from '$lib/api';
import { identityState } from '$lib/identity.svelte';
import { writeCachedConversation } from '$lib/local-cache';
import {
	appendConversationMessage,
	conversationMessageFromUiEvent,
	type P2pUiEvent
} from '$lib/p2p-event-handlers';

type ConversationListener = (messages: ConversationMessage[]) => void;

const listeners = new Set<ConversationListener>();
let openContactId: string | null = null;
let durableMessages: ConversationMessage[] | null = null;

export function setOpenConversation(contactId: string | null): void {
	openContactId = contactId;
	if (!contactId) {
		durableMessages = null;
	}
}

export function getOpenConversationId(): string | null {
	return openContactId;
}

export function subscribeConversationSync(listener: ConversationListener): () => void {
	listeners.add(listener);
	if (durableMessages) listener(durableMessages);
	return () => listeners.delete(listener);
}

function emit(contactId: string, durable: ConversationMessage[]): void {
	if (contactId !== openContactId) return;
	durableMessages = durable;
	void writeCachedConversation(contactId, durable);
	for (const listener of listeners) listener(durable);
}

export function seedConversationSnapshot(
	contactId: string,
	messages: ConversationMessage[]
): void {
	emit(contactId, messages);
}

function mergeServerWithSsePatches(
	server: ConversationMessage[],
	previous: ConversationMessage[] | null
): ConversationMessage[] {
	if (!previous) return server;
	const serverIds = new Set(server.map((message) => message.content_id));
	const sseOnly = previous.filter((message) => !serverIds.has(message.content_id));
	return [...server, ...sseOnly].sort(
		(a, b) => new Date(a.at).getTime() - new Date(b.at).getTime()
	);
}

export function applyServerConversation(
	contactId: string,
	serverMessages: ConversationMessage[]
): void {
	const merged = mergeServerWithSsePatches(serverMessages, durableMessages);
	emit(contactId, merged);
}

/** Background fetch used by SSE fallback while a conversation route is open. */
export async function refreshConversationSilently(contactId: string): Promise<void> {
	if (!identityState.identity || !identityState.apiOnline) return;
	if (contactId !== openContactId) return;

	try {
		const serverMessages = await api.listConversationMessages(contactId);
		applyServerConversation(contactId, serverMessages);
	} catch {
		// background refresh — keep last good durable snapshot
	}
}

/** Apply an SSE dm row to the open thread without waiting for a full fetch. */
export function patchConversationFromEvent(event: P2pUiEvent): boolean {
	if (!openContactId || !durableMessages) return false;

	const incoming = conversationMessageFromUiEvent(event);
	if (!incoming) return false;

	const next = appendConversationMessage(durableMessages, incoming);
	if (next === durableMessages) return true;

	emit(openContactId, next);
	return true;
}

/** Patch delivery ticks on one own message when the recipient acks. */
export function patchDeliveryFromEvent(event: P2pUiEvent): boolean {
	if (event.kind !== 'delivery_acked' || !event.content_id) return false;
	if (!openContactId || !durableMessages) return false;
	if (event.contact_id && event.contact_id !== openContactId) return false;

	const index = durableMessages.findIndex(
		(message) => message.content_id === event.content_id
	);
	if (index < 0) return false;

	const row = durableMessages[index];
	if (!row.is_own) return false;
	if (row.delivery_status === 'delivered') return true;

	const next = [...durableMessages];
	next[index] = { ...row, delivery_status: 'delivered' };
	emit(openContactId, next);
	return true;
}
