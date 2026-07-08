#!/usr/bin/env bash
set -euo pipefail

# Cross-platform entrypoint lives in package.json:
#   npm run api:logs
#
# Defaults:
# - If RUST_LOG is already set by the caller, keep it.
# - Otherwise set a useful verbose filter for local debugging.

if [[ -z "${RUST_LOG:-}" ]]; then
  export RUST_LOG="inertia_api=debug,inertia_core=info,inertia_relay=info"
fi

exec cargo run -p inertia-api

