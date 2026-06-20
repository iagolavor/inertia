<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/logo-dark.png" />
    <img src="docs/logo-light.png" alt="Inertia" width="200" />
  </picture>
</p>

**A local-first, P2P, ephemeral social network**

No central accounts. No ads. No algorithms. Your data stays on your device.

> Active prototype. Each user runs the API and P2P stack **locally**. An optional **VPS relay** (`inertia-relay`) helps friends connect across NAT — it never stores posts, keys, or profiles.

---

## About

**Inertia** is a minimalist alternative to traditional social networks: share photos and posts with people you trust, delivered directly between devices when both are online.

Instead of signing up on a server, your phone or computer generates a **cryptographic identity** (Ed25519/X25519 keypair). Your display name is just a local label. You add friends through signed invites (link or QR), with a safety code and short expiry.

The feed is **chronological** and limited to your connections. Posts and messages expire after **7 days** by default. If you want, you can keep a local history and **back up** your feed to continue on another device — always under your control.

Further reading:

- [Vision & architecture](docs/VISION.md)
- [Design philosophy](docs/DESIGN.md)
- [VPS relay operations](docs/VPS-RELAY.md)
- [P2P experiment (Docker)](docs/P2P-EXPERIMENT.md)

---

## Philosophy

| Principle | What it means in practice |
|-----------|---------------------------|
| **Local-first** | Posts, messages, profile, and photos live in SQLite + files on disk (`./data`). |
| **Ephemerality** | Content fades over time; the system does not build a permanent archive by default. |
| **Closed circle** | You only connect with people you invite. No global search, hashtags, or trending. |
| **Direct P2P** | Delivery peer-to-peer when both sides are online; optional VPS relay for NAT traversal. |
| **Transparency** | Separate **API** and **P2P** status; relay health; outbox retries visible. |
| **Zero tracking** | No analytics, no central user database, no corporate intermediaries. |

### What Inertia is **not**

- Not an Instagram clone (no stories, reels, public likes, or follower counts).
- Not an infinite feed optimized for retention.
- Not real-time delivery — it is **asynchronous**, like messages between close friends.
- Not a cloud social network — no central user database or hosted accounts.

- Not a cloud backup service; **feed backup** is a JSON file you export yourself.

---

## Features (current state)

- **Local identity** — one identity per install, generated on-device.
- **Invites (v2)** — link + QR with signed **relay multiaddr** so new friends bootstrap the same network on accept; 15-minute expiry, single-use, safety code. Invites require **Relay OK** on the inviter.
- **VPS relay** — `inertia-relay` on a small server you control (Docker); connectivity only, one TCP port.
- **Connection settings** — relay multiaddr, listen port, invite announce, shareable multiaddr.
- **Feed** — text and/or photo posts, chronological, 7-day TTL.
- **Profile** — personal photo grid (stored locally).
- **Messages** — P2P DMs with 7-day expiry.
- **Outbox** — delivery queue with retry and visible status.
- **Optional history** — accumulate feed locally + export/restore backup.
- **UI** — SvelteKit PWA with light/dark theme; Feed · Profile · Settings navigation.

---

## Tech stack

### Backend (Rust)

| Component | Role |
|-----------|------|
| **[inertia-core](crates/inertia-core/)** | Identity, invites, SQLite, blobs, expiry, libp2p |
| **[inertia-api](crates/inertia-api/)** | Local HTTP bridge (`127.0.0.1:4783`) between browser and core |
| **[inertia-relay](crates/inertia-relay/)** | Optional VPS libp2p circuit relay (connectivity only) |
| **libp2p** | TCP, Noise, Yamux, relay client, DCUtR, request-response |
| **rusqlite** | Embedded local database |
| **ed25519-dalek / x25519-dalek** | Signatures and key agreement |
| **ChaCha20-Poly1305** | End-to-end envelope encryption |
| **Axum + Tokio** | Async HTTP API |

### Frontend (TypeScript)

| Component | Role |
|-----------|------|
| **[apps/web](apps/web/)** | SvelteKit UI (PWA, SSR disabled) |
| **Svelte 5** | Reactive components |
| **Vite 6** | Dev server and build |
| **adapter-static** | Static export for simple hosting |

