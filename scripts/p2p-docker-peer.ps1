# Bootstrap the Docker peer and print an invite link for your local browser.
# Prerequisites: docker compose up, local inertia-api on :4783 with P2P running.

param(
    [string]$PeerApi = "http://127.0.0.1:4784",
    [string]$WebOrigin = "http://localhost:5173",
    [string]$DisplayName = "Docker Peer",
    [int]$P2pPort = 9002
)

$ErrorActionPreference = "Stop"

function Invoke-PeerApi {
    param([string]$Method, [string]$Path, [object]$Body = $null)
    $uri = "$PeerApi$Path"
    if ($Body) {
        $json = $Body | ConvertTo-Json -Compress
        return Invoke-RestMethod -Uri $uri -Method $Method -Body $json -ContentType "application/json"
    }
    return Invoke-RestMethod -Uri $uri -Method $Method
}

Write-Host "Waiting for Docker peer API at $PeerApi ..."
for ($i = 0; $i -lt 60; $i++) {
    try {
        Invoke-RestMethod -Uri "$PeerApi/health" -Method Get | Out-Null
        break
    } catch {
        if ($i -eq 59) { throw "Peer API not reachable at $PeerApi" }
        Start-Sleep -Seconds 2
    }
}

$identity = Invoke-PeerApi -Method Get -Path "/identity"
if (-not $identity.display_name) {
    Write-Host "Creating identity '$DisplayName' ..."
    Invoke-PeerApi -Method Post -Path "/identity" -Body @{ display_name = $DisplayName } | Out-Null
}

Write-Host "Starting P2P on port $P2pPort ..."
$p2p = Invoke-PeerApi -Method Post -Path "/p2p/start" -Body @{ listen_port = $P2pPort }
Write-Host "Peer ID: $($p2p.peer_id)"
Write-Host "Listen addresses: $($p2p.addresses -join ', ')"

Write-Host "Generating invite (open on your local machine) ..."
$invite = Invoke-PeerApi -Method Post -Path "/invite" -Body @{ web_origin = $WebOrigin }

Write-Host ""
Write-Host "=== Docker peer invite ===" -ForegroundColor Cyan
Write-Host "Safety code: $($invite.safety_code)"
Write-Host "Link:"
Write-Host $invite.link
Write-Host ""
Write-Host "Next steps:"
Write-Host "  1. Open the link on your local app (or paste payload at /invite)"
Write-Host "  2. Confirm the safety code and accept"
Write-Host "  3. Post from Docker peer (see docs/P2P-EXPERIMENT.md)"
Write-Host ""

# Export for scripting
@{
    peer_id = $p2p.peer_id
    invite_link = $invite.link
    invite_payload = $invite.payload
    safety_code = $invite.safety_code
} | ConvertTo-Json | Set-Content -Path ".p2p-docker-peer.json" -Encoding utf8
Write-Host "Saved .p2p-docker-peer.json"
