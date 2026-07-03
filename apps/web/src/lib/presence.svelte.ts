import { api, type P2pStatus } from '$lib/api';
import { ApiRequestError } from '$lib/api-errors';
import { identityState, refreshIdentity } from '$lib/identity.svelte';
import {
	canPatchOpenConversation,
	conversationMessageFromUiEvent,
	shouldRefreshFeedFromEvent,
	shouldRefreshMessagesFromEvent,
	shouldRefreshPeersFromEvent,
	type P2pUiEvent
} from '$lib/p2p-event-handlers';
import { patchFeedFromEvent } from '$lib/feed-sync';
import { patchInboxFromEvent } from '$lib/messages-sync';
import {
	getOpenConversationId,
	patchConversationFromEvent,
	patchDeliveryFromEvent
} from '$lib/conversation-sync';

const REFRESH_DEBOUNCE_MS = 400;
const P2P_RECOVERY_INITIAL_MS = 15_000;
const P2P_RECOVERY_MAX_MS = 60_000;
const P2P_REFRESH_MIN_GAP_MS = 15_000;

export type P2pUiEventPayload = P2pUiEvent;

const MESSAGE_ACTIVITY_KINDS = new Set(['message_received', 'delivery_acked', 'outbox_flush']);

type FeedRefreshFn = () => void | Promise<void>;
type InboxRefreshFn = () => void | Promise<void>;
type ConversationRefreshFn = () => void | Promise<void>;
type RefreshChannel = 'feed' | 'inbox' | 'conversation';

type ChannelState = {
	inFlight: boolean;
	debounceTimer: ReturnType<typeof setTimeout> | null;
};

let lastConnectedKey = '';
let lastActivityAt: string | null = null;
let lastTone = '';
let lastPendingOutbox: number | null = null;
let feedRefresh: FeedRefreshFn | null = null;
let inboxRefresh: InboxRefreshFn | null = null;
let conversationRefresh: ConversationRefreshFn | null = null;
let pulseUntil = 0;
let p2pRecoveryTimer: ReturnType<typeof setTimeout> | null = null;
let p2pRecoveryDelayMs = P2P_RECOVERY_INITIAL_MS;
let p2pRefreshInFlight = false;
let lastP2pRefreshAttemptAt = 0;
let streamTransportProbeInFlight = false;

const channelState: Record<RefreshChannel, ChannelState> = {
	feed: { inFlight: false, debounceTimer: null },
	inbox: { inFlight: false, debounceTimer: null },
	conversation: { inFlight: false, debounceTimer: null }
};

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

function refreshFnForChannel(channel: RefreshChannel): (() => void | Promise<void>) | null {
	switch (channel) {
		case 'feed':
			return feedRefresh;
		case 'inbox':
			return inboxRefresh;
		case 'conversation':
			return conversationRefresh;
	}
}

function clearChannelDebounce(channel: RefreshChannel) {
	const state = channelState[channel];
	if (state.debounceTimer) {
		clearTimeout(state.debounceTimer);
		state.debounceTimer = null;
	}
}

async function executeChannelRefresh(channel: RefreshChannel) {
	const fn = refreshFnForChannel(channel);
	if (!fn) return;

	const state = channelState[channel];
	if (state.inFlight) {
		scheduleChannelRefresh(channel);
		return;
	}

	state.inFlight = true;
	try {
		await fn();
	} catch {
		// pages swallow errors for background refresh
	} finally {
		state.inFlight = false;
	}
}

function scheduleChannelRefresh(channel: RefreshChannel) {
	if (!refreshFnForChannel(channel)) return;

	clearChannelDebounce(channel);
	channelState[channel].debounceTimer = setTimeout(() => {
		channelState[channel].debounceTimer = null;
		void executeChannelRefresh(channel);
	}, REFRESH_DEBOUNCE_MS);
}

function hasNewMessageActivity(status: P2pStatus, previousActivityAt: string | null): boolean {
	const sinceMs = previousActivityAt ? new Date(previousActivityAt).getTime() : 0;
	return status.recent_activity.some(
		(event) =>
			MESSAGE_ACTIVITY_KINDS.has(event.kind) && new Date(event.at).getTime() > sinceMs
	);
}

function hasNewPostReceived(status: P2pStatus, previousActivityAt: string | null): boolean {
	const sinceMs = previousActivityAt ? new Date(previousActivityAt).getTime() : 0;
	return status.recent_activity.some(
		(event) =>
			event.kind === 'message_received' &&
			event.content_type === 'post' &&
			new Date(event.at).getTime() > sinceMs
	);
}

function notifyInboxRefresh() {
	scheduleChannelRefresh('inbox');
}

function notifyConversationRefresh() {
	scheduleChannelRefresh('conversation');
}

function notifyFeedRefresh() {
	scheduleChannelRefresh('feed');
}

function notifyMessageRefresh() {
	notifyInboxRefresh();
	notifyConversationRefresh();
}

function p2pStatusNeedsRecovery(status: P2pStatus | null): boolean {
	if (!status) return true;
	if (!status.running) return true;
	if (status.tone === 'error' || status.tone === 'off') return true;
	if (status.relay_configured && !status.relay_connected) return true;
	if (status.relay_tcp_reachable === false) return true;
	return false;
}

