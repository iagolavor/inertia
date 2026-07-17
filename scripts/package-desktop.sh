#!/usr/bin/env bash
# Build inertia-api + web UI into Tauri sidecar/resources for apps/desktop.
# Usage: ./scripts/package-desktop.sh [--debug-api]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TRIPLE="$(rustc --print host-tuple)"
EXT=""
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*|Windows_NT) EXT=".exe" ;;
esac

API_PROFILE=release
CARGO_FLAGS=(--release)
if [[ "${1:-}" == "--debug-api" ]]; then
  API_PROFILE=debug
  CARGO_FLAGS=()
fi

echo "==> cargo build -p inertia-api (${API_PROFILE})"
(cd "$ROOT" && cargo build "${CARGO_FLAGS[@]}" -p inertia-api)

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
SRC_API="$TARGET_DIR/${API_PROFILE}/inertia-api${EXT}"
if [[ ! -f "$SRC_API" ]]; then
  echo "missing $SRC_API" >&2
  exit 1
fi

BIN_DIR="$ROOT/apps/desktop/src-tauri/binaries"
RES_WEB="$ROOT/apps/desktop/src-tauri/resources/web"
mkdir -p "$BIN_DIR" "$ROOT/apps/desktop/src-tauri/resources"

DEST_API="$BIN_DIR/inertia-api-${TRIPLE}${EXT}"
echo "==> sidecar $DEST_API"
cp -f "$SRC_API" "$DEST_API"
chmod +x "$DEST_API" 2>/dev/null || true

echo "==> web:build"
(cd "$ROOT" && npm run web:build)

echo "==> resources/web"
rm -rf "$RES_WEB"
mkdir -p "$RES_WEB"
cp -a "$ROOT/apps/web/build/." "$RES_WEB/"
touch "$RES_WEB/.gitkeep"

echo "Desktop package ready:"
echo "  $DEST_API"
echo "  $RES_WEB"
