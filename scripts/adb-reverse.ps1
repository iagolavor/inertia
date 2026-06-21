# Forward phone localhost:4783 -> PC API (USB Stage A). Re-run after unplugging the cable.
# Usage:
#   npm run android:reverse              # set up reverse
#   npm run android:reverse -- --remove  # clear reverse (free PC port 4783 for local API)
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Args
)

$ErrorActionPreference = 'Stop'

$SdkRoot = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { Join-Path $env:LOCALAPPDATA 'Android\Sdk' }
$Adb = Join-Path $SdkRoot 'platform-tools\adb.exe'

if (-not (Test-Path $Adb)) {
    Write-Error "adb not found at $Adb. Run: npm run android:sdk"
}

function Invoke-AdbQuiet {
    param([string[]]$AdbArgs)
    $prevPref = $ErrorActionPreference
    $ErrorActionPreference = 'SilentlyContinue'
    try {
        # adb prints "listener not found" to stderr when nothing to remove — ignore.
        & $Adb @AdbArgs 2>$null | Out-Null
    } finally {
        $ErrorActionPreference = $prevPref
    }
}

function Remove-AdbForward {
    param([string]$DeviceSerial)
    if ($DeviceSerial) {
        Invoke-AdbQuiet @('-s', $DeviceSerial, 'forward', '--remove', 'tcp:4783')
    } else {
        Invoke-AdbQuiet @('forward', '--remove', 'tcp:4783')
    }
}

function Remove-AdbReverse {
    param([string]$DeviceSerial)
    if ($DeviceSerial) {
        Invoke-AdbQuiet @('-s', $DeviceSerial, 'reverse', '--remove', 'tcp:4783')
    } else {
        Invoke-AdbQuiet @('reverse', '--remove', 'tcp:4783')
    }
}

$remove = $Args -contains '--remove'
$serial = ($Args | Where-Object { $_ -ne '--remove' } | Select-Object -First 1)

if ($remove) {
    Remove-AdbReverse $serial
    Remove-AdbForward $serial
    Write-Host 'Cleared adb reverse/forward on tcp:4783 (if any were set).'
} elseif ($serial) {
    & $Adb -s $serial reverse tcp:4783 tcp:4783
} else {
    & $Adb reverse tcp:4783 tcp:4783
}

Write-Host 'Active reverse forwards:'
& $Adb reverse --list
