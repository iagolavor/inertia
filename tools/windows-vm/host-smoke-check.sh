#!/usr/bin/env bash
# Host-side checks before / during Windows guest Tauri smoke.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

ok=0
fail=0
check() {
  local name="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    echo "OK  $name"
    ok=$((ok + 1))
  else
    echo "FAIL $name"
    fail=$((fail + 1))
  fi
}

check "compose file" test -f docker-compose.yml
check "guest-build.ps1" test -f guest-build.ps1
check "guest-setup.ps1" test -f guest-setup.ps1
check "container running" bash -c "docker inspect -f '{{.State.Running}}' inertia-windows11 2>/dev/null | grep -qx true"
check "/shared/package.json in container" docker exec inertia-windows11 test -f /shared/package.json
check "samba Data share config" docker exec inertia-windows11 grep -q 'path = /shared' /etc/samba/smb.conf
check "smbd running" docker exec inertia-windows11 sh -c 'pgrep -x smbd >/dev/null'
check "RDP port 3389" bash -c "ss -ltn | grep -q ':3389'"
check "web viewer 8006" curl -sf -o /dev/null http://127.0.0.1:8006/

echo ""
echo "Passed: $ok  Failed: $fail"
echo "Guest next steps (inside Windows as Docker/admin):"
echo "  1. Open Desktop -> Shared (or \\\\host.lan\\Data)"
echo "  2. powershell -ExecutionPolicy Bypass -File .\\tools\\windows-vm\\guest-setup.ps1"
echo "  3. New PowerShell window, then:"
echo "     powershell -ExecutionPolicy Bypass -File .\\tools\\windows-vm\\guest-build.ps1"
exit "$fail"
