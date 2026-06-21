import { Capacitor } from '@capacitor/core';

function trimBase(url: string): string {
	return url.replace(/\/$/, '');
}

/** HTTP origin + `/api` prefix for inertia-api. Web dev uses the Vite proxy (`/api`). */
export function getApiBase(): string {
	if (Capacitor.isNativePlatform()) {
		const fromEnv = import.meta.env.VITE_INERTIA_API_BASE;
		if (typeof fromEnv === 'string' && fromEnv.trim()) {
			const base = trimBase(fromEnv.trim());
			// 10.0.2.2 is emulator-only and breaks physical devices if baked in at build time.
			// USB Stage A uses adb reverse → 127.0.0.1 on both emulator and phone.
			if (!base.includes('10.0.2.2')) {
				return base;
			}
		}
		return 'http://127.0.0.1:4783/api';
	}
	return '/api';
}
