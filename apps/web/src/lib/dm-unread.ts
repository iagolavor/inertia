import type { InboxEntry } from '$lib/api';

const STORAGE_KEY = 'inertia.dm.lastReadByContact';

type Listener = () => void;

const listeners = new Set<Listener>();
let lastReadByContact: Record<string, string> = load();
let baselineReady = Object.keys(lastReadByContact).length > 0;

function load(): Record<string, string> {
	if (typeof localStorage === 'undefined') return {};
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return {};
		const parsed = JSON.parse(raw) as unknown;
		if (!parsed || typeof parsed !== 'object') return {};
		const out: Record<string, string> = {};
		for (const [key, value] of Object.entries(parsed)) {
			if (typeof value === 'string' && value) out[key] = value;
		}
		return out;
	} catch {
		return {};
	}
}

function persist() {
	if (typeof localStorage === 'undefined') return;
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(lastReadByContact));
	} catch {
		// ignore quota / private mode
	}
}

function emit() {
	for (const listener of listeners) listener();
}

/** First visit: treat current inbox as already seen so the badge starts at zero. */
export function ensureDmUnreadBaseline(inbox: InboxEntry[]): void {
	if (baselineReady) return;
	const next = { ...lastReadByContact };
	for (const entry of inbox) {
		if (entry.content_type !== 'message') continue;
		const prev = next[entry.sender_id];
		if (!prev || new Date(entry.received_at) > new Date(prev)) {
			next[entry.sender_id] = entry.received_at;
		}
	}
	lastReadByContact = next;
	baselineReady = true;
	persist();
	emit();
}

export function markDmThreadRead(contactId: string, atIso = new Date().toISOString()): void {
	if (!contactId) return;
	const prev = lastReadByContact[contactId];
	if (prev && new Date(prev) >= new Date(atIso)) return;
	lastReadByContact = { ...lastReadByContact, [contactId]: atIso };
	baselineReady = true;
	persist();
	emit();
}

export function subscribeDmUnread(listener: Listener): () => void {
	listeners.add(listener);
	return () => listeners.delete(listener);
}

function isUnreadEntry(entry: InboxEntry): boolean {
	if (entry.content_type !== 'message') return false;
	const lastRead = lastReadByContact[entry.sender_id];
	return !lastRead || new Date(entry.received_at) > new Date(lastRead);
}

/** Unread inbound DM messages since the contact was last opened. */
export function countUnreadDmMessages(inbox: InboxEntry[]): number {
	let count = 0;
	for (const entry of inbox) {
		if (isUnreadEntry(entry)) count += 1;
	}
	return count;
}

export function countUnreadForContact(
	inbox: InboxEntry[],
	contactId: string,
	signingPubkey?: string
): number {
	let count = 0;
	for (const entry of inbox) {
		if (entry.sender_id !== contactId && entry.sender_id !== signingPubkey) continue;
		if (isUnreadEntry(entry)) count += 1;
	}
	return count;
}

export function formatUnreadBadge(count: number): string {
	if (count <= 0) return '';
	if (count > 9) return '9+';
	return String(count);
}
