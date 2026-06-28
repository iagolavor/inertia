#!/usr/bin/env bash
# Stops any running inertia-api process (frees port 4783 for rebuild).

set -euo pipefail

if ! pgrep -f inertia-api >/dev/null 2>&1; then
	echo 'inertia-api is not running.'
	exit 0
fi

pkill -f inertia-api || true
sleep 1
echo 'Done.'
