#!/usr/bin/env node
/**
 * Set apps/desktop package / Tauri / Cargo versions from a release tag.
 * Usage: node scripts/sync-desktop-version.mjs 0.16.0
 *        node scripts/sync-desktop-version.mjs v0.16.0
 *
 * Only rewrites the version field (preserves surrounding file formatting).
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const raw = process.argv[2];
if (!raw) {
  console.error('Usage: node scripts/sync-desktop-version.mjs <version>');
  process.exit(1);
}

const version = raw.replace(/^v/, '');
if (!/^\d+\.\d+\.\d+([.-][\w.]+)?$/.test(version)) {
  console.error(`Invalid semver-ish version: ${raw}`);
  process.exit(1);
}

function patchFile(relPath, pattern) {
  const path = join(root, relPath);
  const src = readFileSync(path, 'utf8');
  if (!pattern.test(src)) {
    console.error(`version field not found in ${relPath}`);
    process.exit(1);
  }
  // Reset lastIndex for global-safe reuse; patterns here are non-global.
  pattern.lastIndex = 0;
  const out = src.replace(pattern, (match, prefix, _old, suffix) => `${prefix}${version}${suffix}`);
  writeFileSync(path, out);
  console.log(`updated ${relPath} -> ${version}`);
}

// "version": "x.y.z" in package.json / tauri.conf.json
const jsonVersion = /("version"\s*:\s*")([^"]+)(")/;
// version = "x.y.z" at package root in Cargo.toml (first occurrence)
const cargoVersion = /^(version\s*=\s*")([^"]+)(")/m;

patchFile('apps/desktop/package.json', jsonVersion);
patchFile('apps/desktop/src-tauri/tauri.conf.json', jsonVersion);
patchFile('apps/desktop/src-tauri/Cargo.toml', cargoVersion);
