import { api, type P2pStatus } from '$lib/api';
import { identityState } from '$lib/identity.svelte';
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

const P2P_HEARTBEAT_MS = 60_000;
const FEED_POLL_MS = 12_000;
const INBOX_POLL_MS = 30_000;
const CONVERSATION_POLL_MS = 15_000;
const REFRESH_DEBOUNCE_MS = 400;
const TIMER_MIN_GAP_MS = 2_000;

export type P2pUiEventPayload = P2pUiEvent;

const MESSAGE_ACTIVITY_KINDS = new Set(['message_received', 'delivery_acked', 'outbox_flush']);

type FeedRefreshFn = () => void | Promise<void>;
type InboxRefreshFn = () => void | Promise<void>;
type ConversationRefreshFn = () => void | Promise<void>;
type ConversationPatchFn = (event: P2pUiEvent) => boolean;
type RefreshChannel = 'feed' | 'inbox' | 'conversation';

type ChannelState = {
	inFlight: boolean;
	debounceTimer: ReturnType<typeof setTimeout> | null;
	lastFetchedAt: number;
};

let pollTimer: ReturnType<typeof setInterval> | null = null;
let lastConnectedKey = '';
let lastActivityAt: string | null = null;
let lastTone = '';
let lastPendingOutbox: number | null = null;
let feedRefresh: FeedRefreshFn | null = null;
let inboxRefresh: InboxRefreshFn | null = null;
let conversationRefresh: ConversationRefreshFn | null = null;
let conversationPatch: ConversationPatchFn | null = null;
let openConversationContactId: string | null = null;
let pulseUntil = 0;

const channelState: Record<RefreshChannel, ChannelState> = {
	feed: { inFlight: false, debounceTimer: null, lastFetchedAt: 0 },
	inbox: { inFlight: false, debounceTimer: null, lastFetchedAt: 0 },
	conversation: { inFlight: false, debounceTimer: null, lastFetchedAt: 0 }
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

export function registerConversationEventPatch(
	contactId: string | null,
	fn: ConversationPatchFn | null
) {
	openConversationContactId = contactId;
	conversationPatch = fn;
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

async function executeChannelRefresh(channel: RefreshChannel, timerDriven: boolean) {
	const fn = refreshFnForChannel(channel);
	if (!fn) return;

	const state = channelState[channel];
	if (state.inFlight) {
		scheduleChannelRefresh(channel, timerDriven);
		return;
	}
	if (timerDriven && Date.now() - state.lastFetchedAt < TIMER_MIN_GAP_MS) return;

	state.inFlight = true;
	try {
		await fn();
		state.lastFetchedAt = Date.now();
	} catch {
		// pages swallow errors for background refresh
	} finally {
		state.inFlight = false;
	}
}

function scheduleChannelRefresh(channel: RefreshChannel, timerDriven: boolean) {
	if (!refreshFnForChannel(channel)) return;

	clearChannelDebounce(channel);
	channelState[channel].debounceTimer = setTimeout(() => {
		channelState[channel].debounceTimer = null;
		void executeChannelRefresh(channel, timerDriven);
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
	scheduleChannelRefresh('inbox', false);
}

function notifyConversationRefresh() {
	scheduleChannelRefresh('conversation', false);
}

function notifyFeedRefresh() {
	scheduleChannelRefresh('feed', false);
}

function notifyMessageRefresh() {
	notifyInboxRefresh();
	notifyConversationRefresh();
}

export function handleP2pUiEvent(event: P2pUiEvent) {
	if (event.kind === 'catch_up') {
		void refreshP2pLive();
		notifyMessageRefresh();
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

	const incoming = conversationMessageFromUiEvent(event);
	if (
		incoming &&
		canPatchOpenConversation(event, openConversationContactId) &&
		conversationPatch?.(event)
	) {
		patchInboxFromEvent(event);
		return;
	}

	if (patchInboxFromEvent(event)) return;

	notifyMessageRefresh();
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
	pollTimer = setInterval(() => void pollTick(), P2P_HEARTBEAT_MS);
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
			scheduleChannelRefresh('feed', true);
		}
	}, FEED_POLL_MS);
}

export function stopFeedPolling() {
	if (feedPollTimer) {
		clearInterval(feedPollTimer);
		feedPollTimer = null;
	}
	clearChannelDebounce('feed');
	registerFeedRefresh(null);
}

let inboxPollTimer: ReturnType<typeof setInterval> | null = null;

export function startInboxPolling(refresh: InboxRefreshFn) {
	registerInboxRefresh(refresh);
	stopInboxPolling();
	inboxPollTimer = setInterval(() => {
		if (document.visibilityState === 'visible') {
			scheduleChannelRefresh('inbox', true);
		}
	}, INBOX_POLL_MS);
}

export function stopInboxPolling() {
	if (inboxPollTimer) {
		clearInterval(inboxPollTimer);
		inboxPollTimer = null;
	}
	clearChannelDebounce('inbox');
	registerInboxRefresh(null);
}

let conversationPollTimer: ReturnType<typeof setInterval> | null = null;

export function startConversationPolling(refresh: ConversationRefreshFn) {
	registerConversationRefresh(refresh);
	stopConversationPolling();
	conversationPollTimer = setInterval(() => {
		if (document.visibilityState === 'visible') {
			scheduleChannelRefresh('conversation', true);
		}
	}, CONVERSATION_POLL_MS);
}

export function stopConversationPolling() {
	if (conversationPollTimer) {
		clearInterval(conversationPollTimer);
		conversationPollTimer = null;
	}
	clearChannelDebounce('conversation');
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
