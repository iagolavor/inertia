const VIDEO_DEBUG_KEY = 'inertia:video:debug';

export type VideoDebugLine = {
	id: number;
	ms: number;
	step: string;
	detail: Record<string, unknown>;
};

export const videoDebugLog = $state({
	open: true,
	lines: [] as VideoDebugLine[]
});

let sessionStart = 0;
let nextLineId = 0;

export function isVideoDebugEnabled(): boolean {
	if (import.meta.env.DEV) return true;
	try {
		return localStorage.getItem(VIDEO_DEBUG_KEY) === '1';
	} catch {
		return false;
	}
}

export function setVideoDebugEnabled(enabled: boolean): void {
	try {
		if (enabled) localStorage.setItem(VIDEO_DEBUG_KEY, '1');
		else localStorage.removeItem(VIDEO_DEBUG_KEY);
	} catch {
		// ignore private mode
	}
}

export function videoDebugElapsedMs(): number {
	if (!sessionStart) sessionStart = performance.now();
	return Math.round(performance.now() - sessionStart);
}

export function resetVideoDebugSession(): void {
	sessionStart = performance.now();
	videoDebugLog.lines = [];
}

export function appendVideoDebugLine(step: string, detail: Record<string, unknown> = {}): void {
	if (!isVideoDebugEnabled()) return;
	videoDebugLog.lines = [
		...videoDebugLog.lines.slice(-100),
		{ id: ++nextLineId, ms: videoDebugElapsedMs(), step, detail }
	];
}

export function clearVideoDebugLog(): void {
	videoDebugLog.lines = [];
}

export function formatVideoDebugDetail(detail: Record<string, unknown>): string {
	if (Object.keys(detail).length === 0) return '';
	try {
		return JSON.stringify(detail, null, 0);
	} catch {
		return String(detail);
	}
}
