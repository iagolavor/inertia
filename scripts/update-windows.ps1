# Pull the latest Inertia release and apply it (prebuilt zip when available).
# Run from repo root:  npm run update:windows
# Or double-click:    scripts/update-windows.cmd
#
# Prebuilt installs (inertia-api.exe in folder): downloads inertia-windows-x64.zip, no rebuild.
# Source installs (git clone): git pull or source zip + rebuild.

param(
    [ValidateSet('release', 'development')]
    [string]$Channel = 'release',
    [switch]$Force,
    [switch]$Source,
    [switch]$Start
)

$ErrorActionPreference = 'Stop'
$Repo = 'iagolavor/inertia'
$PrebuiltAsset = 'inertia-windows-x64.zip'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$CodeDirs = @('apps', 'crates', 'docker', 'docs', 'scripts', 'tools')
$CodeFiles = @(
    'Cargo.toml', 'Cargo.lock', 'package.json', 'LICENSE', 'README.md',
    'AGENTS.md', '.gitignore'
)

function Write-Step([string]$Message) {
    Write-Host ''
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Test-Tool([string]$Name) {
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Test-PrebuiltInstall {
    return Test-Path (Join-Path $root 'inertia-api.exe')
}

function Get-LocalVersion {
    $versionFile = Join-Path $root '.inertia-version'
    if (Test-Path $versionFile) {
        return (Get-Content $versionFile -Raw).Trim()
    }
    if (Test-Path (Join-Path $root '.git')) {
        $prev = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        try {
            $tag = git describe --tags --exact-match 2>$null
            if ($LASTEXITCODE -eq 0 -and $tag) { return $tag.Trim() }
            $branch = git rev-parse --abbrev-ref HEAD 2>$null
            $sha = git rev-parse --short HEAD 2>$null
            if ($branch -and $sha) { return "$branch@$sha".Trim() }
        } finally {
            $ErrorActionPreference = $prev
        }
    }
    return $null
}

function Set-LocalVersion([string]$Version) {
    Set-Content -Path (Join-Path $root '.inertia-version') -Value $Version -NoNewline
}

function Get-ReleaseInfo {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    $asset = $release.assets | Where-Object { $_.name -eq $PrebuiltAsset } | Select-Object -First 1
    return @{
        Tag = $release.tag_name
        SourceZipUrl = "https://github.com/$Repo/archive/refs/tags/$($release.tag_name).zip"
        PrebuiltUrl = $asset.browser_download_url
        HasPrebuilt = [bool]$asset
    }
}

function Get-DevelopmentInfo {
    return @{
        Tag = 'development'
        SourceZipUrl = "https://github.com/$Repo/archive/refs/heads/development.zip"
        PrebuiltUrl = $null
        HasPrebuilt = $false
    }
}

function Invoke-Robocopy([string]$Source, [string]$Dest) {
    if (-not (Test-Path $Source)) { return }
    if (-not (Test-Path $Dest)) {
        New-Item -ItemType Directory -Path $Dest -Force | Out-Null
    }
    & robocopy $Source $Dest /E /NFL /NDL /NJH /NJS /NC /NS | Out-Null
    if ($LASTEXITCODE -ge 8) {
        throw "robocopy failed copying $Source -> $Dest (exit $LASTEXITCODE)"
    }
}

function Sync-SourceTree([string]$ExtractedRoot) {
    Write-Host "  Syncing source from $ExtractedRoot"
    foreach ($dir in $CodeDirs) {
        Invoke-Robocopy (Join-Path $ExtractedRoot $dir) (Join-Path $root $dir)
    }
    foreach ($file in $CodeFiles) {
        $src = Join-Path $ExtractedRoot $file
        if (Test-Path $src) {
            Copy-Item $src (Join-Path $root $file) -Force
        }
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

function Update-FromPrebuiltZip([string]$Url, [string]$Tag) {
    $tempRoot = Join-Path $env:TEMP "inertia-prebuilt-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
    $zipPath = Join-Path $tempRoot 'inertia.zip'
    $extractDir = Join-Path $tempRoot 'extract'
    New-Item -ItemType Directory -Path $extractDir -Force | Out-Null

    try {
        Write-Host "  Downloading $PrebuiltAsset ($Tag)..."
        Invoke-WebRequest -Uri $Url -OutFile $zipPath -UseBasicParsing

        Write-Host '  Extracting...'
        Expand-Archive -Path $zipPath -DestinationPath $extractDir -Force

        $backupDir = Join-Path $tempRoot 'backup'
        Backup-PreservedData $backupDir

        foreach ($item in @('inertia-api.exe', 'run-desktop.cmd', 'run-desktop.ps1', 'LICENSE')) {
            $src = Join-Path $extractDir $item
            if (Test-Path $src) {
                Copy-Item $src (Join-Path $root $item) -Force
            }
        }

        $scriptsSrc = Join-Path $extractDir 'scripts'
        if (Test-Path $scriptsSrc) {
            $scriptsDest = Join-Path $root 'scripts'
            if (-not (Test-Path $scriptsDest)) {
                New-Item -ItemType Directory -Path $scriptsDest -Force | Out-Null
            }
            Invoke-Robocopy $scriptsSrc $scriptsDest
        }

        $webSrc = Join-Path $extractDir 'web'
        if (Test-Path $webSrc) {
            $webDest = Join-Path $root 'web'
            if (Test-Path $webDest) { Remove-Item $webDest -Recurse -Force }
            Copy-Item $webSrc $webDest -Recurse -Force
        }

        Restore-PreservedData $backupDir
        Set-LocalVersion $Tag
    } finally {
        Remove-Item $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Update-FromSourceZip([hashtable]$Info) {
    $tempRoot = Join-Path $env:TEMP "inertia-update-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
    $zipPath = Join-Path $tempRoot 'inertia.zip'
    New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null

    try {
        Write-Host "  Downloading source ($($Info.Tag))..."
        Invoke-WebRequest -Uri $Info.SourceZipUrl -OutFile $zipPath -UseBasicParsing

        Write-Host '  Extracting...'
        Expand-Archive -Path $zipPath -DestinationPath $tempRoot -Force
        $extracted = Get-ChildItem $tempRoot -Directory |
            Where-Object { $_.Name -like 'inertia-*' } |
            Select-Object -First 1
        if (-not $extracted) {
            throw 'Could not find extracted inertia-* folder in archive'
        }

        Sync-SourceTree $extracted.FullName
    } finally {
        Remove-Item $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Update-FromGit([hashtable]$Info) {
    if (-not (Test-Tool 'git')) {
        throw 'Git is not installed. Use prebuilt or ZIP update - see docs/WINDOWS-SETUP.md'
    }

    $branch = (git rev-parse --abbrev-ref HEAD 2>$null).Trim()
    $safeBranches = @('master', 'development', 'HEAD')
    if ($branch -and $safeBranches -notcontains $branch -and -not $Force) {
        Write-Host ''
        Write-Host "You are on branch '$branch'. This script is for stable end-user installs." -ForegroundColor Yellow
        Write-Host 'Developers: use git pull on your branch. Pass -Force to switch to the update channel anyway.'
        exit 1
    }

    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $dirty = git status --porcelain 2>$null
    $ErrorActionPreference = $prev
    if ($dirty -and -not $Force) {
        Write-Host ''
        Write-Host 'You have uncommitted local changes. Commit or stash them first, or pass -Force.' -ForegroundColor Yellow
        exit 1
    }

    git fetch origin --tags --prune
    if ($LASTEXITCODE -ne 0) { throw 'git fetch failed' }

    if ($Channel -eq 'release') {
        Write-Host "  Checking out $($Info.Tag)..."
        git checkout $Info.Tag
        if ($LASTEXITCODE -ne 0) { throw 'git checkout failed' }
    } else {
        Write-Host '  Pulling development...'
        git checkout development 2>$null
        if ($LASTEXITCODE -ne 0) {
            git checkout -B development 'origin/development'
            if ($LASTEXITCODE -ne 0) { throw 'git checkout development failed' }
        }
        git pull origin development
        if ($LASTEXITCODE -ne 0) { throw 'git pull failed' }
    }
}

function Start-Inertia {
    if (Test-PrebuiltInstall) {
        & (Join-Path $root 'run-desktop.ps1')
    } else {
        & "$PSScriptRoot/run-windows.ps1"
    }
}

Write-Step 'Stopping API (if running)'
& "$PSScriptRoot/stop-api.ps1" | Out-Null

Write-Step "Checking for updates ($Channel)"
$info = if ($Channel -eq 'release') { Get-ReleaseInfo } else { Get-DevelopmentInfo }
$remoteLabel = $info.Tag
$localVersion = Get-LocalVersion
$prebuiltInstall = Test-PrebuiltInstall
$usePrebuilt = $info.HasPrebuilt -and $prebuiltInstall -and -not $Source -and $Channel -eq 'release'

Write-Host "  Local:    $(if ($localVersion) { $localVersion } else { '(unknown)' })"
Write-Host "  Remote:   $remoteLabel"
Write-Host "  Mode:     $(if ($usePrebuilt) { 'prebuilt (no rebuild)' } elseif ($Source -or -not $prebuiltInstall) { 'source + rebuild' } else { 'source + rebuild' })"

if (-not $Force -and $localVersion -and $localVersion -eq $remoteLabel) {
    Write-Host ''
    Write-Host 'Already up to date. Use -Force to apply again anyway.' -ForegroundColor Green
    if ($Start) { Start-Inertia }
    exit 0
}

Write-Step 'Updating'
try {
    if ($usePrebuilt) {
        Update-FromPrebuiltZip $info.PrebuiltUrl $remoteLabel
    } elseif (Test-Path (Join-Path $root '.git')) {
        Update-FromGit $info
        Set-LocalVersion $remoteLabel
    } else {
        Update-FromSourceZip $info
        Set-LocalVersion $remoteLabel
    }
} catch {
    Write-Host ''
    Write-Host "Update failed: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

if (-not $usePrebuilt) {
    Write-Step 'Rebuilding (web + API)'
    & "$PSScriptRoot/setup-windows.ps1"
    Set-LocalVersion $remoteLabel
}

Write-Step 'Update complete'
Write-Host ''
Write-Host "Now on $remoteLabel" -ForegroundColor Green
if ($usePrebuilt -or (Test-PrebuiltInstall)) {
    Write-Host 'Start Inertia:  double-click run-desktop.cmd'
} else {
    Write-Host 'Start Inertia:  npm run run:windows'
}

if ($Start) {
    Start-Inertia
}
