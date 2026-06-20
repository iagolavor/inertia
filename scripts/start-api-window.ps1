# Opens inertia-api in a separate PowerShell window (stays open with logs visible).
$root = Split-Path -Parent $PSScriptRoot
Start-Process powershell -ArgumentList @(
  '-NoExit',
  '-Command',
  "Set-Location '$root'; `$env:RUST_LOG='info'; npm run api:release"
)
