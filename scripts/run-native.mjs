#!/usr/bin/env node
/**
 * Run scripts/<name>.ps1 on Windows or scripts/<name>.sh elsewhere.
 * Built-ins: android-open (Capacitor open with auto-detected Android Studio).
 *
 * Usage: node scripts/run-native.mjs <name> [args...]
 */
import { execSync, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const name = process.argv[2];
const args = process.argv.slice(3);

if (!name) {
	console.error('Usage: node scripts/run-native.mjs <script-name> [args...]');
	process.exit(1);
}

function resolveAndroidStudio() {
	const override = process.env.CAPACITOR_ANDROID_STUDIO_PATH;
	if (override && existsSync(override)) {
		return override;
	}

	const candidates = [
		'/usr/local/android-studio/bin/studio.sh',
		join(homedir(), 'android-studio/bin/studio.sh'),
		'/opt/android-studio/bin/studio.sh',
	];

	if (process.platform === 'win32') {
		candidates.unshift('C:\\Program Files\\Android\\Android Studio\\bin\\studio64.exe');
	}

	for (const path of candidates) {
		if (existsSync(path)) {
			return path;
		}
	}

	if (process.platform !== 'win32') {
		try {
			const loc = execSync('flatpak info --show-location com.google.AndroidStudio', {
				encoding: 'utf8',
				stdio: ['ignore', 'pipe', 'ignore'],
			}).trim();
			const studio = join(loc, 'files/extra/bin/studio.sh');
			if (existsSync(studio)) {
				return studio;
			}
		} catch {
			// Flatpak not installed
		}
	}

	return null;
}

function androidStudioEnv() {
	const studio = resolveAndroidStudio();
	if (!studio) {
		return process.env;
	}
	return { ...process.env, CAPACITOR_ANDROID_STUDIO_PATH: studio };
}

function runAndroidOpen() {
	const web = join(root, 'apps/web');
	const env = androidStudioEnv();

	let result = spawnSync('npm', ['run', 'cap:sync'], { cwd: web, stdio: 'inherit', env });
	if (result.status) {
		process.exit(result.status);
	}

	result = spawnSync('npx', ['cap', 'open', 'android'], { cwd: web, stdio: 'inherit', env });
	process.exit(result.status ?? 1);
}

if (name === 'android-open') {
	runAndroidOpen();
}

const isWin = process.platform === 'win32';
const ext = isWin ? 'ps1' : 'sh';
const script = join(root, 'scripts', `${name}.${ext}`);

if (!existsSync(script)) {
	console.error(`Missing ${script}`);
	process.exit(1);
}

const result = isWin
	? spawnSync(
			'powershell',
			['-ExecutionPolicy', 'Bypass', '-File', script, ...args],
			{ stdio: 'inherit', cwd: root }
		)
	: spawnSync('bash', [script, ...args], { stdio: 'inherit', cwd: root });

process.exit(result.status ?? 1);
