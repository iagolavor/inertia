#!/usr/bin/env bash
# Cross-compile inertia-api for Android arm64 (on-device install).
# Requires: Rust, Android SDK/NDK (npm run android:sdk), cargo-ndk.

set -euo pipefail

# shellcheck source=lib/android-env.sh
source "$(dirname "$0")/lib/android-env.sh"

root="$(inertia_repo_root)"
out_dir="$root/dist/android-arm64"
out_bin="$out_dir/inertia-api"
built_bin="$root/target/aarch64-linux-android/release/inertia-api"
android_api=24

sdk="$(inertia_android_sdk_root)"
if [[ ! -d "$sdk" ]]; then
	echo "Android SDK not found at $sdk — run: npm run android:sdk" >&2
	exit 1
fi

if [[ ! -d "$sdk/ndk" ]]; then
	echo "Android NDK not found under $sdk/ndk — install via Android Studio SDK Manager" >&2
	exit 1
fi

if ! command -v cargo-ndk >/dev/null 2>&1; then
	echo 'Installing cargo-ndk...'
	cargo install cargo-ndk --locked
fi

echo 'Adding Rust target aarch64-linux-android (if missing)...'
rustup target add aarch64-linux-android >/dev/null

echo "Building inertia-api for arm64-v8a (API $android_api)..."
(
	cd "$root"
	cargo ndk -t arm64-v8a -P "$android_api" build --release --bin inertia-api -p inertia-api
)

if [[ ! -f "$built_bin" ]]; then
	echo "Expected binary at $built_bin" >&2
	exit 1
fi

mkdir -p "$out_dir"
cp -f "$built_bin" "$out_bin"
echo "Built $out_bin"
