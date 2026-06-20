# First-time Windows setup: check tools, optional winget installs, build API + web.
# Run from repo root:  npm run setup:windows
# Or double-click:    scripts/setup-windows.cmd

param(
    [switch]$InstallDeps,
    [switch]$SkipBuild,
    [switch]$Start
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Refresh-Path {
    $env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' +
                [Environment]::GetEnvironmentVariable('Path', 'User')
}

function Test-Tool([string]$Name) {
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Write-Step([string]$Message) {
    Write-Host ''
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Install-WingetPackage([string]$Id, [string]$Label) {
    if (-not (Test-Tool 'winget')) {
        Write-Host "  winget is not available. Install $Label manually - see docs/WINDOWS-SETUP.md" -ForegroundColor Yellow
        return $false
    }
    Write-Host "  Installing $Label via winget..."
    winget install -e --id $Id --accept-package-agreements --accept-source-agreements
    Refresh-Path
    return $true
}

function Ensure-Tool {
    param(
        [string]$Command,
        [string]$Label,
        [string]$WingetId,
        [switch]$Required
    )
    if (Test-Tool $Command) {
        Write-Host "  OK  $Label"
        return $true
    }
    Write-Host "  --  $Label not found" -ForegroundColor Yellow
    if ($InstallDeps) {
        if (-not (Install-WingetPackage $WingetId $Label)) { return -not $Required }
        if (-not (Test-Tool $Command)) {
            Write-Host "  $Label still missing after install. Open a new terminal and run this script again." -ForegroundColor Yellow
            return -not $Required
        }
        return $true
    }
    if ($Required) {
        Write-Host "  Re-run with -InstallDeps or: npm run setup:windows -- -InstallDeps" -ForegroundColor Yellow
    }
    return -not $Required
}

Refresh-Path

Write-Step 'Checking prerequisites'
$hasGit = Ensure-Tool -Command 'git' -Label 'Git' -WingetId 'Git.Git'
$hasNode = Ensure-Tool -Command 'node' -Label 'Node.js' -WingetId 'OpenJS.NodeJS.LTS' -Required
$hasNpm = Ensure-Tool -Command 'npm' -Label 'npm' -WingetId 'OpenJS.NodeJS.LTS' -Required
$hasCargo = Ensure-Tool -Command 'cargo' -Label 'Rust (cargo)' -WingetId 'Rustlang.Rustup' -Required

if (-not $hasNode -or -not $hasNpm -or -not $hasCargo) {
    Write-Host ''
    Write-Host 'Missing required tools. See docs/WINDOWS-SETUP.md for manual install links.' -ForegroundColor Red
    exit 1
}

if (-not $hasGit) {
    Write-Host ''
    Write-Host 'Git is optional if you already have the project folder (e.g. downloaded as a ZIP).' -ForegroundColor DarkYellow
}

if ($SkipBuild) {
    Write-Step 'Skipping build (-SkipBuild)'
} else {
    Write-Step 'Installing web dependencies'
    Push-Location apps/web
    npm install
    Pop-Location

    Write-Step 'Building web UI (static preview)'
    npm run web:build

    Write-Step 'Building API (release - first compile may take several minutes)'
    cargo build --release -p inertia-api
}

Write-Step 'Done'
Write-Host ''
Write-Host 'Start Inertia:' -ForegroundColor Green
Write-Host '  npm run run:windows          # two windows: API + web'
Write-Host '  npm run api:window           # API only (separate window)'
Write-Host '  npm run web:preview          # web UI at http://localhost:4173'
Write-Host '  npm run update:windows       # pull latest release + rebuild'
Write-Host ''
Write-Host 'Full guide: docs/WINDOWS-SETUP.md'

try {
    $tag = (Invoke-RestMethod -Uri 'https://api.github.com/repos/iagolavor/inertia/releases/latest').tag_name
    Set-Content -Path (Join-Path $root '.inertia-version') -Value $tag -NoNewline
} catch {
    # Offline or API rate limit — version file is optional
}

if ($Start) {
    & "$PSScriptRoot/run-windows.ps1"
}
