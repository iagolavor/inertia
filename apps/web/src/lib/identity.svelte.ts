import { api, type Identity } from '$lib/api';
import { clearDeviceProfile, readDeviceProfile, writeDeviceProfile } from '$lib/device-db';

export const identityState = $state({
	identity: null as Identity | null,
	apiOnline: false,
	loading: true,
	p2pInfo: null as { peer_id: string | null; addresses: string[] } | null,
	profileLocked: false
});

let refreshCounter = 0;

async function syncFromApi() {
	const identity = await api.getIdentity();
	if (identity.display_name) {
		identityState.identity = identity;
		identityState.profileLocked = true;
		await writeDeviceProfile(identity).catch(() => {});
		try {
			const info = await Promise.race([
				api.p2pAddresses(),
				new Promise<null>((resolve) => setTimeout(() => resolve(null), 3_000))
			]);
			if (info?.peer_id) {
				identityState.p2pInfo = info;
			} else {
				identityState.p2pInfo = await api.startP2p();
			}
		} catch {
			identityState.p2pInfo = null;
		}
		return;
	}

	identityState.identity = null;
	identityState.p2pInfo = null;
	identityState.profileLocked = false;
	await clearDeviceProfile().catch(() => {});
}

async function syncFromLocalDb() {
	try {
		const cached = await readDeviceProfile();
		if (cached?.display_name) {
			identityState.identity = cached;
			identityState.profileLocked = true;
			return;
		}
	} catch {
		// ignore IndexedDB errors
	}

	identityState.identity = null;
	identityState.profileLocked = false;
}

export async function refreshIdentity(options: { silent?: boolean } = {}) {
	const { silent = false } = options;
	const requestId = ++refreshCounter;

	if (!silent) {
		identityState.loading = true;
	}

	try {
		await api.health();
		if (requestId !== refreshCounter) return;

		identityState.apiOnline = true;
		try {
			await syncFromApi();
		} catch {
			if (requestId !== refreshCounter) return;
			await syncFromLocalDb();
		}
	} catch {
		if (requestId !== refreshCounter) return;
		identityState.apiOnline = false;
		await syncFromLocalDb();
	} finally {
		if (requestId === refreshCounter) {
			identityState.loading = false;
		}
	}
}

export async function setIdentity(
	identity: Identity,
	p2pInfo?: { peer_id: string; addresses: string[] }
) {
	identityState.identity = identity;
	identityState.profileLocked = true;
	identityState.loading = false;
	if (p2pInfo) identityState.p2pInfo = p2pInfo;
	await writeDeviceProfile(identity).catch(() => {});
}

export async function toggleApiBridge() {
	if (identityState.loading) return;

	if (!identityState.apiOnline) {
		await refreshIdentity();
		return;
	}

	identityState.loading = true;
	try {
		try {
			await api.shutdownBridge();
		} catch {
			// Server may close the connection before the response completes.
		}
		identityState.apiOnline = false;
		identityState.p2pInfo = null;
		await syncFromLocalDb();
	} finally {
		identityState.loading = false;
	}
}

export async function startP2pInBackground() {
	try {
		identityState.p2pInfo = await api.startP2p();
	} catch {
		identityState.p2pInfo = null;
	}
}
