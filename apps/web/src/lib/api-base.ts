const DEVICE_API_BASE = 'http://127.0.0.1:4783/api';

function trimBase(url: string): string {
	return url.replace(/\/$/, '');
}

/** Tauri / WebView shell origins before navigation to the on-device API. */
function isNativeShellOrigin(): boolean {
	if (typeof window === 'undefined') return false;
	const { hostname, port, protocol } = window.location;
	if (port === '4783') return false;
	if (protocol === 'tauri:' || protocol === 'asset:' || protocol === 'https:') {
		// Tauri 2 Android often serves splash as https://tauri.localhost
		if (hostname === 'tauri.localhost' || hostname.endsWith('.tauri.localhost')) {
			return true;
		}
	}
	if (protocol === 'tauri:' || protocol === 'asset:') return true;
	return hostname === 'localhost' || hostname.endsWith('.localhost');
}

function isApiHostedOrigin(): boolean {
	if (typeof window === 'undefined') return false;
	const { hostname, port } = window.location;
	return hostname === '127.0.0.1' && port === '4783';
}

function isTauriRuntime(): boolean {
	if (typeof window === 'undefined') return false;
	const w = window as unknown as { __TAURI_INTERNALS__?: unknown; __TAURI__?: unknown };
	return w.__TAURI_INTERNALS__ != null || w.__TAURI__ != null;
}

function ensureApiSuffix(base: string): string {
	const trimmed = trimBase(base);
	return trimmed.endsWith('/api') ? trimmed : `${trimmed}/api`;
}

function resolveNativeApiBase(): string {
	const fromEnv = import.meta.env.VITE_INERTIA_API_BASE;
	if (typeof fromEnv === 'string' && fromEnv.trim()) {
		const base = trimBase(fromEnv.trim());
		// 10.0.2.2 is emulator-only and breaks physical devices if baked in at build time.
		if (!base.includes('10.0.2.2')) {
			return ensureApiSuffix(base);
		}
	}
	return DEVICE_API_BASE;
}

/** HTTP origin + `/api` prefix for inertia-api. Web dev uses the Vite proxy (`/api`). */
export function getApiBase(): string {
	if (isTauriRuntime() || isNativeShellOrigin() || isApiHostedOrigin()) {
		return resolveNativeApiBase();
	}
	return '/api';
}
