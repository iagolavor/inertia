import { Capacitor } from '@capacitor/core';

function trimBase(url: string): string {
	return url.replace(/\/$/, '');
}

/** HTTP origin + `/api` prefix for inertia-api. Web dev uses the Vite proxy (`/api`). */
export function getApiBase(): string {
	const fromEnv = import.meta.env.VITE_INERTIA_API_BASE;
	if (typeof fromEnv === 'string' && fromEnv.trim()) {
		return trimBase(fromEnv.trim());
	}
	if (Capacitor.isNativePlatform()) {
		// Stage A: API on dev PC — `adb reverse tcp:4783 tcp:4783` or set VITE_INERTIA_API_BASE to your LAN IP at build time.
		return 'http://127.0.0.1:4783/api';
	}
	return '/api';
}
