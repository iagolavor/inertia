import { Capacitor } from '@capacitor/core';

interface InertiaDownloadBridge {
	enqueue: (url: string, fileName: string) => void;
	saveBase64?: (fileName: string, mimeType: string, dataBase64: string) => void;
}

function nativeDownloadBridge(): InertiaDownloadBridge | null {
	const bridge = (window as unknown as { InertiaDownload?: InertiaDownloadBridge })
		.InertiaDownload;
	return bridge?.enqueue ? bridge : null;
}

/** Stage B serves UI from the on-device API; Capacitor may still report "web". */
function isStageBShell(): boolean {
	if (typeof window === 'undefined') return false;
	const { hostname, port } = window.location;
	return hostname === '127.0.0.1' && port === '4783';
}

function isNativeDownloadContext(): boolean {
	return Capacitor.isNativePlatform() || isStageBShell();
}

/** Save a file served by inertia-api (uses Content-Disposition when provided). */
export async function saveFileFromApi(url: string, filename: string): Promise<void> {
	if (url.startsWith('blob:')) {
		throw new Error('Invalid download URL');
	}
	const name = filename.trim() || 'download';
	const bridge = nativeDownloadBridge();

	// Bridge is registered on Android even when Capacitor.isNativePlatform() is false (Stage B).
	if (bridge) {
		bridge.enqueue(url, name);
		return;
	}

	if (isNativeDownloadContext()) {
		throw new Error(
			'Download bridge unavailable. Reinstall the app (npm run android:stage-b).'
		);
	}

	const res = await fetch(url);
	if (!res.ok) {
		throw new Error('File not available locally yet');
	}
	const blob = await res.blob();
	const objectUrl = URL.createObjectURL(blob);
	try {
		const anchor = document.createElement('a');
		anchor.href = objectUrl;
		anchor.download = name;
		anchor.rel = 'noopener';
		anchor.style.display = 'none';
		document.body.appendChild(anchor);
		anchor.click();
		anchor.remove();
	} finally {
		URL.revokeObjectURL(objectUrl);
	}
}
