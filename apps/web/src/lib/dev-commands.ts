/** npm scripts from repo root — lighter than raw cargo/vite invocations. */
export const DEV_COMMANDS = {
	apiRelease: 'npm run api:release',
	apiWindow: 'npm run api:window',
	webPreview: 'npm run web:preview',
	webBuildPreview: 'npm run web:build && npm run web:preview',
	webDev: 'npm run web'
} as const;

export type WebUiMode = 'dev' | 'preview' | 'other';

export function webUiMode(): WebUiMode {
	if (typeof window === 'undefined') return 'other';
	const port = window.location.port;
	if (port === '5173') return 'dev';
	if (port === '4173') return 'preview';
	return 'other';
}

export function suggestedWebCommand(mode: WebUiMode = webUiMode()): string {
	if (mode === 'dev') return DEV_COMMANDS.webDev;
	return DEV_COMMANDS.webBuildPreview;
}

export async function copyToClipboard(text: string): Promise<boolean> {
	try {
		await navigator.clipboard.writeText(text);
		return true;
	} catch {
		return false;
	}
}

export async function copyDevCommand(command: string): Promise<boolean> {
	return copyToClipboard(command);
}

/** Short user-facing copy — commands live on action buttons, not in error strings. */
export const API_OFFLINE_HINT =
	'The local API bridge is not running. Use the buttons below, then Retry.';

export const API_TIMEOUT_HINT =
	'The local API is not responding. Restart it from the project folder, then Retry.';

export const API_DISCONNECTED_HINT =
	'API disconnected — click Retry or use Start API to copy the command.';
