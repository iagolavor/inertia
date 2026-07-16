import {
	API_OFFLINE_HINT,
	API_TIMEOUT_HINT
} from '$lib/dev-commands';

export type ApiErrorKind = 'offline' | 'timeout' | 'server' | 'client';

export interface ApiErrorInfo {
	kind: ApiErrorKind;
	message: string;
	code?: string;
}

const PROXY_OFFLINE_RE =
	/internal server error|bad gateway|service unavailable|gateway timeout|econnrefused|connection refused/i;

export function classifyFetchFailure(error: unknown): ApiErrorInfo {
	if (error instanceof DOMException && error.name === 'AbortError') {
		return {
			kind: 'timeout',
			message: API_TIMEOUT_HINT
		};
	}
	if (error instanceof TypeError) {
		return {
			kind: 'offline',
			message: API_OFFLINE_HINT
		};
	}
	if (error instanceof Error) {
		if (PROXY_OFFLINE_RE.test(error.message)) {
			return {
				kind: 'offline',
				message: API_OFFLINE_HINT
			};
		}
		return { kind: 'server', message: error.message };
	}
	return { kind: 'server', message: 'Something went wrong talking to the API' };
}

export function classifyHttpFailure(
	status: number,
	rawMessage: string,
	code?: string
): ApiErrorInfo {
	const msg = rawMessage.trim() || `HTTP ${status}`;

	if (code === 'relay_unreachable' || code === 'p2p_not_started') {
		return { kind: 'server', message: msg, code };
	}
	if (code === 'inviter_offline' || code === 'friend_offline') {
		return { kind: 'client', message: msg, code };
	}
	if (status === 502 || status === 503 || status === 504 || PROXY_OFFLINE_RE.test(msg)) {
		return {
			kind: 'offline',
			message: API_OFFLINE_HINT
		};
	}
	if (status >= 500) {
		return {
			kind: 'server',
			message: `API error (${status}) — try Start API, then Retry`
		};
	}
	if (status === 408 || status === 429) {
		return { kind: 'timeout', message: API_TIMEOUT_HINT };
	}
	return { kind: 'client', message: msg, code };
}

export function toApiError(error: unknown): ApiErrorInfo {
	if (error instanceof Error && 'kind' in error) {
		const kind = (error as Error & { kind?: ApiErrorKind }).kind;
		if (kind) return { kind, message: error.message };
	}
	return classifyFetchFailure(error);
}

export class ApiRequestError extends Error {
	kind: ApiErrorKind;
	code?: string;

	constructor(info: ApiErrorInfo) {
		super(info.message);
		this.name = 'ApiRequestError';
		this.kind = info.kind;
		this.code = info.code;
	}
}
