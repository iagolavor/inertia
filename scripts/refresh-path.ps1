# Reload Machine + User PATH into the current PowerShell session.
# Use when node/npm work in a new Windows terminal but not in an old Cursor tab.

$env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
            [Environment]::GetEnvironmentVariable("Path", "User")

Write-Host "PATH refreshed."
Write-Host "node:  $(Get-Command node -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source)"
Write-Host "npm:   $(Get-Command npm -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source)"
Write-Host "cargo: $(Get-Command cargo -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source)"
