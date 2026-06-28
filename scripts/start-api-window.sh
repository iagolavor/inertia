#!/usr/bin/env bash
# Opens inertia-api in a separate terminal window (logs stay visible).

set -euo pipefail

# shellcheck source=lib/android-env.sh
source "$(dirname "$0")/lib/android-env.sh"

root="$(inertia_repo_root)"
cmd="cd '$root' && export RUST_LOG=info && npm run api:release; exec bash"

if command -v gnome-terminal >/dev/null 2>&1; then
	exec gnome-terminal -- bash -lc "$cmd"
elif command -v konsole >/dev/null 2>&1; then
	exec konsole -e bash -lc "$cmd"
elif command -v xterm >/dev/null 2>&1; then
	exec xterm -e bash -lc "$cmd"
fi

echo 'No supported terminal emulator found. Run in another tab: npm run api:release' >&2
exit 1
