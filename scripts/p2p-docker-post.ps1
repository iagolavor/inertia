# Post a message from the Docker peer to all contacts (your local profile after invite accept).
param(
    [string]$PeerApi = "http://127.0.0.1:4784",
    [string]$Body = "Hello from the Docker peer - P2P experiment"
)

$ErrorActionPreference = "Stop"

$json = @{ body = $Body } | ConvertTo-Json -Compress
$result = Invoke-RestMethod -Uri "$PeerApi/posts" -Method Post -Body $json -ContentType "application/json"
Write-Host "Posted content_id: $($result.content_id)"
Write-Host "Check Outbox on Docker peer and reload Feed on your local app."
