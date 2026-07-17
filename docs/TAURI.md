# Tauri desktop shell

Desktop twin of the Android on-device install: one app window starts **`inertia-api`** as a sidecar and loads the Svelte UI from `http://127.0.0.1:4783`.

| | **Desktop (Tauri)** |
|---|---------------------|
| UI | Served by local `inertia-api` |
| API | Sidecar on `127.0.0.1:4783` |
| Data | OS app data dir (`INERTIA_DATA_DIR`) |
| Web assets | Bundled `resources/web` (`INERTIA_WEB_DIR`) |

Same Svelte app as web/Android ([`apps/web`](../apps/web)). No forked product UI.

## Prerequisites

### All platforms

- [Node.js](https://nodejs.org/) 20 LTS+
- Rust toolchain (`rustup`)
- From repo root: workspace builds `inertia-api` with `cargo build --release -p inertia-api`

### Linux

Install Tauri system libraries ([prerequisites](https://tauri.app/start/prerequisites/)):

```bash
# Fedora
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel gcc

# Debian / Ubuntu
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
  librsvg2-dev patchelf build-essential curl wget file libssl-dev
```

### Windows

- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (C++ workload)
- WebView2 (usually preinstalled on Windows 10/11)

macOS packaging is deferred.

## Commands (repo root)

```bash
# Prepare sidecar + web assets, then run the shell
npm run desktop:dev

# Production installers / bundles under apps/desktop/src-tauri/target/release/bundle/
npm run desktop:build

# Only copy API + web into apps/desktop/src-tauri/{binaries,resources}
npm run desktop:package
```

`desktop:package` / `desktop:dev` / `desktop:build` invoke [`scripts/package-desktop.sh`](../scripts/package-desktop.sh) (Linux/macOS) or [`scripts/package-desktop.ps1`](../scripts/package-desktop.ps1) (Windows).

## Layout

```
apps/desktop/
  splash/                 # brief "Starting…" page before navigate to API
  src-tauri/
    binaries/             # gitignored: inertia-api-<target-triple>
    resources/web/        # gitignored: copy of apps/web/build
    src/lib.rs            # spawn sidecar, health wait, navigate, kill on exit
    tauri.conf.json
```

## Lifecycle

1. Shell resolves OS **app data** dir and bundled (or dev) **web** dir.
2. Spawns sidecar `binaries/inertia-api` with:
   - `INERTIA_DATA_DIR` - app data (not repo `./data`)
   - `INERTIA_WEB_DIR` - packaged UI
   - `INERTIA_API_ADDR=127.0.0.1:4783`
3. Polls `GET /api/health` (up to ~45s).
4. Navigates the main window to `http://127.0.0.1:4783/`.
5. On exit, kills the sidecar so port 4783 is freed.

## Smoke checklist

Linux (verified on Fedora with WebKitGTK 4.1):

- [x] `npm run desktop:dev` opens a native window (not an external browser)
- [x] Health + UI at `http://127.0.0.1:4783`; data under OS app data (`~/.local/share/social.inertia.app` on Linux)
- [x] Closing the window / SIGTERM on the shell stops `inertia-api` (port 4783 freed; Linux uses `PR_SET_PDEATHSIG`)
- [ ] Invite / Messages / Settings relay behave like the Windows zip + browser flow

Windows (run before treating installers as release-ready):

- [ ] Prerequisites above, then `npm run desktop:package` and `npm run desktop:build`
- [ ] NSIS/MSI (or exe) under `apps/desktop/src-tauri/target/release/bundle/`
- [ ] Same smoke circle as Linux (identity, invite, Messages, Settings relay)

If port 4783 is already taken (`npm run api:release`, zip `run.cmd`, etc.), the shell exits with a clear error. Stop the other process first (`npm run api:stop` on this repo).

## Relation to Windows zip

[`npm run package:windows`](../docs/WINDOWS-SETUP.md) remains the zip + `run.cmd` path. Tauri is the simpler double-click install UX; zip stays until desktop installers are the default release artifact.

### Windows build path (summary)

```powershell
# From repo root (PowerShell)
npm run desktop:build
# -> scripts/package-desktop.ps1 (release inertia-api + web:build)
# -> tauri build (NSIS under src-tauri/target/release/bundle/)
```

macOS packaging is deferred.

## Security notes

- Cleartext `http://127.0.0.1` only (same as Capacitor / zip). See [SECURITY-TODO.md](./SECURITY-TODO.md) for localhost API auth before exposing beyond loopback.
- Single instance: do not run zip `inertia-api` and the Tauri sidecar on the same port at once.