### Local data layout

```
data/
  inertia.db      # contacts, inbox, outbox, posts, feed archive
  blobs/          # images keyed by SHA-256 hash (deduplicated)
```

---

## Architecture

```
┌─────────────────────┐         ┌─────────────────────┐
│  Your device        │         │  Friend's device    │
│  Browser (Svelte)   │         │  Browser (Svelte)   │
│  inertia-api :4783  │         │  inertia-api :4783  │
│  inertia-core+P2P   │         │  inertia-core+P2P   │
└──────────┬──────────┘         └──────────┬──────────┘
           │  E2E encrypted P2P            │
           └────────────┬─────────────────┘
                        │ circuit relay (optional)
                        ▼
              ┌─────────────────────┐
              │  VPS (you control)  │
              │  inertia-relay :9000│
              │  connectivity only  │
              └─────────────────────┘
```

**Local per device:** SQLite, blobs, identity, and keys never leave the machine. The API binds `127.0.0.1` only.

**Invite links (v2)** carry:

- **`multiaddrs`** — how to reach the inviter (relay circuit paths when NAT blocks direct dial).
- **`relay_multiaddr`** — signed shared relay address; applied on the accepter's device so they can connect without a separate relay handoff.

Invites are only generated when the inviter shows **Relay OK** in the header.

Typical flow: you publish a post → stored locally → encrypted and queued in the outbox → delivered over P2P when your friend is online → expires at TTL (unless local history is enabled).

---

## Repository structure

```
inertia/
├── crates/
│   ├── inertia-core/     # Rust library: storage, P2P, crypto
│   ├── inertia-api/      # Local API binary
│   └── inertia-relay/    # Optional VPS libp2p circuit relay
├── docker/
│   └── relay/            # Docker Compose for VPS deploy
├── apps/
│   └── web/              # SvelteKit frontend
├── docs/
│   ├── VISION.md         # Technical vision and decisions
│   ├── WINDOWS-SETUP.md  # Windows end-user guide (prebuilt zip)
│   ├── VPS-RELAY.md      # Relay deploy and brother test guide
│   └── DESIGN.md         # Visual and UX philosophy
├── scripts/              # Dev helpers, release tooling, scripts/windows/ (zip contents)
└── package.json          # Root npm scripts (api, web, relay)
```

---

## Getting started on Windows

No Rust, Node, or Git required.

