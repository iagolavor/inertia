# Build inertia-api + web UI into Tauri sidecar/resources for apps/desktop.
# Usage: powershell -ExecutionPolicy Bypass -File scripts/package-desktop.ps1 [-DebugApi]
param(
    [switch]$DebugApi
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$Triple = (rustc --print host-tuple).Trim()
$Ext = if ($IsWindows -or $env:OS -match "Windows") { ".exe" } else { "" }

$Profile = if ($DebugApi) { "debug" } else { "release" }
Push-Location $Root
try {
    Write-Host "==> cargo build -p inertia-api ($Profile)"
    if ($DebugApi) {
        cargo build -p inertia-api
    } else {
        cargo build --release -p inertia-api
    }

    $TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root "target" }
    $SrcApi = Join-Path $TargetDir "$Profile/inertia-api$Ext"
    if (-not (Test-Path $SrcApi)) {
        throw "missing $SrcApi"
    }

    $BinDir = Join-Path $Root "apps/desktop/src-tauri/binaries"
    $ResWeb = Join-Path $Root "apps/desktop/src-tauri/resources/web"
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    New-Item -ItemType Directory -Force -Path (Split-Path $ResWeb) | Out-Null

    $DestApi = Join-Path $BinDir "inertia-api-$Triple$Ext"
    Write-Host "==> sidecar $DestApi"
    Copy-Item -Force $SrcApi $DestApi

    Write-Host "==> web:build"
    npm run web:build

    Write-Host "==> resources/web"
    if (Test-Path $ResWeb) {
        Remove-Item -Recurse -Force $ResWeb
    }
    New-Item -ItemType Directory -Force -Path $ResWeb | Out-Null
    Copy-Item -Recurse -Force (Join-Path $Root "apps/web/build/*") $ResWeb
    New-Item -ItemType File -Force -Path (Join-Path $ResWeb ".gitkeep") | Out-Null

    Write-Host "Desktop package ready:"
    Write-Host "  $DestApi"
    Write-Host "  $ResWeb"
}
finally {
    Pop-Location
}
