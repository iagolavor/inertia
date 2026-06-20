# Start release API + static web preview in separate PowerShell windows.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

function Start-InertiaWindow([string]$Title, [string]$Command) {
    Start-Process powershell -ArgumentList @(
        '-NoExit',
        '-Command',
        "`$Host.UI.RawUI.WindowTitle = '$Title'; Set-Location '$root'; $Command"
    )
}

Write-Host 'Starting inertia-api (release) and static web preview...'
Write-Host '  API:  http://127.0.0.1:4783'
Write-Host '  Web:  http://localhost:4173  (build + preview, not Vite dev)'
Write-Host ''

Start-InertiaWindow 'inertia-api' "`$env:RUST_LOG='info'; npm run api:release"
Start-Sleep -Seconds 1
Start-InertiaWindow 'inertia-web' 'npm run web:build; if ($?) { npm run web:preview }'

Write-Host 'Two windows opened. Close them to stop Inertia.'
