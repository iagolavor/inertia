#!/usr/bin/env bash
# Opens SSH to your inertia relay VPS.
# Usage:
#   ./scripts/vps-ssh.sh
#   INERTIA_VPS_HOST=203.0.113.10 INERTIA_VPS_USER=ubuntu ./scripts/vps-ssh.sh
#
# Set defaults in .env (gitignored):
#   INERTIA_VPS_HOST=your.vps.ip
#   INERTIA_VPS_USER=ubuntu

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
env_file="$repo_root/.env"

if [[ -f "$env_file" ]]; then
  set -a
  # shellcheck disable=SC1090
  source "$env_file"
  set +a
fi

host="${INERTIA_VPS_HOST:-}"
user="${INERTIA_VPS_USER:-ubuntu}"
port="${INERTIA_VPS_PORT:-22}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -Host|--host) host="$2"; shift 2 ;;
    -User|--user) user="$2"; shift 2 ;;
    -Port|--port) port="$2"; shift 2 ;;
    -h|--help)
      echo "Usage: $0 [-Host IP] [-User name] [-Port 22]"
      exit 0
      ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

if [[ -z "$host" ]]; then
  cat >&2 <<'EOF'
Missing VPS host. Either:

  1. Add to .env in the repo root:
       INERTIA_VPS_HOST=your.vps.ip
       INERTIA_VPS_USER=ubuntu

  2. Or export INERTIA_VPS_HOST before running.
EOF
  exit 1
fi

echo "Connecting to ${user}@${host} ..."
exec ssh -p "$port" "${user}@${host}"
