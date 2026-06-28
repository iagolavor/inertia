#!/usr/bin/env bash
# Build, sync, and install the Android app on a connected device (no interactive picker).
# Usage:
#   npm run android:run:device
#   ANDROID_SERIAL=DEVICE npm run android:run:device
#   bash scripts/android-run-device.sh --no-sync DEVICE_SERIAL

set -euo pipefail

# shellcheck source=lib/android-env.sh
source "$(dirname "$0")/lib/android-env.sh"

root="$(inertia_repo_root)"
web="$root/apps/web"
adb="$(inertia_android_adb)"
inertia_require_adb

target="${ANDROID_SERIAL:-}"
no_sync=0

for arg in "$@"; do
	case "$arg" in
		--no-sync) no_sync=1 ;;
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

echo "Installing on $target ..."

cap_args=(run android --target "$target")
if [[ "$no_sync" -eq 1 ]]; then
	cap_args+=(--no-sync)
fi

(cd "$web" && npx cap "${cap_args[@]}")
