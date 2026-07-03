import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('$lib/api', () => ({
	api: {
		p2pStatus: vi.fn()
	}
}));
vi.mock('$lib/identity.svelte', () => ({
	identityState: { identity: { signing_pubkey: 'me' }, apiOnline: true },
	refreshIdentity: vi.fn()
}));

import { api } from '$lib/api';
import { refreshIdentity } from '$lib/identity.svelte';
import {
	handleP2pStreamTransportError,
	handleP2pUiEvent,
	registerConversationRefresh,
	registerFeedRefresh,
	registerInboxRefresh
} from '$lib/presence.svelte';
import { setOpenConversation } from '$lib/conversation-sync';
import { seedInboxSnapshot } from '$lib/messages-sync';
import type { P2pUiEvent } from '$lib/p2p-event-handlers';

const dmEvent: P2pUiEvent = {
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

describe('presence live sync routing', () => {
	beforeEach(() => {
		vi.useFakeTimers();
		setOpenConversation(null);
		registerFeedRefresh(null);
		registerInboxRefresh(null);
		registerConversationRefresh(null);
		vi.mocked(api.p2pStatus).mockResolvedValue({
			running: true,
			peer_id: 'peer-me',
			listen_addresses: [],
			connected_peer_ids: [],
			relay_configured: false,
			relay_peer_id: null,
			relay_connected: false,
			relay_tcp_reachable: null,
			pending_outbox_count: 0,
			dial_in_progress: false,
			last_activity_at: null,
			recent_activity: [],
			layers: {
				node: 'running',
				relay: 'not_configured',
				friends: 'offline',
				sync: 'idle',
				friends_online_count: 0,
				pending_outbox_count: 0
			},
			labels: {
				headline: 'P2P',
				node: 'Node: running',
				relay: 'Relay: not configured',
				friends: 'Friends: offline',
				sync: 'Outbox: idle'
			},
			tone: 'idle'
		});
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('schedules feed refresh on catch_up', () => {
		const feedRefresh = vi.fn();
		registerFeedRefresh(feedRefresh);

		handleP2pUiEvent({
			at: '2026-06-27T12:00:00.000Z',
			kind: 'catch_up',
			detail: 'refresh'
		});

		vi.advanceTimersByTime(500);
		expect(feedRefresh).toHaveBeenCalled();
	});

	it('reconciles open chat when inbox patches before durable snapshot exists', () => {
		const inboxRefresh = vi.fn();
		const conversationRefresh = vi.fn();
		registerInboxRefresh(inboxRefresh);
		registerConversationRefresh(conversationRefresh);

		setOpenConversation('contact-1');
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

		handleP2pUiEvent(dmEvent);

		vi.advanceTimersByTime(500);
		expect(conversationRefresh).toHaveBeenCalled();
		expect(inboxRefresh).not.toHaveBeenCalled();
	});

	it('probes identity once on SSE transport error instead of P2P recovery spam', async () => {
		vi.mocked(api.p2pStatus).mockClear();
		vi.mocked(refreshIdentity).mockResolvedValue(undefined);

		await handleP2pStreamTransportError();

		expect(refreshIdentity).toHaveBeenCalledWith({ silent: true });
		expect(api.p2pStatus).not.toHaveBeenCalled();
	});
});
