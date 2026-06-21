# VPS relay operations

Deploy **`inertia-relay`** on a small VPS you control. It is **connectivity only** — no SQLite, no user keys, no decrypted content.

See [MILESTONE-VPS-RELAY.md](./MILESTONE-VPS-RELAY.md) for the full client + relay roadmap.

## What the relay does

- Runs libp2p **circuit relay v2** on one TCP port (default **9000**)
- Persists a stable **relay peer id** under `INERTIA_RELAY_DATA_DIR`
- Logs reservations and circuits — never message payloads

Clients configure the relay multiaddr in **Settings → Connection** or via `INERTIA_RELAY`.

---

## Deploy with Docker (recommended)

On the VPS (Ubuntu 22.04+ assumed):

```bash
git clone https://github.com/iagolavor/inertia.git
cd inertia/docker/relay
docker compose up -d --build
docker compose logs -f
```

Open the firewall:

```bash
sudo ufw allow 9000/tcp
sudo ufw status
```

Smoke test from your laptop:

```bash
nc -vz YOUR_VPS_IP 9000
```

### Read the relay peer id

On first start the relay generates `relay-data/relay_identity.key` and logs lines like:

```
relay listening address=/ip4/0.0.0.0/tcp/9000
share with clients: /ip4/0.0.0.0/tcp/9000/p2p/12D3Koo…
```

Use the **public** IP in client config:

```text
/ip4/YOUR_VPS_IP/tcp/9000/p2p/RELAY_PEER_ID
```

Copy that into each client's **Relay multiaddr** field (Settings) or:

```bash
export INERTIA_RELAY="/ip4/YOUR_VPS_IP/tcp/9000/p2p/RELAY_PEER_ID"
```

Environment variables override saved settings when set.

---

## Run locally (dev)

```bash
cargo run -p inertia-relay
# or
INERTIA_RELAY_ADDR=127.0.0.1:9000 cargo run -p inertia-relay
```

Data directory defaults to `./relay-data`.

---

## Brother test checklist (Phase 3)

### 1. Deploy relay on VPS

Confirm port open (`nc -vz YOUR_VPS_IP 9000`). Copy relay multiaddr from logs.

### 2. Build and run on each machine

Each person runs **inertia-api** locally and opens the **built web UI** in a browser on the same machine. The API never leaves the device.

**Prerequisites**

| Tool | Windows | Linux |
|------|---------|-------|
| Rust | [rustup.rs](https://rustup.rs) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node 20+ | [nodejs.org](https://nodejs.org) | `sudo apt install nodejs npm` or nvm |

**Terminal 1 — API** (keep running; release build, low memory)

```powershell
# Windows (repo root)
npm run api:release
```

```bash
# Linux (repo root)
cargo run --release -p inertia-api
```

**Terminal 2 — web UI**

Recommended (daily use / brother test — static build, low memory):

```bash
npm run web:build
npm run web:preview
# default http://localhost:4173 — use --host so LAN devices can connect
```

Dev mode (only while editing the UI — Vite holds ~200 MB RAM):

```bash
npm run web
# open http://localhost:5173
```

On Windows, `web:preview` binds `0.0.0.0` via `--host`. Note your LAN IP (e.g. `192.168.1.10`) and the preview port.

### 3. Connection settings (both sides)

**Inviter** must show **Relay OK** before generating an invite.

**Accepter** — relay is applied automatically from the invite on Accept (no manual relay paste needed unless using `INERTIA_RELAY` env override).

Optional per device:

| Field | Example |
|-------|---------|
| Invite announce addresses | Your LAN/public reachability, e.g. `/ip4/192.168.1.10/tcp/4784` |
| Invite link base URL | `http://192.168.1.10:4173` (your preview URL, no trailing slash) |

### 4. Test flow

1. **You:** create profile, set relay in Settings (Relay OK), generate invite.
2. **Brother:** create profile, open invite — relay configured from link on accept.
3. **You:** Friends → Generate invite → send link (Signal/iMessage/etc.).
4. **Brother:** open link in his browser (must hit *his* local web UI via the base URL you configured).
5. **Brother:** verify safety code, Accept.
6. **Brother:** post text; **you:** Feed updates automatically (polls while the tab is open).
7. **You:** DM; **brother:** Messages.
8. Restart one API; confirm delivery resumes without a new invite.

Photo posts sync over P2P after the text envelope is delivered (blob push on ack, pull on reconnect).

### 5. Invite links without a shared URL

If you skip **Invite link base URL**, generated links use the `inertia://invite/…` scheme. The recipient can paste the payload at `/invite` on their local app instead of opening a web URL.

---

## Troubleshooting

| Symptom | Check |
|---------|--------|
| Clients never connect | Relay multiaddr uses **public** IP + correct peer id |
| Port closed | `ufw`, cloud provider security group, `docker compose ps` |
| Relay peer id changed | Wipe volume only if intentional — clients must update pinned id |
| P2P badge stuck idle | Both sides running API; relay reachable; invite addresses set |

---

## What not to run on the VPS

- `inertia-api` / SQLite (local-first stays on each device)
- The Svelte web app as a shared account server
- Any service that stores user posts or keys

---

## Related env vars

**Relay (VPS)**

| Variable | Default | Purpose |
|----------|---------|---------|
| `INERTIA_RELAY_ADDR` | `0.0.0.0:9000` | Relay listen host:port |
| `INERTIA_RELAY_DATA_DIR` | `./relay-data` | Relay identity persistence |
| `INERTIA_RELAY_MAX_RESERVATIONS` | `64` | Global reservation cap |
| `INERTIA_RELAY_MAX_RESERVATIONS_PER_PEER` | `4` | Per-peer reservation cap |
| `INERTIA_RELAY_MAX_CIRCUITS` | `32` | Global active circuit cap |
| `INERTIA_RELAY_MAX_CIRCUITS_PER_PEER` | `4` | Per-peer circuit cap |
| `RUST_LOG` | `inertia_relay=info` | Log verbosity |

**Client (each device)** — see also Settings → Connection

| Variable | Default | Purpose |
|----------|---------|---------|
| `INERTIA_RELAY` | — | Relay multiaddr clients dial |
| `INERTIA_P2P_LISTEN_PORT` | `4784` | Fixed TCP listen port |
| `INERTIA_P2P_ANNOUNCE` | — | Comma-separated multiaddrs for invites |
| `INERTIA_WEB_ORIGIN` | — | Base URL for invite links (overrides browser origin) |
