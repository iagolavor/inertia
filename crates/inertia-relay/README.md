# inertia-relay

libp2p **circuit relay v2** for Inertia. Run it on a small VPS you control so friends behind home NAT can reach each other.

**Connectivity only.** No SQLite, no user profiles, no posts, no keys, no decrypted content. The relay never sees message payloads.

---

## What you need

| Item | Notes |
|------|--------|
| VPS | Any Linux host with a public IP (1 vCPU / 1 GB RAM is enough for a small circle) |
| Port **9000/tcp** | Open on the VPS firewall and your cloud provider security group |
| Docker (recommended) | Or Rust 1.75+ if you build the binary yourself |

Each **Inertia user** still runs `inertia-api` on their own device. The relay is not a social server.

---

## Deploy on a VPS (Docker)

On the VPS:

```bash
git clone https://github.com/iagolavor/inertia.git
cd inertia/docker/relay
docker compose up -d --build
docker compose logs -f
```

Open the relay port:

```bash
sudo ufw allow 9000/tcp
sudo ufw status
```

From your laptop, confirm the port is reachable:

```bash
nc -vz YOUR_VPS_IP 9000
```

Identity data is stored in the Docker volume `relay-data` (mapped to `/data` in the container). **Do not delete it** unless you intend to rotate the relay peer id (every client must update their pinned multiaddr if you do).

---

## Client multiaddr

On first start the relay logs lines like:

```text
relay listening address=/ip4/0.0.0.0/tcp/9000
share with clients: /ip4/0.0.0.0/tcp/9000/p2p/12D3Koo…
```

Replace `0.0.0.0` with your VPS **public IP** and copy the full string:

```text
/ip4/YOUR_VPS_IP/tcp/9000/p2p/RELAY_PEER_ID
```

**On each Inertia device**

1. Open **Settings → Connection**
2. Paste into **Relay multiaddr**
3. Save and wait for **Relay OK** in the header. Before generating an invite, wait until **Generate** is enabled (relay reservation active — see `GET /invite/readiness` or the Friends add screen).

Or set an environment variable when starting the API (overrides saved settings):

```bash
export INERTIA_RELAY="/ip4/YOUR_VPS_IP/tcp/9000/p2p/RELAY_PEER_ID"
```

Invites you send include your relay address. Friends who accept get it applied automatically on their device.

Connection architecture (relay session vs reservation, invite bootstrap): [docs/RELAY-CONNECTIVITY.md](../../docs/RELAY-CONNECTIVITY.md).

---

## Run locally (development)

From the repo root:

```bash
cargo run -p inertia-relay
```

Defaults:

- Listen: `0.0.0.0:9000`
- Data dir: `./relay-data` (creates `relay_identity.key` on first run)

Custom listen address:

```bash
INERTIA_RELAY_ADDR=127.0.0.1:9000 cargo run -p inertia-relay
```

Use the logged multiaddr in client **Settings → Connection** (with `127.0.0.1` only for same-machine tests).

---

## Environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `INERTIA_RELAY_ADDR` | `0.0.0.0:9000` | Listen host:port (or full multiaddr) |
| `INERTIA_RELAY_DATA_DIR` | `./relay-data` | Directory for `relay_identity.key` |
| `INERTIA_RELAY_PUBLIC_ADDR` | _(auto from identify)_ | Routable multiaddr advertised in reservation responses (e.g. `/ip4/YOUR_VPS_IP/tcp/9000`). Set when auto-detect is wrong. |
| `INERTIA_RELAY_MAX_RESERVATIONS` | `128` | Global reservation cap (max inbound-dialable peers at once) |
| `INERTIA_RELAY_MAX_RESERVATIONS_PER_PEER` | `4` | Per-peer reservation cap |
| `INERTIA_RELAY_MAX_CIRCUITS` | `64` | Global active circuit cap (concurrent relayed friend paths) |
| `INERTIA_RELAY_MAX_CIRCUITS_PER_PEER` | `4` | Per-source-peer circuit cap (limits one poster fanning out via relay) |
| `RUST_LOG` | `inertia_relay=info` | Log verbosity |

Docker Compose sets these in [`docker/relay/docker-compose.yml`](../../docker/relay/docker-compose.yml).

---

## Build a release binary (no Docker)

On the VPS or your build machine:

```bash
cargo build --release -p inertia-relay
# binary: target/release/inertia-relay
```

Run with systemd or your process manager, pointing `INERTIA_RELAY_DATA_DIR` at a persistent directory.

---

## Troubleshooting

| Symptom | What to check |
|---------|----------------|
| Clients never connect | Multiaddr uses **public** IP and the correct `/p2p/` peer id from logs |
| Port closed | `ufw`, cloud security group, `docker compose ps` |
| Relay peer id changed | Volume was wiped; update every client's relay multiaddr |
| **Relay OK** never appears | API running on the device, relay reachable on `:9000`, multiaddr pasted exactly |
| Invite generate disabled / accept fails to reach inviter | Inviter lacks inbound **reservation** (circuit slot), not just Relay OK. Redeploy relay if reservations lack external addresses; set `INERTIA_RELAY_PUBLIC_ADDR` on the VPS |

---

## Do not run on the VPS

- `inertia-api` or SQLite (local-first data stays on each user's device)
- The Svelte web app as a shared login server
- Any service that stores user posts or private keys

---

## License

AGPL-3.0-or-later (same as the Inertia workspace).
