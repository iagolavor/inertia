import type { ConversationMessage } from '$lib/api';

export interface P2pUiEvent {
	at: string;
	kind: string;
	detail: string;
	sender_id?: string;
	contact_id?: string;
	content_id?: string;
	body?: string;
	content_type?: 'message' | 'post' | 'comment';
	expires_at?: string;
	post_id?: string;
	connected_peer_ids?: string[];
	tone?: string;
	pending_outbox_count?: number;
	dial_in_progress?: boolean;
	layers?: import('$lib/api').P2pLayers;
	labels?: import('$lib/api').P2pLayerLabels;
}

const MESSAGE_REFRESH_KINDS = new Set([
	'message_received',
	'message_sent',
	'delivery_acked'
]);

const PEER_REFRESH_KINDS = new Set(['peer_connected', 'peer_disconnected']);

const CONTACT_REFRESH_KINDS = new Set(['friend_request']);

const FEED_REFRESH_KINDS = new Set(['comment_received', 'blob_sync']);

export function shouldRefreshMessagesFromEvent(event: P2pUiEvent): boolean {
	if (event.kind === 'catch_up') return true;
	return MESSAGE_REFRESH_KINDS.has(event.kind);
}

export function shouldRefreshPeersFromEvent(event: P2pUiEvent): boolean {
	return PEER_REFRESH_KINDS.has(event.kind);
}

export function shouldRefreshContactsFromEvent(event: P2pUiEvent): boolean {
	return CONTACT_REFRESH_KINDS.has(event.kind);
}

export function shouldRefreshFeedFromEvent(event: P2pUiEvent): boolean {
	if (FEED_REFRESH_KINDS.has(event.kind)) return true;
	return event.kind === 'message_received' && event.content_type === 'post';
}

export function isP2pStatusChangedEvent(event: P2pUiEvent): boolean {
	return event.kind === 'p2p_status_changed';
}

export function conversationMessageFromUiEvent(event: P2pUiEvent): ConversationMessage | null {
	if (event.kind !== 'message_received' || event.content_type !== 'message') return null;
	if (!event.content_id || !event.body) return null;

	const expiresAt =
		event.expires_at ?? new Date(Date.now() + 7 * 86_400_000).toISOString();

	return {
		content_id: event.content_id,
		body: event.body,
		at: event.at,
		expires_at: expiresAt,
		is_own: false,
		delivery_status: null
	};
}

export function canPatchOpenConversation(
	event: P2pUiEvent,
	openContactId: string | null | undefined
): boolean {
	if (!openContactId) return false;
	if (event.kind !== 'message_received' || event.content_type !== 'message') return false;
	return event.contact_id === openContactId;
}

export function appendConversationMessage(
	existing: ConversationMessage[],
	incoming: ConversationMessage
): ConversationMessage[] {
	if (existing.some((row) => row.content_id === incoming.content_id)) {
		return existing;
	}
	return [...existing, incoming].sort(
		(a, b) => new Date(a.at).getTime() - new Date(b.at).getTime()
	);
}
