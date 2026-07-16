#!/usr/bin/env bash
# Forward phone localhost:4783 -> PC API (USB; UI on phone, API on PC). Re-run after unplugging the cable.
# Usage:
#   npm run android:reverse
#   npm run android:reverse -- --remove

set -euo pipefail

# shellcheck source=lib/android-env.sh
source "$(dirname "$0")/lib/android-env.sh"

adb="$(inertia_android_adb)"
inertia_require_adb

remove=0
serial=""

for arg in "$@"; do
	case "$arg" in
		--remove) remove=1 ;;
		*) serial="$arg" ;;
	esac
done

adb_quiet() {
	"$adb" "$@" >/dev/null 2>&1 || true
}

remove_reverse() {
	if [[ -n "$serial" ]]; then
		adb_quiet -s "$serial" reverse --remove tcp:4783
		adb_quiet -s "$serial" forward --remove tcp:4783
	else
		adb_quiet reverse --remove tcp:4783
		adb_quiet forward --remove tcp:4783
	fi
}

if [[ "$remove" -eq 1 ]]; then
	remove_reverse
	echo 'Cleared adb reverse/forward on tcp:4783 (if any were set).'
elif [[ -n "$serial" ]]; then
	"$adb" -s "$serial" reverse tcp:4783 tcp:4783
else
	"$adb" reverse tcp:4783 tcp:4783
fi

echo 'Active reverse forwards:'
"$adb" reverse --list
