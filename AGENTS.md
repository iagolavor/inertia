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
- **Ephemeral content**: posts 48h, messages 7d, invites 15min single-use.
- **P2P**: libp2p strict mode; friends = contacts; posts fan-out to all contacts.

## Phased delivery

| Phase | Status | Scope |
|-------|--------|-------|
| 0–2 | Done | Rust core, P2P, storage |
| 3–4 | **Current** | SvelteKit web UI, invite flow, feed, profile |
| 5 | Next | **Capacitor** mobile shell |
| 6 | Partial | Posts/feed (in progress), on-demand friend profiles |

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
4. **Run `inertia-api` on device** — hardest part: bundle Rust as a sidecar, JNI/FFI, or foreground service. Desktop runs `cargo run -p inertia-api`; mobile needs a native integration plan.
5. Optional: `@capacitor/camera`, `@capacitor/filesystem` for profile photos instead of `<input type="file">`.

### What not to do before Capacitor

- Avoid SSR-only SvelteKit features.
- Avoid APIs that assume Node.js on the server.
- Keep using `api.ts` as the single HTTP boundary so the base URL can switch per platform.

## Commands

```bash
# Frontend
cd apps/web && npm run dev

# Backend
cargo run -p inertia-api

# Checks
cargo check -p inertia-core -p inertia-api
cd apps/web && npm run check
```
