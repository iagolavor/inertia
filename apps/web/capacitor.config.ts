import type { CapacitorConfig } from '@capacitor/cli';

const config: CapacitorConfig = {
	appId: 'social.inertia.app',
	appName: 'Inertia',
	webDir: 'build',
	server: {
		// HTTP app origin so PC-API dev can call http://127.0.0.1:4783 (with adb reverse or 10.0.2.2).
		androidScheme: 'http',
		// On-device WebView loads the device API - without this Capacitor opens Chrome for 127.0.0.1.
		allowNavigation: ['127.0.0.1', 'localhost']
	}
};

export default config;
