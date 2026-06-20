import { api, type P2pStatus } from '$lib/api';
import { identityState } from '$lib/identity.svelte';

const P2P_POLL_MS = 5_000;
const FEED_POLL_MS = 12_000;

type FeedRefreshFn = () => void | Promise<void>;
type InboxRefreshFn = () => void | Promise<void>;

let pollTimer: ReturnType<typeof setInterval> | null = null;
let lastConnectedKey = '';
let lastActivityAt: string | null = null;
let lastTone = '';
let feedRefresh: FeedRefreshFn | null = null;
let inboxRefresh: InboxRefreshFn | null = null;
let pulseUntil = 0;

export function presencePulseActive(): boolean {
	return Date.now() < pulseUntil;
}

export function registerFeedRefresh(fn: FeedRefreshFn | null) {
	feedRefresh = fn;
}

export function registerInboxRefresh(fn: InboxRefreshFn | null) {
	inboxRefresh = fn;
}

export async function refreshP2pLive() {
	if (!identityState.apiOnline || !identityState.identity) return;

	try {
		const status = await api.p2pStatus();
		const connectedKey = status.connected_peer_ids.slice().sort().join(',');
		const activityAt = status.last_activity_at ?? null;
		const tone = status.tone;

		if (
			connectedKey !== lastConnectedKey ||
			(activityAt && activityAt !== lastActivityAt) ||
			tone !== lastTone
		) {
			pulseUntil = Date.now() + 2_500;
		}

		lastConnectedKey = connectedKey;
		lastActivityAt = activityAt;
		lastTone = tone;
		identityState.p2pStatus = status;

		const latest = status.recent_activity[0];
		if (latest?.kind === 'message_received' && feedRefresh) {
			void feedRefresh();
		}
		if (latest?.kind === 'message_received' && inboxRefresh) {
			void inboxRefresh();
		}
	} catch {
		// keep last snapshot on transient errors
	}
}

async function pollTick() {
	if (typeof document !== 'undefined' && document.visibilityState !== 'visible') {
		return;
	}
	await refreshP2pLive();
}

export function startPresencePolling() {
	stopPresencePolling();
	void pollTick();
	pollTimer = setInterval(() => void pollTick(), P2P_POLL_MS);
}

export function stopPresencePolling() {
	if (pollTimer) {
		clearInterval(pollTimer);
		pollTimer = null;
	}
}

let feedPollTimer: ReturnType<typeof setInterval> | null = null;

export function startFeedPolling(refresh: FeedRefreshFn) {
	registerFeedRefresh(refresh);
	stopFeedPolling();
	feedPollTimer = setInterval(() => {
		if (document.visibilityState === 'visible') {
			void refresh();
		}
	}, FEED_POLL_MS);
}

export function stopFeedPolling() {
	if (feedPollTimer) {
		clearInterval(feedPollTimer);
		feedPollTimer = null;
	}
	registerFeedRefresh(null);
}

export function formatActivityLine(event: P2pStatus['recent_activity'][number]): string {
	switch (event.kind) {
		case 'peer_connected':
			return event.detail;
		case 'peer_disconnected':
			return event.detail;
		case 'message_received':
			return `Received: ${event.detail}`;
		case 'delivery_acked':
			return event.detail;
		case 'blob_sync':
			return event.detail;
		case 'outbox_flush':
			return event.detail;
		case 'dial':
		case 'dial_failed':
			return event.detail;
		default:
			return event.detail;
	}
}
