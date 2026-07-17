#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { platform } from 'node:os';

const root = join(dirname(fileURLToPath(import.meta.url)), '../..');
const isWin = platform() === 'win32';
const desktop = join(root, 'apps/desktop');

function run(cmd, args, cwd = root, env = process.env) {
  const r = spawnSync(cmd, args, { stdio: 'inherit', cwd, shell: isWin, env });
  if (r.status !== 0) process.exit(r.status ?? 1);
}

/** Tauri expects sidecars under apps/desktop/src-tauri/target; ignore shared CARGO_TARGET_DIR. */
function tauriEnv({ forBuild = false } = {}) {
  const env = { ...process.env };
  delete env.CARGO_TARGET_DIR;
  // Fedora / rolling distros: linuxdeploy strip breaks AppImage; ARCH needed by appimagetool.
  if (forBuild && !isWin && platform() === 'linux') {
    env.NO_STRIP = env.NO_STRIP || 'true';
    if (!env.ARCH) {
      const map = { x64: 'x86_64', arm64: 'aarch64' };
      env.ARCH = map[process.arch] || process.arch;
    }
  }
  return env;
}

function packageDesktop(debugApi = false) {
  if (isWin) {
    const args = [
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      join(root, 'scripts/package-desktop.ps1')
    ];
    if (debugApi) args.push('-DebugApi');
    run('powershell', args);
  } else {
    const sh = join(root, 'scripts/package-desktop.sh');
    run('bash', debugApi ? [sh, '--debug-api'] : [sh]);
  }
}

/** e.g. --bundles nsis or DESKTOP_BUNDLES=rpm,appimage */
function parseBundles(argv) {
  const i = argv.indexOf('--bundles');
  if (i >= 0 && argv[i + 1]) return argv[i + 1];
  if (process.env.DESKTOP_BUNDLES) return process.env.DESKTOP_BUNDLES;
  return null;
}

const cmd = process.argv[2];
const debugApi = process.argv.includes('--debug-api');
const bundles = parseBundles(process.argv);

if (cmd === 'package') {
  packageDesktop(debugApi);
} else if (cmd === 'dev') {
  packageDesktop(debugApi);
  run('npm', ['install'], desktop);
  run('npm', ['run', 'tauri', '--', 'dev'], desktop, tauriEnv());
} else if (cmd === 'build') {
  packageDesktop(false);
  run('npm', ['install'], desktop);
  const tauriArgs = ['run', 'tauri', '--', 'build'];
  if (bundles) tauriArgs.push('--bundles', bundles);
  run('npm', tauriArgs, desktop, tauriEnv({ forBuild: true }));
} else {
  console.error(
    'Usage: node scripts/run-native/desktop.mjs package|dev|build [--debug-api] [--bundles nsis|rpm,appimage|...]'
  );
  process.exit(1);
}
