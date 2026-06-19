# Opens SSH to your inertia relay VPS.
# Usage:
#   npm run vps:ssh
#   npm run vps:ssh -- -User root
#   npm run vps:ssh -- -VpsHost 203.0.113.10 -User ubuntu
#
# Set defaults in .env (gitignored):
#   INERTIA_VPS_HOST=your.vps.ip
#   INERTIA_VPS_USER=ubuntu

param(
    [string]$VpsHost,
    [string]$User,
    [int]$Port = 22
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path $PSScriptRoot -Parent
$envFile = Join-Path $repoRoot ".env"

if (Test-Path $envFile) {
    Get-Content $envFile | ForEach-Object {
        $line = $_.Trim()
        if ($line -eq "" -or $line.StartsWith("#")) { return }
        if ($line -match '^\s*([^#=]+)=(.*)$') {
            $name = $Matches[1].Trim()
            $value = $Matches[2].Trim().Trim('"').Trim("'")
            Set-Item -Path "Env:$name" -Value $value
        }
    }
}

if (-not $VpsHost) { $VpsHost = $env:INERTIA_VPS_HOST }
if (-not $User) { $User = $env:INERTIA_VPS_USER }

if (-not $User) { $User = "ubuntu" }

if (-not $VpsHost) {
    Write-Host @"
Missing VPS host. Either:

  1. Add to .env in the repo root:
       INERTIA_VPS_HOST=your.vps.ip
       INERTIA_VPS_USER=ubuntu

  2. Or pass -VpsHost:
       npm run vps:ssh -- -VpsHost your.vps.ip -User ubuntu
"@ -ForegroundColor Yellow
    exit 1
}

$target = "${User}@${VpsHost}"
Write-Host "Connecting to $target ..." -ForegroundColor Cyan
ssh -p $Port $target