1. Download **[inertia-windows-x64.zip](https://github.com/iagolavor/inertia/releases/latest)** from GitHub Releases.
2. Extract the folder anywhere.
3. Double-click **`run.cmd`** — opens [http://127.0.0.1:4783](http://127.0.0.1:4783).
4. To update later, double-click **`update.cmd`** (keeps your `data/` folder).

Details and troubleshooting: **[docs/WINDOWS-SETUP.md](docs/WINDOWS-SETUP.md)**.

---

## Quick start (developers)

For macOS, Linux, or [developing on Windows](docs/WINDOWS-SETUP.md#developing-inertia-on-windows).

### Prerequisites

- **[Rust](https://rustup.rs/)** 1.75+ (`cargo`, `rustc`)
- **[Node.js](https://nodejs.org/)** 20 LTS+ (`npm`)
- **Git**

Open a new terminal after installing Rust/Node so `PATH` is updated.

### 1. Clone the repository

```bash
git clone https://github.com/iagolavor/inertia.git
cd inertia
```

### 2. Run Inertia

**Daily use (low memory):** release API + static preview — not Vite dev (~400 MB+).

**Terminal 1 — API** (repo root):

```bash
npm run api:release
```

**Terminal 2 — web UI** (repo root; install once, build once, then serve):

```bash
cd apps/web && npm install
cd ../..
npm run web:build
npm run web:preview
```

Open [http://localhost:4173](http://localhost:4173). The API listens on `http://127.0.0.1:4783`. Data is stored in `./data` (gitignored).

> **Note:** Large `node.exe` entries in Task Manager are usually your **editor** (TypeScript language server), not Inertia. The API itself is `inertia-api.exe` (~12 MB release).

### 3. First-time setup

1. Create your profile (display name) on the **Profile** tab.
2. **Settings → Connection** — set your **relay multiaddr** (full `/ip4/…/tcp/9000/p2p/…` string from [VPS-RELAY.md](docs/VPS-RELAY.md)). Header should show **Relay OK**.
3. Under **Friends** (⋯ menu), generate an invite and share the link or QR.
4. Your friend opens the link, verifies the safety code, and **Accept** — the relay from the invite is applied automatically.
5. Post on **Feed**; direct messages live under **Messages**.

### Optional: VPS relay

Deploy `inertia-relay` on a VPS you control (see [docs/VPS-RELAY.md](docs/VPS-RELAY.md)):

```bash
cd docker/relay
docker compose up -d --build
```

Open TCP **9000** on the VPS firewall. Copy the relay peer id from the logs into client Settings.

**Root scripts:**

| Command | Description |
|---------|-------------|
| `npm run api:release` | Start optimized `inertia-api` (**recommended for daily use**) |
| `npm run api` | Start debug `inertia-api` (faster rebuilds while hacking Rust) |
| `npm run api:stop` | Kill process on port 4783 (Windows) |
| `npm run api:restart` | Restart the API (debug) |
| `npm run relay` | Run `inertia-relay` locally (dev) |
| `npm run vps:ssh` | SSH to VPS (`INERTIA_VPS_HOST` in `.env`) |
| `npm run web:build` | Production static build |
| `npm run web:preview` | Serve built web UI (**recommended for daily use**) |
| `npm run web` | Vite dev server in `apps/web` (UI development only) |

Copy [`.env.example`](.env.example) to `.env` for VPS SSH defaults (gitignored).

### VS Code / Cursor

- **`run`** — release API + static web preview (low memory; run `npm run web:build` first if `build/` is missing).
- **`dev`** — debug API + Vite dev server (use only while editing code).

The **inertia-api** task stops any running instance before starting. Use **api:stop** if port 4783 is stuck.

---

## Development

When editing Rust or Svelte, use the fast-rebuild dev servers (higher RAM — Vite dev alone is ~200 MB):

```bash
# Terminal 1 — API (debug, faster compile)
npm run api

# Terminal 2 — frontend (HMR)
cd apps/web && npm install && npm run dev
# open http://localhost:5173
```

```bash
# Core tests
cargo test -p inertia-core

# Build the full Rust workspace
cargo build --workspace

# Frontend typecheck
cd apps/web && npm run check
```

Optional environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `INERTIA_DATA_DIR` | `./data` | Local data directory |
| `INERTIA_API_ADDR` | `127.0.0.1:4783` | API listen address |
| `INERTIA_WEB_DIR` | `./web` (beside exe) | Static UI folder; when set, API serves the app on the same port |
| `INERTIA_P2P_LISTEN_PORT` | `4784` | libp2p TCP listen port |
| `INERTIA_RELAY` | — | Relay multiaddr (overrides Settings) |
| `INERTIA_P2P_ANNOUNCE` | — | Comma-separated multiaddrs for invites |
| `INERTIA_WEB_ORIGIN` | — | Base URL for invite links |
| `INERTIA_VPS_HOST` | — | VPS IP for `npm run vps:ssh` (`.env`) |

---

## Roadmap

| Phase | Status | Focus |
|-------|--------|-------|
| 0–1 | Done | Vision, identity, storage, expiry |
| 2 | Done | libp2p, outbox, messaging |
| 3 | Done | SvelteKit UI + local API |
| 4 | In progress | Invites, feed, profile, backup |
| 4b | In progress | VPS relay, relay client, invite v2 with embedded relay |
| 5 | Planned | Mobile shell (Capacitor) |
| 6 | Planned | P2P blob sync, thumbnails, orphan file GC |

See [docs/VISION.md](docs/VISION.md) for technical decisions and open questions.

---

## Contributing

This project is in its early stages. Issues and PRs are welcome — please read the [vision](docs/VISION.md) and [git workflow](docs/GIT-WORKFLOW.md) before opening a PR.

By contributing, you agree that your work is licensed under the [AGPL-3.0-or-later](LICENSE) license.

---

## License

Licensed under [GNU Affero General Public License v3.0 or later](LICENSE) — community-built software; share improvements when you run or distribute modified versions.
