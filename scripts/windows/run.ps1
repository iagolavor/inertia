# Start prebuilt Inertia (API + UI on http://127.0.0.1:4783).
$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
Set-Location $root

$api = Join-Path $root 'inertia-api.exe'
if (-not (Test-Path $api)) {
    Write-Host 'inertia-api.exe not found in this folder.' -ForegroundColor Red
    Write-Host 'Download inertia-windows-x64.zip from GitHub Releases.'
    exit 1
}

$env:INERTIA_DATA_DIR = Join-Path $root 'data'
$env:INERTIA_WEB_DIR = Join-Path $root 'web'
$env:RUST_LOG = 'info'

$url = 'http://127.0.0.1:4783'
Write-Host "Starting Inertia — open $url"
Write-Host 'Close this window to stop.'
Write-Host ''

Start-Process $url
& $api
