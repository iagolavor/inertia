const INVITE_SCHEME = 'inertia://invite/';
const WEB_INVITE_PREFIX = '/invite#';

/** Strip http(s) / inertia:// wrappers so the API always gets the base64 payload. */
export function normalizeInviteInput(raw: string): string {
	const trimmed = raw.trim();
	if (!trimmed) return trimmed;

	if (trimmed.startsWith(INVITE_SCHEME)) {
		return trimmed.slice(INVITE_SCHEME.length);
	}

	const webIdx = trimmed.indexOf(WEB_INVITE_PREFIX);
	if (webIdx !== -1) {
		return trimmed.slice(webIdx + WEB_INVITE_PREFIX.length);
	}

	const hashIdx = trimmed.indexOf('#');
	if (hashIdx !== -1) {
		const fragment = trimmed.slice(hashIdx + 1);
		if (fragment) return fragment;
	}

	const query = trimmed.split('?')[1];
	if (query) {
		for (const part of query.split('&')) {
			if (part.startsWith('d=')) {
				return decodeURIComponent(part.slice(2));
			}
		}
	}

	return trimmed;
}
