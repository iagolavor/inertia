#!/usr/bin/env node
/**
 * Set Android versionName / versionCode from a release tag.
 * Usage: node scripts/sync-android-version.mjs 0.15.3
 *        node scripts/sync-android-version.mjs v0.15.3
 *
 * versionCode = major*10000 + minor*100 + patch (e.g. 0.15.3 -> 1503).
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

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

const gradlePath = join(root, 'apps/web/android/app/build.gradle');
let src = readFileSync(gradlePath, 'utf8');

if (!/versionCode\s+\d+/.test(src) || !/versionName\s+"[^"]+"/.test(src)) {
  console.error('versionCode / versionName not found in apps/web/android/app/build.gradle');
  process.exit(1);
}

src = src.replace(/versionCode\s+\d+/, `versionCode ${versionCode}`);
src = src.replace(/versionName\s+"[^"]+"/, `versionName "${version}"`);
writeFileSync(gradlePath, src);
console.log(`updated android versionName=${version} versionCode=${versionCode}`);
