# Build inertia-windows-x64.zip for GitHub Releases (run after release API + web build).
# Usage: powershell -ExecutionPolicy Bypass -File scripts/package-windows.ps1 [-Tag v0.7.0]

param(
    [string]$OutDir = 'dist',
    [string]$Tag = ''
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$win = Join-Path $root 'scripts/windows'

$apiExe = Join-Path $root 'target/release/inertia-api.exe'
$webBuild = Join-Path $root 'apps/web/build'

if (-not (Test-Path $apiExe)) {
    throw 'Missing target/release/inertia-api.exe - run: cargo build --release -p inertia-api'
}
if (-not (Test-Path (Join-Path $webBuild 'index.html'))) {
    throw 'Missing apps/web/build - run: npm run web:build'
}

$stage = Join-Path $env:TEMP "inertia-pack-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
New-Item -ItemType Directory -Path $stage -Force | Out-Null

try {
    Copy-Item $apiExe (Join-Path $stage 'inertia-api.exe')
    Copy-Item $webBuild (Join-Path $stage 'web') -Recurse
    Copy-Item (Join-Path $root 'LICENSE') $stage -ErrorAction SilentlyContinue

    foreach ($name in @('run.cmd', 'run.ps1', 'update.cmd', 'update.ps1')) {
        Copy-Item (Join-Path $win $name) $stage
    }

    if ($Tag) {
        Set-Content -Path (Join-Path $stage '.inertia-version') -Value $Tag -NoNewline
    }

    $outPath = Join-Path $root $OutDir
    New-Item -ItemType Directory -Path $outPath -Force | Out-Null
    $zip = Join-Path $outPath 'inertia-windows-x64.zip'
    if (Test-Path $zip) { Remove-Item $zip -Force }

    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip
    Write-Host "Created $zip"
} finally {
    Remove-Item $stage -Recurse -Force -ErrorAction SilentlyContinue
}
