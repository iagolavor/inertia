#!/usr/bin/env bash
# Build, package, and install the Tauri Android app on a connected device.
# Usage:
#   npm run android:run:device
#   ANDROID_SERIAL=DEVICE npm run android:run:device

set -euo pipefail

# shellcheck source=lib/android-env.sh
source "$(dirname "$0")/lib/android-env.sh"

root="$(inertia_repo_root)"
adb="$(inertia_android_adb)"
inertia_require_adb

target="${ANDROID_SERIAL:-}"

for arg in "$@"; do
	case "$arg" in
		*) target="$arg" ;;
	esac
done

mapfile -t ids < <("$adb" devices | awk 'NR>1 && $2=="device" { print $1 }')

if [[ ${#ids[@]} -eq 0 ]]; then
	echo 'No Android device connected. Plug in the phone and enable USB debugging.' >&2
	exit 1
fi

if [[ -z "$target" ]]; then
	if [[ ${#ids[@]} -gt 1 ]]; then
		echo "Multiple devices: ${ids[*]}. Set ANDROID_SERIAL or pass a serial argument." >&2
		exit 1
	fi
	target="${ids[0]}"
else
	found=0
	for id in "${ids[@]}"; do
		if [[ "$id" == "$target" ]]; then
			found=1
			break
		fi
	done
	if [[ "$found" -eq 0 ]]; then
		echo "Target $target not connected. Available: ${ids[*]}" >&2
		exit 1
	fi
fi

echo "Building and installing on $target ..."
(
	cd "$root/apps/desktop"
	export CI=true
	npx tauri android run --no-watch "$target"
)
