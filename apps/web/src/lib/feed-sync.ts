import { api, type FeedItem } from '$lib/api';
import { identityState } from '$lib/identity.svelte';
import { writeCachedFeed } from '$lib/local-cache';
import type { P2pUiEvent } from '$lib/p2p-event-handlers';

type FeedListener = (items: FeedItem[]) => void;

const listeners = new Set<FeedListener>();
let lastFeed: FeedItem[] | null = null;

export function subscribeFeedSync(listener: FeedListener): () => void {
	listeners.add(listener);
	if (lastFeed) listener(lastFeed);
	return () => listeners.delete(listener);
}

export function seedFeedSnapshot(items: FeedItem[]): void {
	void writeCachedFeed(items);
	emit(items);
}

function emit(items: FeedItem[]) {
	lastFeed = items;
	for (const listener of listeners) listener(items);
}

/** Background fetch used by SSE fallback while the feed page is open. */
export async function refreshFeedSilently(): Promise<void> {
	if (!identityState.identity || !identityState.apiOnline) return;
	try {
		const items = await api.listFeed();
		await writeCachedFeed(items);
		emit(items);
	} catch {
		// background refresh — keep last good snapshot
	}
}

/** Apply an SSE post without waiting for a full feed fetch. */
export function patchFeedFromEvent(event: P2pUiEvent): boolean {
	if (event.kind !== 'message_received' || event.content_type !== 'post') return false;
	if (!event.content_id || !event.body || !event.sender_id) return false;
	if (lastFeed?.some((row) => row.content_id === event.content_id)) return true;

	const authorName = authorNameFromDetail(event.detail);

	const item: FeedItem = {
		content_id: event.content_id,
		author_id: event.sender_id,
		author_name: authorName,
		body: event.body,
		media_ref: null,
		media_kind: null,
		media_ready: false,
		created_at: event.at,
		expires_at: event.expires_at ?? new Date(Date.now() + 7 * 86_400_000).toISOString(),
		is_own: false,
		is_archived: false
	};

	const items = [item, ...(lastFeed ?? [])];
	void writeCachedFeed(items);
	emit(items);
	return true;
}

function authorNameFromDetail(detail: string): string {
	const idx = detail.indexOf(':');
	if (idx > 0) return detail.slice(0, idx).trim();
	return 'Friend';
}
