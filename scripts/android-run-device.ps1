# Build, sync, and install the Android app on a connected device (no interactive picker).
# Usage:
#   npm run android:run:device
#   powershell -File scripts/android-run-device.ps1 -Target RQCX302579V
#   $env:ANDROID_SERIAL = 'RQCX302579V'; npm run android:run:device

param(
    [string]$Target = $env:ANDROID_SERIAL,
    [switch]$NoSync
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$web = Join-Path $root 'apps/web'

$sdk = $env:ANDROID_HOME
if (-not $sdk) {
    $sdk = Join-Path $env:LOCALAPPDATA 'Android\Sdk'
}
$adb = Join-Path $sdk 'platform-tools\adb.exe'
if (-not (Test-Path $adb)) {
    throw "adb not found at $adb - run npm run android:sdk"
}

$lines = & $adb devices | Select-Object -Skip 1 | Where-Object { $_ -match '\S+\s+device$' }
$ids = @($lines | ForEach-Object { ($_ -split '\s+')[0] })

if ($ids.Count -eq 0) {
    throw "No Android device connected. Plug in the phone and enable USB debugging."
}

if (-not $Target) {
    if ($ids.Count -gt 1) {
        throw "Multiple devices: $($ids -join ', '). Set -Target or env ANDROID_SERIAL."
    }
    $Target = $ids[0]
} elseif ($Target -notin $ids) {
    throw "Target $Target not connected. Available: $($ids -join ', ')"
}

Write-Host "Installing on $Target ..."

Push-Location $web
try {
    $capArgs = @('run', 'android', '--target', $Target)
    if ($NoSync) {
        $capArgs += '--no-sync'
    }
    & npx cap @capArgs
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
} finally {
    Pop-Location
}
