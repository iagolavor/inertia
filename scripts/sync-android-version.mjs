#!/usr/bin/env node
/**
 * Set Android versionName / versionCode from a release tag (Tauri Android gradle).
 * Usage: node scripts/sync-android-version.mjs 0.15.3
 *        node scripts/sync-android-version.mjs v0.15.3
 *
 * Also syncs apps/desktop Tauri/Cargo/package versions (same tag).
 * versionCode = major*10000 + minor*100 + patch (e.g. 0.15.3 -> 1503).
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const raw = process.argv[2];
if (!raw) {
  console.error('Usage: node scripts/sync-android-version.mjs <version>');
  process.exit(1);
}

const version = raw.replace(/^v/, '');
const m = version.match(/^(\d+)\.(\d+)\.(\d+)(?:[.-].*)?$/);
if (!m) {
  console.error(`Invalid semver-ish version: ${raw}`);
  process.exit(1);
}

const major = Number(m[1]);
const minor = Number(m[2]);
const patch = Number(m[3]);
const versionCode = major * 10000 + minor * 100 + patch;
if (!Number.isFinite(versionCode) || versionCode < 1) {
  console.error(`Invalid versionCode derived from ${version}`);
  process.exit(1);
}

// Keep desktop package version in lockstep for the shared Tauri shell.
const syncDesktop = spawnSync(
  process.execPath,
  [join(root, 'scripts/sync-desktop-version.mjs'), version],
  { stdio: 'inherit' }
);
if (syncDesktop.status) {
  process.exit(syncDesktop.status ?? 1);
}

const gradlePath = join(
  root,
  'apps/desktop/src-tauri/gen/android/app/build.gradle.kts'
);
let src = readFileSync(gradlePath, 'utf8');

// Prefer writing defaults used when tauri.properties is absent at sync time.
if (!/versionCode\s*=/.test(src) || !/versionName\s*=/.test(src)) {
  console.error('versionCode / versionName not found in Tauri Android build.gradle.kts');
  process.exit(1);
}

src = src.replace(
  /versionCode\s*=\s*tauriProperties\.getProperty\("tauri\.android\.versionCode",\s*"[^"]*"\)\.toInt\(\)/,
  `versionCode = tauriProperties.getProperty("tauri.android.versionCode", "${versionCode}").toInt()`
);
src = src.replace(
  /versionName\s*=\s*tauriProperties\.getProperty\("tauri\.android\.versionName",\s*"[^"]*"\)/,
  `versionName = tauriProperties.getProperty("tauri.android.versionName", "${version}")`
);
writeFileSync(gradlePath, src);
console.log(`updated android versionName=${version} versionCode=${versionCode}`);
