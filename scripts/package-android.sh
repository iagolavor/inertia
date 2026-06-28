#!/usr/bin/env bash
# Stage B: package inertia-api (jniLibs) + web (assets). Mirrors Windows zip layout.

set -euo pipefail

# shellcheck source=lib/android-env.sh
source "$(dirname "$0")/lib/android-env.sh"

root="$(inertia_repo_root)"
api_bin="$root/dist/android-arm64/inertia-api"
web_build="$root/apps/web/build"
assets_root="$root/apps/web/android/app/src/main/assets/inertia"
jni_libs="$root/apps/web/android/app/src/main/jniLibs/arm64-v8a"
jni_lib_name='libinertia_api.so'

if [[ ! -f "$api_bin" ]]; then
	echo "Missing $api_bin — run: npm run android:api:build" >&2
	exit 1
fi

if [[ ! -f "$web_build/index.html" ]]; then
	echo 'Missing apps/web/build — run: npm run web:build' >&2
	exit 1
fi

rm -rf "$assets_root"
mkdir -p "$assets_root"
cp -a "$web_build" "$assets_root/web"

bundle_id="$(sha256sum "$web_build/index.html" | awk '{print $1}' | cut -c1-12)"
printf '%s' "$bundle_id" >"$assets_root/web/bundle-id"

mkdir -p "$jni_libs"
cp -f "$api_bin" "$jni_libs/$jni_lib_name"

echo "Packaged web assets at $assets_root"
echo "Packaged API binary at $jni_libs/$jni_lib_name"
echo 'Next: npm run android:sync (or rebuild/install from Android Studio)'