function clearP2pRecoveryTimer() {
	if (p2pRecoveryTimer) {
		clearTimeout(p2pRecoveryTimer);
		p2pRecoveryTimer = null;
	}
}

function resetP2pRecoveryBackoff() {
	p2pRecoveryDelayMs = P2P_RECOVERY_INITIAL_MS;
	clearP2pRecoveryTimer();
}

function scheduleP2pRecoveryRetry() {
	if (!identityState.apiOnline || !identityState.identity) return;
	if (p2pRecoveryTimer) return;
	if (typeof document !== 'undefined' && document.visibilityState !== 'visible') return;

	p2pRecoveryTimer = setTimeout(() => {
		p2pRecoveryTimer = null;
		void refreshP2pLive();
	}, p2pRecoveryDelayMs);
	p2pRecoveryDelayMs = Math.min(p2pRecoveryDelayMs * 2, P2P_RECOVERY_MAX_MS);
}

function markApiTransportOffline(error: ApiRequestError) {
	identityState.apiOnline = false;
	identityState.apiError = { kind: error.kind, message: error.message };
	identityState.p2pInfo = null;
	identityState.p2pStatus = null;
	resetP2pRecoveryBackoff();
}

/**
 * SSE transport error: close the stream, probe API health once, then reconnect
 * only if the API is still up. Avoids EventSource auto-reconnect spam when offline.
 */
export async function handleP2pStreamTransportError(reconnect?: () => void): Promise<void> {
	if (streamTransportProbeInFlight) return;
	streamTransportProbeInFlight = true;
	stopP2pLiveRecovery();
	try {
		await refreshIdentity({ silent: true });
		if (identityState.apiOnline && identityState.identity && reconnect) {
			reconnect();
		}
	} finally {
		streamTransportProbeInFlight = false;
	}
}

/** Refresh P2P status when the app opens or the tab becomes visible again. */
export function refreshP2pOnAppOpen() {
	lastPendingOutbox = null;
	resetP2pRecoveryBackoff();
	void refreshP2pLive({ force: true });
}

export function stopP2pLiveRecovery() {
	resetP2pRecoveryBackoff();
}

export function handleP2pUiEvent(event: P2pUiEvent) {
	if (event.kind === 'catch_up') {
		void refreshP2pLive({ force: true });
		notifyMessageRefresh();
		notifyFeedRefresh();
		return;
	}

	if (event.kind === 'dial' || event.kind === 'dial_failed') {
		void refreshP2pLive();
		return;
	}

	if (shouldRefreshPeersFromEvent(event)) {
		pulseUntil = Date.now() + 2_500;
		notifyInboxRefresh();
		notifyConversationRefresh();
		void refreshP2pLive();
		return;
	}

	if (shouldRefreshFeedFromEvent(event)) {
		if (!patchFeedFromEvent(event)) {
			notifyFeedRefresh();
		}
	}

	if (!shouldRefreshMessagesFromEvent(event)) return;

	pulseUntil = Date.now() + 2_500;

	if (patchDeliveryFromEvent(event)) {
		return;
	}

	const incoming = conversationMessageFromUiEvent(event);
	const openContactId = getOpenConversationId();
	const forOpenChat =
		incoming !== null && canPatchOpenConversation(event, openContactId);

	if (forOpenChat && patchConversationFromEvent(event)) {
		patchInboxFromEvent(event);
		return;
	}

	if (patchInboxFromEvent(event)) {
		if (forOpenChat) notifyConversationRefresh();
		return;
	}

	notifyMessageRefresh();
}

/** Called when the tab becomes visible — refreshes any registered message views. */
export function refreshMessagesOnVisible() {
	notifyMessageRefresh();
}

export async function refreshP2pLive(options: { force?: boolean } = {}) {
	if (!identityState.apiOnline || !identityState.identity) return;

	const now = Date.now();
	if (!options.force) {
		if (p2pRefreshInFlight) return;
		if (now - lastP2pRefreshAttemptAt < P2P_REFRESH_MIN_GAP_MS) return;
	}
	if (p2pRefreshInFlight) return;

	p2pRefreshInFlight = true;
	lastP2pRefreshAttemptAt = now;

	try {
		const status = await api.p2pStatus();
		const connectedKey = status.connected_peer_ids.slice().sort().join(',');
		const activityAt = status.last_activity_at ?? null;
		const previousActivityAt = lastActivityAt;
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

		if (activityAdvanced && hasNewMessageActivity(status, previousActivityAt)) {
			if (hasNewPostReceived(status, previousActivityAt)) {
				notifyFeedRefresh();
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

		if (p2pStatusNeedsRecovery(status)) {
			scheduleP2pRecoveryRetry();
		} else {
			resetP2pRecoveryBackoff();
		}
	} catch (error) {
		if (error instanceof ApiRequestError && error.kind === 'offline') {
			markApiTransportOffline(error);
			return;
		}
		scheduleP2pRecoveryRetry();
	} finally {
		p2pRefreshInFlight = false;
	}
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
