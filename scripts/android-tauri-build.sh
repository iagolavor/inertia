#!/usr/bin/env bash
# Build Tauri Android debug APK (arm64) after android:package.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root/apps/desktop"
export CI="${CI:-true}"
# Prefer aarch64 phone APK; matches on-device inertia-api jniLibs.
npx tauri android build --debug --apk --target aarch64 --ci "$@"
