import { sveltekit } from '@sveltejs/kit/vite';
import { createLogger, defineConfig } from 'vite';

import {
	createInertiaDevLogger,
	inertiaApiProxyOptions
} from './vite-api-proxy';

const apiProxy = inertiaApiProxyOptions();

export default defineConfig({
	customLogger: createInertiaDevLogger(createLogger()),
	plugins: [sveltekit()],
	server: {
		proxy: {
			'/api': apiProxy
		}
	},
	preview: {
		proxy: {
			'/api': apiProxy
		}
	}
});
