#!/usr/bin/env bash
# Install Android SDK command-line tools + packages required by apps/web/android (Capacitor).
# Run once after installing Android Studio on Linux.

set -euo pipefail

# shellcheck source=lib/android-env.sh
source "$(dirname "$0")/lib/android-env.sh"

repo_root="$(inertia_repo_root)"
sdk_root="$(inertia_android_sdk_root)"
local_props="$repo_root/apps/web/android/local.properties"
cmdline_zip="${TMPDIR:-/tmp}/commandlinetools-linux.zip"
cmdline_url='https://dl.google.com/android/repository/commandlinetools-linux-13114758_latest.zip'

if [[ -z "${JAVA_HOME:-}" ]]; then
	JAVA_HOME="$(inertia_java_home)" || exit 1
fi

mkdir -p "$sdk_root"

if [[ ! -x "$sdk_root/cmdline-tools/latest/bin/sdkmanager" ]]; then
	echo 'Downloading Android command-line tools...'
	curl -fsSL "$cmdline_url" -o "$cmdline_zip"
	extract_root="${TMPDIR:-/tmp}/android-cmdline-tools"
	rm -rf "$extract_root"
	mkdir -p "$extract_root"
	unzip -q "$cmdline_zip" -d "$extract_root"
	rm -f "$cmdline_zip"
	mkdir -p "$sdk_root/cmdline-tools"
	rm -rf "$sdk_root/cmdline-tools/latest"
	mv "$extract_root/cmdline-tools" "$sdk_root/cmdline-tools/latest"
fi

export JAVA_HOME
export ANDROID_HOME="$sdk_root"
export ANDROID_SDK_ROOT="$sdk_root"
export PATH="$JAVA_HOME/bin:$sdk_root/platform-tools:$sdk_root/cmdline-tools/latest/bin:$PATH"

echo 'Accepting SDK licenses...'
yes | sdkmanager --licenses >/dev/null || true

echo 'Installing platform-tools, Android 35, build-tools 35.0.0, NDK 26.3...'
sdkmanager \
	'platform-tools' \
	'platforms;android-35' \
	'build-tools;35.0.0' \
	'ndk;26.3.11579264'

printf 'sdk.dir=%s\n' "$sdk_root" >"$local_props"

echo
echo "Done. SDK: $sdk_root"
echo "Wrote $local_props"
echo 'Add to ~/.bashrc if needed:'
echo "  export ANDROID_HOME=\"$sdk_root\""
echo "  export ANDROID_SDK_ROOT=\"$sdk_root\""
echo "  export JAVA_HOME=\"$JAVA_HOME\""
echo 'Then: npm run android:sync && npm run android:open'
