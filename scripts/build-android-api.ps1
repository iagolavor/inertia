# Cross-compile inertia-api for Android arm64 (Stage B).
# Requires: Rust, Android SDK/NDK (npm run android:sdk), cargo-ndk.
# Usage: powershell -ExecutionPolicy Bypass -File scripts/build-android-api.ps1

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$outDir = Join-Path $root 'dist/android-arm64'
$outBin = Join-Path $outDir 'inertia-api'
$builtBin = Join-Path $root 'target/aarch64-linux-android/release/inertia-api'

# getifaddrs/freeifaddrs (libp2p -> if-addrs) require Android API 24+.
$androidApi = 24

$sdk = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { Join-Path $env:LOCALAPPDATA 'Android\Sdk' }
if (-not (Test-Path $sdk)) {
    throw "Android SDK not found at $sdk - run: npm run android:sdk"
}

$ndkRoot = Join-Path $sdk 'ndk'
if (-not (Test-Path $ndkRoot)) {
    throw "Android NDK not found under $sdk\ndk - install via Android Studio SDK Manager"
}

$cargoNdk = Get-Command cargo-ndk -ErrorAction SilentlyContinue
if (-not $cargoNdk) {
    Write-Host "Installing cargo-ndk..."
    cargo install cargo-ndk --locked
}

Write-Host "Adding Rust target aarch64-linux-android (if missing)..."
rustup target add aarch64-linux-android | Out-Null

Push-Location $root
try {
    Write-Host "Building inertia-api for arm64-v8a (API $androidApi)..."
    # Do not use -o here: that mode copies JNI .so artifacts, not the standalone binary.
    cargo ndk -t arm64-v8a -P $androidApi build --release --bin inertia-api -p inertia-api
    if (-not (Test-Path $builtBin)) {
        throw "Expected binary at $builtBin"
    }
    New-Item -ItemType Directory -Path $outDir -Force | Out-Null
    Copy-Item $builtBin $outBin -Force
    Write-Host "Built $outBin"
} finally {
    Pop-Location
}
