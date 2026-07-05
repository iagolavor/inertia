import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { handleP2pStreamTransportError } = vi.hoisted(() => ({
	handleP2pStreamTransportError: vi.fn()
}));

vi.mock('$lib/presence.svelte', () => ({
	handleP2pStreamTransportError,
	handleP2pUiEvent: vi.fn(),
	refreshMessagesOnVisible: vi.fn(),
	refreshP2pLive: vi.fn()
}));

import { startP2pEventStream, stopP2pEventStream } from './p2p-events.svelte';

class MockEventSource {
	static instances: MockEventSource[] = [];
	onopen: (() => void) | null = null;
	onmessage: ((event: MessageEvent) => void) | null = null;
	onerror: (() => void) | null = null;
	closed = false;

	constructor(public url: string) {
		MockEventSource.instances.push(this);
	}

	close() {
		this.closed = true;
	}

	emitError() {
		this.onerror?.();
	}
}

describe('p2p-events stream', () => {
	beforeEach(() => {
		MockEventSource.instances = [];
		vi.stubGlobal('EventSource', MockEventSource);
		handleP2pStreamTransportError.mockReset();
	});

	afterEach(() => {
		stopP2pEventStream();
		vi.unstubAllGlobals();
	});

	it('closes EventSource and probes transport on error', () => {
		startP2pEventStream();
		const instance = MockEventSource.instances[0];
		expect(instance).toBeDefined();

		instance.emitError();

		expect(instance.closed).toBe(true);
		expect(handleP2pStreamTransportError).toHaveBeenCalledTimes(1);
		expect(typeof handleP2pStreamTransportError.mock.calls[0][0]).toBe('function');
	});
});
