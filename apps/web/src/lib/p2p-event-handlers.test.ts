import { describe, expect, it } from 'vitest';

import {
	appendConversationMessage,
	canPatchOpenConversation,
	conversationMessageFromUiEvent,
	isP2pStatusChangedEvent,
	shouldRefreshFeedFromEvent,
	shouldRefreshMessagesFromEvent,
	shouldRefreshPeersFromEvent,
	type P2pUiEvent
} from './p2p-event-handlers';

const baseEvent: P2pUiEvent = {
	at: '2026-06-27T12:00:00.000Z',
	kind: 'message_received',
	detail: 'Alice: hi',
	sender_id: 'signer-1',
	contact_id: 'contact-1',
	content_id: 'msg-1',
	body: 'hi',
	content_type: 'message',
	expires_at: '2026-07-04T12:00:00.000Z'
};

describe('p2p event handlers', () => {
	it('flags message refresh kinds', () => {
		expect(shouldRefreshMessagesFromEvent({ ...baseEvent, kind: 'message_received' })).toBe(true);
		expect(shouldRefreshMessagesFromEvent({ ...baseEvent, kind: 'message_sent' })).toBe(true);
		expect(shouldRefreshMessagesFromEvent({ ...baseEvent, kind: 'delivery_acked' })).toBe(true);
		expect(shouldRefreshMessagesFromEvent({ ...baseEvent, kind: 'dial' })).toBe(false);
	});

	it('flags feed refresh for comments and blob sync', () => {
		expect(
			shouldRefreshFeedFromEvent({ ...baseEvent, kind: 'comment_received', content_type: 'comment' })
		).toBe(true);
		expect(shouldRefreshFeedFromEvent({ ...baseEvent, kind: 'blob_sync' })).toBe(true);
	});

	it('flags peer refresh kinds', () => {
		expect(shouldRefreshPeersFromEvent({ ...baseEvent, kind: 'peer_connected' })).toBe(true);
		expect(shouldRefreshPeersFromEvent({ ...baseEvent, kind: 'message_received' })).toBe(false);
	});

	it('flags feed refresh for received posts', () => {
		expect(shouldRefreshFeedFromEvent({ ...baseEvent, content_type: 'post' })).toBe(true);
		expect(shouldRefreshFeedFromEvent(baseEvent)).toBe(false);
	});

	it('builds a conversation message payload', () => {
		const row = conversationMessageFromUiEvent(baseEvent);
		expect(row).toMatchObject({
			content_id: 'msg-1',
			body: 'hi',
			is_own: false,
			delivery_status: null
		});
	});

	it('patches only the open conversation contact', () => {
		expect(canPatchOpenConversation(baseEvent, 'contact-1')).toBe(true);
		expect(canPatchOpenConversation(baseEvent, 'contact-2')).toBe(false);
	});

	it('appends without duplicate content ids', () => {
		const first = conversationMessageFromUiEvent(baseEvent)!;
		const merged = appendConversationMessage([], first);
		expect(merged).toHaveLength(1);
		expect(appendConversationMessage(merged, first)).toHaveLength(1);
	});

	it('detects p2p status changed events', () => {
		expect(isP2pStatusChangedEvent({ ...baseEvent, kind: 'p2p_status_changed' })).toBe(true);
		expect(isP2pStatusChangedEvent(baseEvent)).toBe(false);
	});
});
