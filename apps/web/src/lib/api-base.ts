import { Capacitor } from '@capacitor/core';

const DEVICE_API_BASE = 'http://127.0.0.1:4783/api';

function trimBase(url: string): string {
	return url.replace(/\/$/, '');
}

/** Capacitor shell (localhost) serves index.html for `/api/*`; on-device UI should use the device API. */
function isCapacitorShellOrigin(): boolean {
	if (typeof window === 'undefined') return false;
	const { hostname, port } = window.location;
	if (port === '4783') return false;
	return hostname === 'localhost' || hostname.endsWith('.localhost');
}

function isApiHostedOrigin(): boolean {
	if (typeof window === 'undefined') return false;
	const { hostname, port } = window.location;
	return hostname === '127.0.0.1' && port === '4783';
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
	if (Capacitor.isNativePlatform() || isCapacitorShellOrigin() || isApiHostedOrigin()) {
		return resolveNativeApiBase();
	}
	return '/api';
}