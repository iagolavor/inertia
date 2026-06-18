# Stops any running inertia-api process (frees port 4783 for rebuild).
$procs = Get-Process -Name "inertia-api" -ErrorAction SilentlyContinue
if (-not $procs) {
  Write-Host "inertia-api is not running."
  exit 0
}
$procs | ForEach-Object {
  Write-Host "Stopping inertia-api (PID $($_.Id))..."
  Stop-Process -Id $_.Id -Force
}
Start-Sleep -Seconds 1
Write-Host "Done."
