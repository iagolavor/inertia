import type { CapacitorConfig } from '@capacitor/cli';

const config: CapacitorConfig = {
	appId: 'social.inertia.app',
	appName: 'Inertia',
	webDir: 'build',
	server: {
		// Allow navigation to the on-device API origin in the WebView.
		androidScheme: 'http',
		// Without this Capacitor opens Chrome for 127.0.0.1.
		allowNavigation: ['127.0.0.1', 'localhost']
	}
};

export default config;
