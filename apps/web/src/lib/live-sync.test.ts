import { describe, expect, it, vi } from 'vitest';

vi.mock('$lib/api', () => ({ api: {} }));
vi.mock('$lib/identity.svelte', () => ({
	identityState: { identity: { signing_pubkey: 'me' }, apiOnline: true }
}));
vi.mock('$lib/local-cache', () => ({
	writeCachedMessages: vi.fn(),
	writeCachedFeed: vi.fn()
}));

import { patchFeedFromEvent, seedFeedSnapshot } from './feed-sync';
import { patchInboxFromEvent, seedInboxSnapshot } from './messages-sync';
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
