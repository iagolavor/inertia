# Inertia — Agent Guide

This repo has two specialized agent contexts. Cursor rules activate them automatically when you edit matching files.

| Agent | Rule file | Scope |
|-------|-----------|-------|
| **Rust backend** | `.cursor/rules/rust-backend.mdc` | `crates/**`, `tools/**`, root `Cargo.toml` |
| **Svelte frontend** | `.cursor/rules/svelte-frontend.mdc` | `apps/web/**` |

Read `docs/VISION.md` for product/architecture and `docs/DESIGN.md` for UI philosophy.

## Architecture (short)

```
apps/web (SvelteKit PWA)  →  HTTP /api  →  inertia-api  →  inertia-core (SQLite + libp2p)
```

- **Local-first**: no cloud backend. API binds `127.0.0.1:4783` on the user's machine.
- **Ephemeral content**: posts and messages 7d, invites 15min single-use.
- **P2P**: libp2p strict mode; friends = contacts; posts fan-out to all contacts.

## Phased delivery

| Phase | Status | Scope |
|-------|--------|-------|
| 0–2 | Done | Rust core, P2P, storage |
| 3–4 | **Current** | SvelteKit web UI, invite flow, feed, profile, settings, backup |
| 5 | Next | **Capacitor** mobile shell |
| 6 | Planned | P2P blob sync, thumbnails, orphan file GC |

**Yes — we are building the web app first.** That is intentional. The Svelte app is the shared UI for web and mobile.

## Capacitor + Ionic

**Capacitor** wraps the built Svelte SPA in a native WebView (iOS/Android). **Ionic** is an optional UI kit — we use **Svelte** for UI, not Ionic components. Capacitor alone is enough for “web + app from one codebase.”

### Already Capacitor-ready

- `@sveltejs/adapter-static` with SPA `fallback: 'index.html'`
- `ssr = false`, `prerender = true` (client-only app)
- All data via `apps/web/src/lib/api.ts` → `/api` (no server-side Svelte data fetching)

### Phase 5 work (not started)

1. Add `@capacitor/core` + platform packages under `apps/web` (or `apps/mobile` wrapper).
2. Point Capacitor `webDir` at SvelteKit `build/` output.
3. Replace dev proxy: use `CapacitorHttp` or absolute URL to local API when packaged.
4. **Run `inertia-api` on device** — hardest part: bundle Rust as a sidecar, JNI/FFI, or foreground service. Desktop runs `npm run api:release` (or `cargo run --release -p inertia-api`); mobile needs a native integration plan.
5. Optional: `@capacitor/camera`, `@capacitor/filesystem` for profile photos instead of `<input type="file">`.

### What not to do before Capacitor

- Avoid SSR-only SvelteKit features.
- Avoid APIs that assume Node.js on the server.
- Keep using `api.ts` as the single HTTP boundary so the base URL can switch per platform.

## Commands

**Git:** integration branch is `development`; feature branches use `feature/<name>`; **PRs target `development` by default**. See [docs/GIT-WORKFLOW.md](docs/GIT-WORKFLOW.md).

### Recommended — daily use (low memory)

Release API (~12 MB) + static web preview. Avoid Vite dev unless you are editing the UI.

```powershell
# Terminal 1 — API
npm run api:release

# Terminal 2 — web (build once, then serve)
npm run web:build
npm run web:preview
# http://localhost:4173
```

**Cursor tasks:** `run` (release API + preview). Run `web:build` first if `apps/web/build/` is missing.

**Separate OS window (outside Cursor)**

```powershell
npm run api:window
```

### Development — editing code (higher memory)

Fast rebuilds; Vite dev holds ~200 MB for HMR and file watching.

1. `Terminal` → `Run Task…` → **`dev`** — debug API + Vite in two tabs  
2. Or manually:

```bash
# Terminal 1 — API (debug)
cargo run -p inertia-api

# Terminal 2 — web (Vite dev)
cd apps/web && npm run dev
```

**If rebuild fails with `Access is denied` on `inertia-api.exe`**, a previous instance is still running:

```powershell
npm run api:stop
# then start again via task or: npm run api
```
