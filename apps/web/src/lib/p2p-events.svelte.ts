import { getApiBase } from '$lib/api-base';
import {
	handleP2pUiEvent,
	notifyP2pStreamDisconnected,
	refreshMessagesOnVisible,
	refreshP2pLive,
	type P2pUiEventPayload
} from '$lib/presence.svelte';

let source: EventSource | null = null;
let openedOnce = false;

function parseUiEvent(raw: string): P2pUiEventPayload | null {
	try {
		return JSON.parse(raw) as P2pUiEventPayload;
	} catch {
		return null;
	}
}

function resyncAfterStreamGap() {
	void refreshP2pLive();
	refreshMessagesOnVisible();
}

export function startP2pEventStream() {
	stopP2pEventStream();
	if (typeof EventSource === 'undefined') return;

	openedOnce = false;
	const url = `${getApiBase()}/p2p/events`;
	source = new EventSource(url);

	source.onopen = () => {
		if (openedOnce) {
			resyncAfterStreamGap();
		}
		openedOnce = true;
	};

	source.onmessage = (message) => {
		const event = parseUiEvent(message.data);
		if (event) handleP2pUiEvent(event);
	};

	source.onerror = () => {
		notifyP2pStreamDisconnected();
	};
}

export function stopP2pEventStream() {
	if (source) {
		source.close();
		source = null;
	}
	openedOnce = false;
}
