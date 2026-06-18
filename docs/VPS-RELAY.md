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

## Brother test checklist (Phase 3 preview)

1. Deploy relay on VPS; confirm port open.
2. **You:** start API + web, set relay multiaddr in Settings, create profile.
3. **Brother:** same relay config, create profile.
4. **You:** Friends → Generate invite → send link.
5. **Brother:** open invite, verify safety code, Accept.
6. **Brother:** post text; **you:** Feed reload.
7. **You:** DM; **brother:** Messages.
8. Restart one API; confirm delivery resumes without a new invite.

Text-only posts until P2P blob sync lands (Phase 4 in milestone doc).

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

| Variable | Default | Purpose |
|----------|---------|---------|
| `INERTIA_RELAY_ADDR` | `0.0.0.0:9000` | Relay listen host:port |
| `INERTIA_RELAY_DATA_DIR` | `./relay-data` | Relay identity persistence |
| `RUST_LOG` | `inertia_relay=info` | Log verbosity |

Client-side (each device): `INERTIA_RELAY`, `INERTIA_P2P_LISTEN_PORT`, `INERTIA_P2P_ANNOUNCE` — see Settings → Connection.
