import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$lib/api', () => ({ api: {} }));
vi.mock('$lib/identity.svelte', () => ({
	identityState: { identity: { signing_pubkey: 'me' }, apiOnline: true }
}));
vi.mock('$lib/local-cache', () => ({
	writeCachedMessages: vi.fn(),
	writeCachedFeed: vi.fn(),
	writeCachedConversation: vi.fn()
}));

import { patchFeedFromEvent, seedFeedSnapshot } from './feed-sync';
import { patchInboxFromEvent, seedInboxSnapshot } from './messages-sync';
import {
	applyServerConversation,
	patchConversationFromEvent,
	patchDeliveryFromEvent,
	seedConversationSnapshot,
	setOpenConversation,
	subscribeConversationSync
} from './conversation-sync';
import type { P2pUiEvent } from './p2p-event-handlers';

describe('messages-sync', () => {
	it('patches the thread list from an SSE dm event', () => {
		seedInboxSnapshot({
			contacts: [
				{
					id: 'contact-1',
					phone_hash: null,
					display_name: 'Alice',
					peer_id: 'peer-1',
					signing_pubkey: 'signer-1',
					encryption_pubkey: 'enc-1',
					last_seen: null,
					connection_state: 'offline'
				}
			],
			inbox: []
		});

		const event: P2pUiEvent = {
			at: '2026-06-27T12:00:00.000Z',
			kind: 'message_received',
			detail: 'Alice: hello',
			sender_id: 'signer-1',
			contact_id: 'contact-1',
			content_id: 'msg-1',
			body: 'hello',
			content_type: 'message',
			expires_at: '2026-07-04T12:00:00.000Z'
		};

		expect(patchInboxFromEvent(event)).toBe(true);
	});

	it('patches the feed from an SSE post event', () => {
		seedFeedSnapshot([]);

		const event: P2pUiEvent = {
			at: '2026-06-27T12:00:00.000Z',
			kind: 'message_received',
			detail: 'Alice: new post',
			sender_id: 'signer-1',
			contact_id: 'contact-1',
			content_id: 'post-1',
			body: 'new post',
			content_type: 'post',
			expires_at: '2026-07-04T12:00:00.000Z'
		};

		expect(patchFeedFromEvent(event)).toBe(true);
	});
});

describe('conversation-sync', () => {
	const ownMessage = {
		content_id: 'msg-own-1',
		body: 'hello back',
		at: '2026-06-27T12:00:00.000Z',
		expires_at: '2026-07-04T12:00:00.000Z',
		is_own: true,
		delivery_status: 'sent' as const
	};

	beforeEach(() => {
		setOpenConversation(null);
	});

	it('patches the open conversation from an SSE dm event', () => {
		setOpenConversation('contact-1');
		seedConversationSnapshot('contact-1', []);

		const event: P2pUiEvent = {
			at: '2026-06-27T12:01:00.000Z',
			kind: 'message_received',
			detail: 'Alice: hello',
			sender_id: 'signer-1',
			contact_id: 'contact-1',
			content_id: 'msg-1',
			body: 'hello',
			content_type: 'message',
			expires_at: '2026-07-04T12:00:00.000Z'
		};

		expect(patchConversationFromEvent(event)).toBe(true);
	});

	it('patches delivery status for one own message', () => {
		setOpenConversation('contact-1');
		seedConversationSnapshot('contact-1', [ownMessage]);

		const event: P2pUiEvent = {
			at: '2026-06-27T12:02:00.000Z',
			kind: 'delivery_acked',
			detail: 'Delivered msg-own-1 to Alice',
			contact_id: 'contact-1',
			content_id: 'msg-own-1',
			content_type: 'message'
		};

		expect(patchDeliveryFromEvent(event)).toBe(true);
	});

	it('keeps SSE rows when the server list lags', () => {
		setOpenConversation('contact-1');
		let latest: { content_id: string }[] = [];
		const unsub = subscribeConversationSync((messages) => {
			latest = messages;
		});

		seedConversationSnapshot('contact-1', [
			{
				content_id: 'msg-sse',
				body: 'from sse',
				at: '2026-06-27T12:01:00.000Z',
				expires_at: '2026-07-04T12:00:00.000Z',
				is_own: false,
				delivery_status: null
			}
		]);
		applyServerConversation('contact-1', []);
		unsub();

		expect(latest.some((row) => row.content_id === 'msg-sse')).toBe(true);
	});
});
