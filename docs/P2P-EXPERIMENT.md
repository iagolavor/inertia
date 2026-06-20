# P2P experiment guide

This document explains how Inertia connects peers **today**, what works across the internet, and how to run a **local Docker experiment** (two profiles on one machine).

---

## How P2P works in Inertia (current code)

```
┌──────────────┐     invite (signed link + multiaddrs)     ┌──────────────┐
│   Peer A     │ ────────────────────────────────────────► │   Peer B     │
│  inertia-api │                                           │  inertia-api │
│  + libp2p    │ ◄──────── TCP dial + Noise + Yamux ────── │  + libp2p    │
└──────────────┘     request-response /inertia/1.0.0        └──────────────┘
```

1. **Identity** — Ed25519/X25519 keypairs in SQLite (one profile per device).
2. **Friends** — Signed **invite links** (15 min, single-use) carry public keys + optional `peer_id` + **multiaddrs**.
3. **P2P start** — `POST /p2p/start` opens a libp2p TCP listener (`0.0.0.0:<port>`).
4. **Accept invite** — Accepter dials inviter multiaddrs, sends `InviteRedemption`; both store contacts.
5. **Posts / messages** — Encrypted `ContentEnvelope` per recipient → **outbox** → `SendEnvelope` over P2P when connected.
6. **Receive** — Recipient verifies signature, decrypts, writes **inbox** → appears in **feed**.

### Strict mode (by design)

| Feature | Status |
|---------|--------|
| TCP + Noise + Yamux | Yes |
| Relay / DCUtR (VPS `inertia-relay`) | **Yes** — circuit relay + hole punching |
| NAT traversal | **Via relay** when direct TCP fails |
| Blob sync (post images) | **Yes** — hash in envelope; bytes via `BlobPush` after ack or `BlobRequest` on demand |
| Auto outbox flush on reconnect | **Yes** |

### Across the real internet

For two home networks to connect **without** a relay, you typically need:

- A **public IP** or **port forwarding** on at least one side, and
- Invites that advertise a **reachable** multiaddr (not `0.0.0.0` or a private IP the other peer cannot route to).

`INERTIA_P2P_ANNOUNCE` (comma-separated) overrides addresses embedded in invites:

```bash
# Example: you forwarded host port 9001 → your machine
INERTIA_P2P_ANNOUNCE=/ip4/203.0.113.10/tcp/9001
```

Without that, invites often contain useless listen addresses like `/ip4/0.0.0.0/tcp/54321`.

**Same LAN** or **Docker port mapping on one host** is the realistic way to test today.

---

## Docker experiment (recommended)

Simulates a “remote friend” on your PC:

| Role | API | P2P port | Data |
|------|-----|----------|------|
| **You (host)** | `127.0.0.1:4783` | e.g. `9001` (optional fixed) | `./data` |
| **Docker peer** | `127.0.0.1:4784` | `9002` (mapped) | Docker volume |

### 1. Start your local stack

```powershell
# Terminal 1 — your API (restart after pulling latest code)
cargo run -p inertia-api

# Terminal 2 — web UI
cd apps/web; npm run dev
```

Create a profile in the app. P2P starts in the background (or call `POST /p2p/start`).

Optional — fixed P2P port for your host:

```powershell
$env:INERTIA_P2P_ANNOUNCE = "/ip4/127.0.0.1/tcp/9001"
# Then POST /p2p/start with listen_port 9001 via API or restart after we add UI
```

### 2. Start the Docker peer

```powershell
docker compose up --build -d
```

### 3. Bootstrap the Docker profile + invite

```powershell
powershell -ExecutionPolicy Bypass -File scripts/p2p-docker-peer.ps1
```

Copy the **invite link** from the output.

### 4. Accept on your local app

1. Open `http://localhost:5173/invite`
2. Paste the link or payload
3. Verify **safety code**
4. Accept (Docker peer must stay up; your local P2P must be running)

### 5. Docker peer posts to your feed

```powershell
powershell -ExecutionPolicy Bypass -File scripts/p2p-docker-post.ps1 -Body "Hello from Docker!"
```

### 6. See it locally

1. Ensure both APIs still have P2P running
2. If the post did not arrive, open **Outbox** on the Docker side (port 4784 is API-only — use curl or check host outbox if you posted the other way)
3. On your **Feed**, click **Reload**
4. You should see the post from `Docker Peer` (text only; images are not synced yet)

### Troubleshooting

| Problem | Likely cause |
|---------|----------------|
| Invite accept fails | Docker peer not up, P2P not started, or wrong safety code |
| Post not in feed | Peers not connected; retry outbox on sender; both need `peer_id` on contact |
| Connection timeout | Wrong multiaddr; use `INERTIA_P2P_ANNOUNCE=/ip4/127.0.0.1/tcp/9002` for Docker |
| Image missing | Peer offline or blob transfer failed — retry by reconnecting; check sender has the photo locally |

### Manual API checks

```powershell
# Docker peer health
curl http://127.0.0.1:4784/health

# Your contacts
curl http://127.0.0.1:4783/contacts

# Docker outbox
curl http://127.0.0.1:4784/outbox
```

---

## Reverse flow (you invite Docker)

Harder from inside Docker (localhost ≠ host). Prefer **Docker invites, you accept** (above).

If you invite Docker instead, set on your host before creating the invite:

```powershell
$env:INERTIA_P2P_ANNOUNCE = "/ip4/host.docker.internal/tcp/9001"
```

Then Docker can dial your host P2P when accepting.

---

## What we fixed for this experiment

- **Posts attempt P2P send** on publish (like messages), not only outbox queue.
- **`INERTIA_P2P_ANNOUNCE`** for reachable invite addresses.
- **Docker compose + scripts** for a second peer.

## Next steps (real internet)

1. ~~Outbox auto-retry when `PeerConnected`~~
2. ~~P2P blob transfer for `media_ref`~~
3. Optional relay hardening (reservation limits)
4. Thumbnails + orphan blob GC (Phase 6)

See [VISION.md](./VISION.md) § P2P Transport.
