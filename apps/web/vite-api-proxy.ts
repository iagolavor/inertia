import type { Logger, ProxyOptions } from 'vite';

export const INERTIA_API_TARGET = 'http://127.0.0.1:4783';
export const INERTIA_API_START_CMD = 'npm run api:release';

/** Cooldown so 5s presence polling does not flood the dev terminal. */
const LOG_COOLDOWN_MS = 15_000;

let lastOfflineLogAt = 0;
let apiWasOffline = false;

export function formatApiProxyError(
	err: NodeJS.ErrnoException,
	requestUrl?: string
): string {
	const path = requestUrl?.split('?')[0] ?? '/api';
	const code = err.code ?? 'PROXY_ERROR';

	if (code === 'ECONNREFUSED' || code === 'ECONNRESET' || code === 'ETIMEDOUT') {
		return [
			`[inertia] API offline — cannot reach ${INERTIA_API_TARGET}`,
			`           request: ${path}`,
			`           fix:     ${INERTIA_API_START_CMD}`
		].join('\n');
	}

	return `[inertia] API proxy error on ${path} (${code}: ${err.message})`;
}

function logOffline(err: NodeJS.ErrnoException, requestUrl?: string) {
	const now = Date.now();
	if (now - lastOfflineLogAt < LOG_COOLDOWN_MS) return;
	lastOfflineLogAt = now;
	apiWasOffline = true;
	console.warn(formatApiProxyError(err, requestUrl));
}

function logReconnected() {
	if (!apiWasOffline) return;
	apiWasOffline = false;
	lastOfflineLogAt = 0;
	console.warn(`[inertia] API connected — ${INERTIA_API_TARGET}`);
}

/** Dev/preview proxy for `/api` → local inertia-api with readable errors. */
export function inertiaApiProxyOptions(): ProxyOptions {
	return {
		target: INERTIA_API_TARGET,
		changeOrigin: true,
		rewrite: (path: string) => path.replace(/^\/api/, ''),
		timeout: 120_000,
		proxyTimeout: 120_000,
		configure(proxy) {
			proxy.on('proxyRes', () => {
				logReconnected();
			});

			proxy.on('error', (err, req, res) => {
				const nodeErr = err as NodeJS.ErrnoException;
				logOffline(nodeErr, req.url);

				if (!('writeHead' in res) || res.headersSent || res.writableEnded) {
					return;
				}

				res.writeHead(503, { 'Content-Type': 'application/json' });
				res.end(
					JSON.stringify({
						error: 'Local API is offline — start it with: npm run api:release',
						code: 'API_OFFLINE'
					})
				);
			});
		}
	};
}

/** Hide Vite's default multi-line proxy stack traces for our API target. */
export function createInertiaDevLogger(base: Logger): Logger {
	return {
		...base,
		error(msg, options) {
			if (isInertiaApiProxyNoise(msg)) return;
			base.error(msg, options);
		},
		warn(msg, options) {
			if (isInertiaApiProxyNoise(msg)) return;
			base.warn(msg, options);
		}
	};
}

function isInertiaApiProxyNoise(msg: string): boolean {
	if (!msg.includes('http proxy error') && !msg.includes('proxy error')) {
		return false;
	}
	return msg.includes('4783') || msg.includes('127.0.0.1');
}
