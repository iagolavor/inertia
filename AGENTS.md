# Inertia — Agent Guide

This repo has two specialized agent contexts. Cursor rules activate them automatically when you edit matching files.

| Agent | Rule file | Scope |
|-------|-----------|-------|
| **Rust backend** | `.cursor/rules/rust-backend.mdc` | `crates/**`, `tools/**`, root `Cargo.toml` |
| **Svelte frontend** | `.cursor/rules/svelte-frontend.mdc` | `apps/web/**` |

Read `docs/VISION.md` for product/architecture, `docs/LIVE-SYNC.md` for SSE and sync modules, and `docs/DESIGN.md` for UI philosophy.

## Copy (repo-wide)

**Never use em dashes (`—`).** Not in UI, docs, scripts (especially `scripts/windows/*.ps1`), errors, commits, or PRs. Use ASCII `-`, periods, commas, or colons. Unicode dashes break Windows PowerShell and are not project style.

## Architecture (short)

```
apps/web (SvelteKit PWA)  →  HTTP /api  →  inertia-api  →  inertia-core (SQLite + libp2p)
```

- **Local-first**: no cloud backend. API binds `127.0.0.1:4783` on the user's machine.
- **Live UI**: SSE (`GET /api/p2p/events`) plus `messages-sync`, `feed-sync`, `conversation-sync` in `apps/web/src/lib/`. See [docs/LIVE-SYNC.md](docs/LIVE-SYNC.md).
- **Ephemeral content**: posts and messages 7d, invites 15min single-use.
- **P2P**: libp2p strict mode; friends = contacts; posts fan-out to all contacts.

## Phased delivery

| Phase | Status | Scope |
|-------|--------|-------|
| 0–2 | Done | Rust core, P2P, storage |
| 3–4 | **Current** | SvelteKit web UI, invite flow, feed, profile, settings, backup |
| 5 | **In progress** | **Capacitor Android** — Stage B shell shipped in v0.10; polish + iOS remain |
| 6 | Planned | P2P blob sync, thumbnails, orphan file GC |

**Yes — we are building the web app first.** That is intentional. The Svelte app is the shared UI for web and mobile.

## Capacitor + Ionic

**Capacitor** wraps the built Svelte SPA in a native WebView (iOS/Android). **Ionic** is an optional UI kit — we use **Svelte** for UI, not Ionic components. Capacitor alone is enough for “web + app from one codebase.”

### Already Capacitor-ready

- `@sveltejs/adapter-static` with SPA `fallback: 'index.html'`
- `ssr = false`, `prerender = true` (client-only app)
- All data via `apps/web/src/lib/api.ts` → `/api` (no server-side Svelte data fetching)

### Capacitor Android (v0.10 — Stage B alpha)

**Shipped in v0.10.0** — see [docs/CAPACITOR.md](docs/CAPACITOR.md) for build commands and resume checklist.

| Mode | Status |
|------|--------|
| **Stage B** (API + UI on phone) | Working — `npm run android:stage-b`, separate device identity/DB, P2P + invite accept tested PC ↔ phone |
| **Stage A** (UI on phone, API on PC) | Supported — `adb reverse`, `npm run android:sync`; less exercised after Stage B |

**Done**

- `@capacitor/core` + Android project under `apps/web/android`
- `webDir` → SvelteKit `build/`; native shell uses absolute `http://127.0.0.1:4783/api` ([api-base.ts](apps/web/src/lib/api-base.ts))
- Bundled `inertia-api` on device: NDK arm64 cross-compile, `jniLibs` + `InertiaRuntime` / foreground `InertiaApiService`
- Splash → health wait → WebView at `127.0.0.1:4783`; invite deep links stay in-app (`InertiaWebViewClient`, `inertia://invite/…`)
- Invite accept: relay apply + dial waits, paste normalization ([invite-input.ts](apps/web/src/lib/invite-input.ts))

**Resume next** (mobile polish — branch from `development` as `feature/android-*`)

1. **P2pStatus on touch** — replace hover `title` tooltip with tap-to-expand panel in header
2. **Invite preview UX** — remove or fix misleading red offline dot on `ProfileHeader` (not inviter presence)
3. **Stage A smoke** — confirm `android:reverse` loop still works after Stage B changes
4. **Release** — Play signing, optional APK in CI (Windows zip only today)
5. **Optional** — `@capacitor/camera` / filesystem for profile photos; iOS shell; API auth on localhost ([SECURITY-TODO.md](docs/SECURITY-TODO.md))

### What not to do on mobile

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

### Android quick commands

```powershell
# Stage B — self-contained phone build (default for real-device testing)
npm run android:stage-b
npm run android:run

# Stage A — UI on phone, API on PC
npm run api:release
npm run android:sync
npm run android:reverse
npm run android:run
```

**Windows (end users):** [docs/WINDOWS-SETUP.md](docs/WINDOWS-SETUP.md) — download `inertia-windows-x64.zip`, `run.cmd`, `update.cmd`. No npm scripts.

**Releases:** push a `v*` tag ([scripts/release-tag.ps1](scripts/release-tag.ps1)); GitHub Actions builds the Windows zip. See [docs/RELEASE.md](docs/RELEASE.md).
