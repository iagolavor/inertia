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
function tauriEnv() {
  const env = { ...process.env };
  delete env.CARGO_TARGET_DIR;
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

const cmd = process.argv[2];
const debugApi = process.argv.includes('--debug-api');

if (cmd === 'package') {
  packageDesktop(debugApi);
} else if (cmd === 'dev') {
  packageDesktop(debugApi);
  run('npm', ['install'], desktop);
  run('npm', ['run', 'tauri', '--', 'dev'], desktop, tauriEnv());
} else if (cmd === 'build') {
  packageDesktop(false);
  run('npm', ['install'], desktop);
  run('npm', ['run', 'tauri', '--', 'build'], desktop, tauriEnv());
} else {
  console.error('Usage: node scripts/run-native/desktop.mjs package|dev|build [--debug-api]');
  process.exit(1);
}
