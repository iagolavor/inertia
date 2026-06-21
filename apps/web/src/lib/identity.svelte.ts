import { api, type ApiErrorInfo, type Identity, type P2pStatus } from '$lib/api';
import { ApiRequestError } from '$lib/api-errors';
import { API_DISCONNECTED_HINT, API_OFFLINE_HINT } from '$lib/dev-commands';
import { clearDeviceProfile, readDeviceProfile, writeDeviceProfile } from '$lib/device-db';

export const identityState = $state({
	identity: null as Identity | null,
	apiOnline: false,
	loading: true,
	apiError: null as ApiErrorInfo | null,
	lastApiOkAt: null as string | null,
	p2pInfo: null as { peer_id: string | null; addresses: string[] } | null,
	p2pStatus: null as P2pStatus | null,
	profileLocked: false
});

let refreshCounter = 0;
let lastP2pStatusAt = 0;
const P2P_STATUS_MIN_INTERVAL_MS = 15_000;

/** Sync P2P state from the API; start only if boot auto-start failed or P2P stopped. */
async function ensureP2pRunning() {
	if (!identityState.identity?.display_name) return;

	try {
		const status = await api.p2pStatus();
		if (status.running && status.peer_id) {
			identityState.p2pInfo = {
				peer_id: status.peer_id,
				addresses: status.listen_addresses
			};
			identityState.p2pStatus = status;
			lastP2pStatusAt = Date.now();
			return;
		}

		const started = await api.startP2p();
		identityState.p2pInfo = started;
		identityState.p2pStatus = await api.p2pStatus();
		lastP2pStatusAt = Date.now();
	} catch {
		identityState.p2pInfo = null;
		identityState.p2pStatus = null;
	}
}

async function refreshP2pStatusIfDue() {
	const now = Date.now();
	if (now - lastP2pStatusAt < P2P_STATUS_MIN_INTERVAL_MS) return;
	if (!identityState.identity?.display_name || !identityState.p2pInfo?.peer_id) return;

	try {
		identityState.p2pStatus = await api.p2pStatus();
		lastP2pStatusAt = now;
		if (!identityState.p2pStatus.running) {
			await ensureP2pRunning();
		}
	} catch {
		identityState.p2pStatus = null;
	}
}

async function syncFromApi() {
	const identity = await api.getIdentity();
	if (identity.display_name) {
		identityState.identity = identity;
		identityState.profileLocked = true;
		await writeDeviceProfile(identity).catch(() => {});
		await ensureP2pRunning();
		await refreshP2pStatusIfDue();
		return;
	}

	identityState.identity = null;
	identityState.p2pInfo = null;
	identityState.p2pStatus = null;
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
		lastP2pStatusAt = 0;
	}

	try {
		await api.health();
		if (requestId !== refreshCounter) return;

		identityState.apiOnline = true;
		identityState.apiError = null;
		identityState.lastApiOkAt = new Date().toISOString();
		try {
			await syncFromApi();
		} catch (error) {
			if (requestId !== refreshCounter) return;
			if (error instanceof ApiRequestError) {
				identityState.apiError = { kind: error.kind, message: error.message };
			}
			await syncFromLocalDb();
		}
	} catch (error) {
		if (requestId !== refreshCounter) return;
		identityState.apiOnline = false;
		identityState.apiError =
			error instanceof ApiRequestError
				? { kind: error.kind, message: error.message }
				: { kind: 'offline', message: API_OFFLINE_HINT };
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
		identityState.apiError = {
			kind: 'offline',
			message: API_DISCONNECTED_HINT
		};
		identityState.p2pInfo = null;
		identityState.p2pStatus = null;
		await syncFromLocalDb();
	} finally {
		identityState.loading = false;
	}
}

/** Explicit P2P start (e.g. right after profile creation). */
export async function startP2pInBackground() {
	await ensureP2pRunning();
}
