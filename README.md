<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/logo-dark.png" />
    <img src="docs/logo-light.png" alt="Inertia" width="200" />
  </picture>
</p>

**A local-first, P2P, ephemeral social network**

No central accounts. No ads. No algorithms. Your data stays on your device.

> Active prototype. The API and P2P stack run locally; there are no cloud services.

---

## About

**Inertia** is a minimalist alternative to traditional social networks: share photos and posts with people you trust, delivered directly between devices when both are online.

Instead of signing up on a server, your phone or computer generates a **cryptographic identity** (Ed25519/X25519 keypair). Your display name is just a local label. You add friends through signed invites (link or QR), with a safety code and short expiry.

The feed is **chronological** and limited to your connections. Posts and messages expire after **7 days** by default. If you want, you can keep a local history and **back up** your feed to continue on another device — always under your control.

Further reading:

- [Vision & architecture](docs/VISION.md)
- [Design philosophy](docs/DESIGN.md)
- [P2P experiment (Docker)](docs/P2P-EXPERIMENT.md)

---

## Philosophy

| Principle | What it means in practice |
|-----------|---------------------------|
| **Local-first** | Posts, messages, profile, and photos live in SQLite + files on disk (`./data`). |
| **Ephemerality** | Content fades over time; the system does not build a permanent archive by default. |
| **Closed circle** | You only connect with people you invite. No global search, hashtags, or trending. |
| **Direct P2P** | Delivery peer-to-peer when both sides are online; failures stay visible in the outbox. |
| **Transparency** | Online/offline state is shown; retries and expirations are not hidden. |
| **Zero tracking** | No analytics, no central user database, no corporate intermediaries. |

### What Inertia is **not**

- Not an Instagram clone (no stories, reels, public likes, or follower counts).
- Not an infinite feed optimized for retention.
- Not real-time delivery — it is **asynchronous**, like messages between close friends.
- Not a cloud backup service; **feed backup** is a JSON file you export yourself.

---

## Features (current state)

- **Local identity** — one identity per install, generated on-device.
- **Invites** — link + QR, 15-minute expiry, single-use, safety code.
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
| **libp2p** | TCP, Noise, Yamux, request-response, identify |
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
┌─────────────────────────────────────────────────────────┐
│  Browser (SvelteKit PWA)                                │
│  Feed · Profile · Settings · Friends · Messages         │
└───────────────────────────┬─────────────────────────────┘
                            │ HTTP /api → Vite proxy
                            ▼
┌─────────────────────────────────────────────────────────┐
│  inertia-api  (127.0.0.1:4783)                          │
│  Local REST — runs only on the user's machine           │
└───────────────────────────┬─────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│  inertia-core                                           │
│  SQLite · blobs · identity · expiry · outbox            │
│  libp2p — direct friend connections when online         │
└─────────────────────────────────────────────────────────┘
```

Typical flow: you publish a post → stored locally → encrypted and queued in the outbox → delivered over P2P when your friend is online → expires at TTL (unless local history is enabled).

---

## Repository structure

```
inertia/
├── crates/
│   ├── inertia-core/     # Rust library: storage, P2P, crypto
│   └── inertia-api/      # Local API binary
├── apps/
│   └── web/              # SvelteKit frontend
├── docs/
│   ├── VISION.md         # Technical vision and decisions
│   └── DESIGN.md         # Visual and UX philosophy
├── scripts/              # Utilities (stop API, etc.)
└── package.json          # Root npm scripts (api, web)
```

---

## Prerequisites

- **[Rust](https://rustup.rs/)** 1.75+ (`cargo`, `rustc`)
- **[Node.js](https://nodejs.org/)** 20 LTS+ (`npm`)

Works on **Windows**, **macOS**, and **Linux**. On Windows, open a new terminal after installing Rust/Node so `PATH` is updated.

---

## Quick start

### 1. Clone the repository

```bash
git clone https://github.com/iagolavor/inertia.git
cd inertia
```

### 2. Start the local API

```bash
# From the repo root
npm run api
# or: cargo run -p inertia-api
```

The API listens on `http://127.0.0.1:4783`. Data is stored in `./data` (gitignored).

### 3. Install and run the frontend

```bash
cd apps/web
npm install
npm run dev
```

Open [http://localhost:5173](http://localhost:5173).

### 4. First-time setup

1. Create your profile (display name) on the **Profile** tab.
2. Under **Friends** (⋯ menu), generate an invite and share the link or QR.
3. Your friend accepts with the safety code — the inviter must be **online** with P2P running.
4. Post on **Feed**; direct messages live under **Messages**.

**Root scripts:**

| Command | Description |
|---------|-------------|
| `npm run api` | Start `inertia-api` |
| `npm run api:stop` | Kill process on port 4783 (Windows) |
| `npm run api:restart` | Restart the API |
| `npm run web` | Run `npm run dev` in `apps/web` |

### VS Code / Cursor

The **inertia-api** task in [`.vscode/tasks.json`](.vscode/tasks.json) stops any running instance before starting. Use **dev** to run API + web in parallel.

---

## Development

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

---

## Roadmap

| Phase | Status | Focus |
|-------|--------|-------|
| 0–1 | Done | Vision, identity, storage, expiry |
| 2 | Done | libp2p, outbox, messaging |
| 3 | Done | SvelteKit UI + local API |
| 4 | In progress | Invites, feed, profile, backup |
| 5 | Planned | Mobile shell (Capacitor) |
| 6 | Planned | P2P blob sync, thumbnails, orphan file GC |

See [docs/VISION.md](docs/VISION.md) for technical decisions and open questions.

---

## Contributing

This project is in its early stages. Issues and PRs are welcome — please read the [vision](docs/VISION.md) before proposing features that conflict with core principles (centralization, algorithmic feeds, permanent archives by default, etc.).

---

## License

[MIT](LICENSE)
