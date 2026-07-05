import type {
	Contact,
	ConversationMessage,
	FeedItem,
	InboxEntry,
	ProfilePhoto
} from '$lib/api';
import { blobUrl } from '$lib/api';

const DB_NAME = 'inertia-cache';
const DB_VERSION = 1;
const STORE = 'snapshots';

interface CacheEnvelope<T> {
	saved_at: string;
	data: T;
}

const fingerprintByKey = new Map<string, string>();

function openDb(): Promise<IDBDatabase> {
	return new Promise((resolve, reject) => {
		const request = indexedDB.open(DB_NAME, DB_VERSION);
		request.onerror = () => reject(request.error ?? new Error('Failed to open cache'));
		request.onsuccess = () => resolve(request.result);
		request.onupgradeneeded = () => {
			const db = request.result;
			if (!db.objectStoreNames.contains(STORE)) {
				db.createObjectStore(STORE);
			}
		};
	});
}

async function readSnapshot<T>(key: string): Promise<CacheEnvelope<T> | null> {
	if (typeof indexedDB === 'undefined') return null;
	try {
		const db = await openDb();
		return new Promise((resolve, reject) => {
			const tx = db.transaction(STORE, 'readonly');
			const request = tx.objectStore(STORE).get(key);
			request.onerror = () => reject(request.error ?? new Error('Cache read failed'));
			request.onsuccess = () =>
				resolve((request.result as CacheEnvelope<T> | undefined) ?? null);
		});
	} catch {
		return null;
	}
}

async function writeSnapshot<T>(key: string, data: T): Promise<void> {
	if (typeof indexedDB === 'undefined') return;
	try {
		const db = await openDb();
		const envelope: CacheEnvelope<T> = { saved_at: new Date().toISOString(), data };
		await new Promise<void>((resolve, reject) => {
			const tx = db.transaction(STORE, 'readwrite');
			const request = tx.objectStore(STORE).put(envelope, key);
			request.onerror = () => reject(request.error ?? new Error('Cache write failed'));
			request.onsuccess = () => resolve();
		});
	} catch {
		// cache is best-effort
	}
}

async function writeSnapshotIfChanged<T>(
	key: string,
	data: T,
	fingerprint: string
): Promise<void> {
	if (fingerprintByKey.get(key) === fingerprint) return;
	await writeSnapshot(key, data);
	fingerprintByKey.set(key, fingerprint);
}

function fingerprintFeed(items: FeedItem[]): string {
	if (items.length === 0) return 'empty';
	const newest = items[0];
	return `${items.length}:${newest.content_id}:${newest.created_at}`;
}

function fingerprintMessages(contacts: Contact[], inbox: InboxEntry[]): string {
	let maxReceived = '';
	for (const entry of inbox) {
		if (entry.received_at > maxReceived) maxReceived = entry.received_at;
	}
	const presence = contacts
		.map(
			(c) =>
				`${c.id}:${c.connection_state}:${c.last_seen ?? ''}:${c.peer_id ?? ''}:${(c.multiaddrs ?? []).join(',')}`
		)
		.join('|');
	return `${contacts.length}:${inbox.length}:${maxReceived}:${presence}`;
}

export function fingerprintConversation(messages: ConversationMessage[]): string {
	if (messages.length === 0) return 'empty';
	const last = messages[messages.length - 1];
	const ownDelivery = messages
		.filter((message) => message.is_own)
		.map((message) => `${message.content_id}:${message.delivery_status ?? ''}`)
		.join(',');
	return `${messages.length}:${last.content_id}:${last.at}:${ownDelivery}`;
}

export async function readCachedFeed(): Promise<{ items: FeedItem[]; saved_at: string } | null> {
	const snap = await readSnapshot<FeedItem[]>('feed');
	if (!snap?.data?.length) return null;
	return { items: snap.data, saved_at: snap.saved_at };
}

export async function writeCachedFeed(items: FeedItem[]): Promise<void> {
	if (items.length === 0) return;
	await writeSnapshotIfChanged('feed', items, fingerprintFeed(items));
}

export async function readCachedMessages(): Promise<{
	contacts: Contact[];
	inbox: InboxEntry[];
	saved_at: string;
} | null> {
	const snap = await readSnapshot<{ contacts: Contact[]; inbox: InboxEntry[] }>('messages');
	if (!snap?.data) return null;
	return { ...snap.data, saved_at: snap.saved_at };
}

export async function writeCachedMessages(contacts: Contact[], inbox: InboxEntry[]): Promise<void> {
	await writeSnapshotIfChanged(
		'messages',
		{ contacts, inbox },
		fingerprintMessages(contacts, inbox)
	);
}

export async function readCachedConversation(
	contactId: string
): Promise<{ messages: ConversationMessage[]; saved_at: string } | null> {
	const snap = await readSnapshot<ConversationMessage[]>(`conversation:${contactId}`);
	if (!snap?.data) return null;
	return { messages: snap.data, saved_at: snap.saved_at };
}

export async function writeCachedConversation(
	contactId: string,
	messages: ConversationMessage[]
): Promise<void> {
	const key = `conversation:${contactId}`;
	await writeSnapshotIfChanged(key, messages, fingerprintConversation(messages));
}

export interface ProfileSnapshot {
	photos: ProfilePhoto[];
	friend_count: number;
	/** blob_hash → data URL for offline thumbnails */
	blob_previews: Record<string, string>;
}

export async function readCachedProfile(): Promise<(ProfileSnapshot & { saved_at: string }) | null> {
	const snap = await readSnapshot<ProfileSnapshot>('profile');
	if (!snap?.data) return null;
	return { ...snap.data, saved_at: snap.saved_at };
}

export async function writeCachedProfile(snapshot: ProfileSnapshot): Promise<void> {
	await writeSnapshot('profile', snapshot);
}

async function blobToDataUrl(blob: Blob): Promise<string> {
	return new Promise((resolve, reject) => {
		const reader = new FileReader();
		reader.onload = () => resolve(reader.result as string);
		reader.onerror = () => reject(reader.error ?? new Error('Failed to read blob'));
		reader.readAsDataURL(blob);
	});
}

/** Cache profile photos + small data-URL previews for offline viewing. */
export async function buildProfileBlobPreviews(
	photos: ProfilePhoto[],
	limit = 24
): Promise<Record<string, string>> {
	const previews: Record<string, string> = {};
	for (const photo of photos.slice(0, limit)) {
		if (previews[photo.blob_hash]) continue;
		try {
			const res = await fetch(blobUrl(photo.blob_hash));
			if (!res.ok) continue;
			previews[photo.blob_hash] = await blobToDataUrl(await res.blob());
		} catch {
			// skip failed blobs
		}
	}
	return previews;
}

export function resolveCachedBlobUrl(
	hash: string,
	previews: Record<string, string>,
	apiOnline: boolean
): string {
	if (!apiOnline && previews[hash]) return previews[hash];
	return blobUrl(hash);
}

export function formatCacheAge(iso: string): string {
	const ms = Date.now() - new Date(iso).getTime();
	if (ms < 60_000) return 'just now';
	const mins = Math.floor(ms / 60_000);
	if (mins < 60) return `${mins}m ago`;
	const hours = Math.floor(mins / 60);
	if (hours < 48) return `${hours}h ago`;
	return new Date(iso).toLocaleDateString();
}
