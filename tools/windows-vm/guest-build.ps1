# Run inside the Windows 11 guest (PowerShell as user Docker).
# Copies the Shared repo onto NTFS, then builds the Tauri desktop installer.
#
# Prerequisites (one-time): Node 20 LTS, Rust (MSVC), VS Build Tools C++, WebView2.
#
# Usage (from Desktop\Shared or \\host.lan\Data):
#   powershell -ExecutionPolicy Bypass -File .\tools\windows-vm\guest-build.ps1

$ErrorActionPreference = "Stop"

function Test-RepoRoot([string]$root) {
    if ([string]::IsNullOrWhiteSpace($root)) { return $false }
    return (Test-Path -LiteralPath (Join-Path $root "package.json"))
}

function Find-SharedRepo {
    # Prefer Desktop shortcut first; Samba UNC is \\host.lan\Data (not Resolve-Path - it mangles UNC).
    $candidates = @(
        (Join-Path $env:USERPROFILE "Desktop\Shared"),
        "\\host.lan\Data",
        "\\host.lan\Data\",
        "\\tsclient\Shared",
        "Z:\",
        "Y:\",
        "X:\"
    )

    # If this script lives under the shared tree (...\tools\windows-vm), use that repo root.
    # Do not use Resolve-Path on UNC (it injects Microsoft.PowerShell.Core\FileSystem::).
    if ($PSScriptRoot) {
        $fromScript = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
        if (Test-RepoRoot $fromScript) {
            return $fromScript
        }
    }

    foreach ($root in $candidates) {
        if (-not (Test-Path -LiteralPath $root)) { continue }
        if (Test-RepoRoot $root) {
            # Normalize without PowerShell provider prefix (FileSystem::\\host...)
            $item = Get-Item -LiteralPath $root
            return $item.FullName
        }
        $nested = Join-Path $root "inertia"
        if (Test-RepoRoot $nested) {
            return (Get-Item -LiteralPath $nested).FullName
        }
    }
    throw "Could not find Inertia package.json. Open Desktop\Shared or \\host.lan\Data, then re-run this script from that folder."
}

$src = Find-SharedRepo
$dest = "C:\Users\Docker\src\inertia"

# robocopy wants a trailing separator on UNC sources
if ($src -match '^\\\\' -and -not $src.EndsWith("\")) {
    $src = "$src\"
}

Write-Host "==> Shared source: $src"
Write-Host "==> Copy to: $dest"

New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null

# /XD matches directory *names* only (not full paths). Never copy the dockur VM disk.
$xd = @(
    "node_modules",
    "target",
    "build",
    ".svelte-kit",
    "data",
    ".git",
    "dist"
)
$xf = @(
    "*.img",
    "*.qcow2",
    "*.vmdk",
    "*.iso"
)

$robocopyArgs = @($src, $dest, "/MIR", "/NFL", "/NDL", "/NJH", "/NJS", "/nc", "/ns", "/np")
foreach ($d in $xd) {
    $robocopyArgs += "/XD"
    $robocopyArgs += $d
}
foreach ($f in $xf) {
    $robocopyArgs += "/XF"
    $robocopyArgs += $f
}

& robocopy @robocopyArgs
if ($LASTEXITCODE -ge 8) {
    throw "robocopy failed with exit code $LASTEXITCODE"
}

Set-Location -LiteralPath $dest

# Root package.json has no deps; vite lives under apps/web (and tauri under apps/desktop).
Write-Host "==> npm install (apps/web)"
npm install --prefix apps/web
if ($LASTEXITCODE -ne 0) { throw "npm install apps/web failed" }

Write-Host "==> npm install (apps/desktop)"
npm install --prefix apps/desktop
if ($LASTEXITCODE -ne 0) { throw "npm install apps/desktop failed" }

Write-Host "==> npm run desktop:build"
npm run desktop:build
if ($LASTEXITCODE -ne 0) { throw "desktop:build failed" }

$bundle = Join-Path $dest "apps\desktop\src-tauri\target\release\bundle"
Write-Host "Build finished. Look for NSIS/MSI under:"
Write-Host "  $bundle"
Get-ChildItem -Recurse -Path $bundle -Include *.exe,*.msi -ErrorAction SilentlyContinue | ForEach-Object { Write-Host "  $($_.FullName)" }
