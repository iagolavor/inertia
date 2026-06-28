# Download and apply the latest inertia-windows-x64.zip (keeps data/ and .env).
# Double-click update.cmd in your install folder, or:
#   powershell -ExecutionPolicy Bypass -File update.ps1 [-Start] [-Force]

param(
    [switch]$Force,
    [switch]$Start
)

$ErrorActionPreference = 'Stop'
$Repo = 'iagolavor/inertia'
$PrebuiltAsset = 'inertia-windows-x64.zip'
$root = $PSScriptRoot

function Write-Step([string]$Message) {
    Write-Host ''
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Stop-InertiaApi {
    $procs = Get-Process -Name 'inertia-api' -ErrorAction SilentlyContinue
    if (-not $procs) { return }
    $procs | ForEach-Object {
        Write-Host "  Stopping inertia-api (PID $($_.Id))..."
        Stop-Process -Id $_.Id -Force
    }
    Start-Sleep -Seconds 1
}

function Get-LocalVersion {
    $versionFile = Join-Path $root '.inertia-version'
    if (Test-Path $versionFile) {
        return (Get-Content $versionFile -Raw).Trim()
    }
    return $null
}

function Get-ReleaseAsset {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    $asset = $release.assets | Where-Object { $_.name -eq $PrebuiltAsset } | Select-Object -First 1
    if (-not $asset) {
        throw "No $PrebuiltAsset on latest release - wait for CI or download manually from GitHub Releases"
    }
    return @{
        Tag = $release.tag_name
        Url = $asset.browser_download_url
    }
}

function Backup-PreservedData([string]$BackupDir) {
    New-Item -ItemType Directory -Path $BackupDir -Force | Out-Null
    foreach ($item in @('data', '.env')) {
        $src = Join-Path $root $item
        if (Test-Path $src) {
            Copy-Item $src (Join-Path $BackupDir $item) -Recurse -Force
        }
    }
}

function Restore-PreservedData([string]$BackupDir) {
    foreach ($item in @('data', '.env')) {
        $bak = Join-Path $BackupDir $item
        if (Test-Path $bak) {
            Copy-Item $bak (Join-Path $root $item) -Recurse -Force
        }
    }
}

if (-not (Test-Path (Join-Path $root 'inertia-api.exe'))) {
    Write-Host 'Run update from your Inertia install folder (next to inertia-api.exe).' -ForegroundColor Red
    exit 1
}

Set-Location $root

Write-Step 'Stopping Inertia'
Stop-InertiaApi

Write-Step 'Checking for updates'
$remote = Get-ReleaseAsset
$localVersion = Get-LocalVersion

Write-Host "  Local:  $(if ($localVersion) { $localVersion } else { '(unknown)' })"
Write-Host "  Remote: $($remote.Tag)"

if (-not $Force -and $localVersion -eq $remote.Tag) {
    Write-Host ''
    Write-Host 'Already up to date.' -ForegroundColor Green
    if ($Start) { & (Join-Path $root 'run.ps1') }
    exit 0
}

Write-Step "Downloading $PrebuiltAsset"
$tempRoot = Join-Path $env:TEMP "inertia-update-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
$zipPath = Join-Path $tempRoot 'inertia.zip'
$extractDir = Join-Path $tempRoot 'extract'
New-Item -ItemType Directory -Path $extractDir -Force | Out-Null

try {
    Invoke-WebRequest -Uri $remote.Url -OutFile $zipPath -UseBasicParsing
    Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force

    $backupDir = Join-Path $tempRoot 'backup'
    Backup-PreservedData $backupDir

    foreach ($item in @('inertia-api.exe', 'run.cmd', 'run.ps1', 'update.cmd', 'update.ps1', 'LICENSE', '.inertia-version')) {
        $src = Join-Path $extractDir $item
        if (Test-Path $src) {
            Copy-Item $src (Join-Path $root $item) -Force
        }
    }

    $webSrc = Join-Path $extractDir 'web'
    if (Test-Path $webSrc) {
        $webDest = Join-Path $root 'web'
        if (Test-Path $webDest) { Remove-Item $webDest -Recurse -Force }
        Copy-Item $webSrc $webDest -Recurse -Force
    }

    Restore-PreservedData $backupDir
    Set-Content -Path (Join-Path $root '.inertia-version') -Value $remote.Tag -NoNewline
} catch {
    Write-Host ''
    Write-Host "Update failed: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
} finally {
    Remove-Item $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Step 'Update complete'
Write-Host "Now on $($remote.Tag)" -ForegroundColor Green
Write-Host 'Start Inertia: double-click run.cmd'

if ($Start) {
    & (Join-Path $root 'run.ps1')
}
