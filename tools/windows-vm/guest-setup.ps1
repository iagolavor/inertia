# One-time toolchain install inside the Windows guest (PowerShell as admin preferred).
# Requires network. Uses winget when available; otherwise prints manual download URLs.
#
#   powershell -ExecutionPolicy Bypass -File .\tools\windows-vm\guest-setup.ps1

$ErrorActionPreference = "Continue"

function Test-Cmd($name) {
    return [bool](Get-Command $name -ErrorAction SilentlyContinue)
}

Write-Host "==> Checking winget..."
if (-not (Test-Cmd "winget")) {
    Write-Host "winget not found. Install App Installer from the Microsoft Store, then re-run this script."
    Write-Host "Manual downloads:"
    Write-Host "  Node 20 LTS: https://nodejs.org/"
    Write-Host "  Rust:        https://rustup.rs/"
    Write-Host "  VS Build Tools (C++): https://visualstudio.microsoft.com/visual-cpp-build-tools/"
    exit 1
}

Write-Host "==> Node.js 20 LTS"
winget install -e --id OpenJS.NodeJS.LTS --accept-package-agreements --accept-source-agreements
Write-Host "==> Rust (MSVC)"
winget install -e --id Rustlang.Rust.MSVC --accept-package-agreements --accept-source-agreements
Write-Host "==> Visual Studio 2022 Build Tools (C++ workload)"
winget install -e --id Microsoft.VisualStudio.2022.BuildTools `
  --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended" `
  --accept-package-agreements --accept-source-agreements

Write-Host ""
Write-Host "Close and reopen PowerShell, then verify:"
Write-Host "  node -v"
Write-Host "  npm -v"
Write-Host "  rustc -V"
Write-Host "  cargo -V"
Write-Host ""
Write-Host "Then build:"
Write-Host "  powershell -ExecutionPolicy Bypass -File .\tools\windows-vm\guest-build.ps1"
