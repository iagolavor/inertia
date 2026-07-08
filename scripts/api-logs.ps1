$ErrorActionPreference = "Stop"

# Cross-platform entrypoint lives in package.json:
#   npm run api:logs
#
# Defaults:
# - If RUST_LOG is already set by the caller, keep it.
# - Otherwise set a useful verbose filter for local debugging.

if (-not $env:RUST_LOG -or $env:RUST_LOG.Trim() -eq "") {
  $env:RUST_LOG = "inertia_api=debug,inertia_core=info,inertia_relay=info"
}

cargo run -p inertia-api

