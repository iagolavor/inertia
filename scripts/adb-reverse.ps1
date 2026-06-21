# Forward phone localhost:4783 → PC API (USB Stage A). Re-run after unplugging the cable.
$ErrorActionPreference = 'Stop'

$SdkRoot = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { Join-Path $env:LOCALAPPDATA 'Android\Sdk' }
$Adb = Join-Path $SdkRoot 'platform-tools\adb.exe'

if (-not (Test-Path $Adb)) {
    Write-Error "adb not found at $Adb. Run: npm run android:sdk"
}

$serial = $args[0]
if ($serial) {
    & $Adb -s $serial reverse tcp:4783 tcp:4783
} else {
    & $Adb reverse tcp:4783 tcp:4783
}

Write-Host 'Active reverse forwards:'
& $Adb reverse --list
