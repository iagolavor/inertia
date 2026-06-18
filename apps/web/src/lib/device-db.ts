import type { Identity } from '$lib/api';

const DB_NAME = 'inertia-device';
const DB_VERSION = 1;
const STORE = 'profile';
const DEVICE_KEY = 'device';

function openDb(): Promise<IDBDatabase> {
	return new Promise((resolve, reject) => {
		const request = indexedDB.open(DB_NAME, DB_VERSION);
		request.onerror = () => reject(request.error ?? new Error('Failed to open local database'));
		request.onsuccess = () => resolve(request.result);
		request.onupgradeneeded = () => {
			const db = request.result;
			if (!db.objectStoreNames.contains(STORE)) {
				db.createObjectStore(STORE);
			}
		};
	});
}

export async function readDeviceProfile(): Promise<Identity | null> {
	if (typeof indexedDB === 'undefined') return null;

	const db = await openDb();
	return new Promise((resolve, reject) => {
		const tx = db.transaction(STORE, 'readonly');
		const request = tx.objectStore(STORE).get(DEVICE_KEY);
		request.onerror = () => reject(request.error ?? new Error('Failed to read profile'));
		request.onsuccess = () => resolve((request.result as Identity | undefined) ?? null);
	});
}

export async function writeDeviceProfile(profile: Identity): Promise<void> {
	if (typeof indexedDB === 'undefined') {
		throw new Error('Local database is not available in this environment');
	}

	const existing = await readDeviceProfile();
	if (existing && existing.signing_pubkey !== profile.signing_pubkey) {
		throw new Error('A profile already exists on this device');
	}

	const db = await openDb();
	return new Promise((resolve, reject) => {
		const tx = db.transaction(STORE, 'readwrite');
		const request = tx.objectStore(STORE).put(profile, DEVICE_KEY);
		request.onerror = () => reject(request.error ?? new Error('Failed to save profile'));
		request.onsuccess = () => resolve();
	});
}

export async function clearDeviceProfile(): Promise<void> {
	if (typeof indexedDB === 'undefined') return;

	const db = await openDb();
	return new Promise((resolve, reject) => {
		const tx = db.transaction(STORE, 'readwrite');
		const request = tx.objectStore(STORE).delete(DEVICE_KEY);
		request.onerror = () => reject(request.error ?? new Error('Failed to clear profile'));
		request.onsuccess = () => resolve();
	});
}

export async function hasDeviceProfile(): Promise<boolean> {
	const profile = await readDeviceProfile();
	return profile !== null && Boolean(profile.display_name);
}
