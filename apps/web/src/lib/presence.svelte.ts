import { api, type P2pStatus } from '$lib/api';
import { identityState } from '$lib/identity.svelte';

const P2P_POLL_MS = 5_000;
const FEED_POLL_MS = 12_000;
const INBOX_POLL_MS = 8_000;
const CONVERSATION_POLL_MS = 4_000;

const MESSAGE_ACTIVITY_KINDS = new Set(['message_received', 'delivery_acked', 'outbox_flush']);

type FeedRefreshFn = () => void | Promise<void>;
type InboxRefreshFn = () => void | Promise<void>;
type ConversationRefreshFn = () => void | Promise<void>;

let pollTimer: ReturnType<typeof setInterval> | null = null;
let lastConnectedKey = '';
let lastActivityAt: string | null = null;
let lastTone = '';
let lastPendingOutbox: number | null = null;
let feedRefresh: FeedRefreshFn | null = null;
let inboxRefresh: InboxRefreshFn | null = null;
let conversationRefresh: ConversationRefreshFn | null = null;
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

export function registerConversationRefresh(fn: ConversationRefreshFn | null) {
	conversationRefresh = fn;
}

function latestActivityKind(status: P2pStatus): string | undefined {
	return status.recent_activity[0]?.kind;
}

function hasMessageActivity(status: P2pStatus): boolean {
	const kind = latestActivityKind(status);
	return kind !== undefined && MESSAGE_ACTIVITY_KINDS.has(kind);
}

function notifyInboxRefresh() {
	if (inboxRefresh) void inboxRefresh();
}

function notifyConversationRefresh() {
	if (conversationRefresh) void conversationRefresh();
}

function notifyMessageRefresh() {
	notifyInboxRefresh();
	notifyConversationRefresh();
}

/** Called when the tab becomes visible — refreshes any registered message views. */
export function refreshMessagesOnVisible() {
	notifyMessageRefresh();
}

export async function refreshP2pLive() {
	if (!identityState.apiOnline || !identityState.identity) return;

	try {
		const status = await api.p2pStatus();
		const connectedKey = status.connected_peer_ids.slice().sort().join(',');
		const activityAt = status.last_activity_at ?? null;
		const tone = status.tone;
		const pendingOutbox = status.layers.pending_outbox_count;
		const peersChanged = connectedKey !== lastConnectedKey;
		const activityAdvanced = Boolean(activityAt && activityAt !== lastActivityAt);
		const outboxChanged = lastPendingOutbox !== null && pendingOutbox !== lastPendingOutbox;

		if (peersChanged || activityAdvanced || tone !== lastTone) {
			pulseUntil = Date.now() + 2_500;
		}

		if (peersChanged) {
			notifyInboxRefresh();
			notifyConversationRefresh();
		}

		if (activityAdvanced && hasMessageActivity(status)) {
			const latest = status.recent_activity[0];
			if (latest?.kind === 'message_received' && feedRefresh) {
				void feedRefresh();
			}
			notifyMessageRefresh();
		}

		if (outboxChanged) {
			notifyMessageRefresh();
		}

		lastConnectedKey = connectedKey;
		lastActivityAt = activityAt;
		lastTone = tone;
		lastPendingOutbox = pendingOutbox;
		identityState.p2pStatus = status;
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
	lastPendingOutbox = null;
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

let inboxPollTimer: ReturnType<typeof setInterval> | null = null;

export function startInboxPolling(refresh: InboxRefreshFn) {
	registerInboxRefresh(refresh);
	stopInboxPolling();
	inboxPollTimer = setInterval(() => {
		if (document.visibilityState === 'visible') {
			void refresh();
		}
	}, INBOX_POLL_MS);
}

export function stopInboxPolling() {
	if (inboxPollTimer) {
		clearInterval(inboxPollTimer);
		inboxPollTimer = null;
	}
	registerInboxRefresh(null);
}

let conversationPollTimer: ReturnType<typeof setInterval> | null = null;

export function startConversationPolling(refresh: ConversationRefreshFn) {
	registerConversationRefresh(refresh);
	stopConversationPolling();
	conversationPollTimer = setInterval(() => {
		if (document.visibilityState === 'visible') {
			void refresh();
		}
	}, CONVERSATION_POLL_MS);
}

export function stopConversationPolling() {
	if (conversationPollTimer) {
		clearInterval(conversationPollTimer);
		conversationPollTimer = null;
	}
	registerConversationRefresh(null);
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
