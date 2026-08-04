# Tauri shell (desktop + Android)

One native shell in [`apps/desktop`](../apps/desktop) starts **`inertia-api`** and loads the Svelte UI from `http://127.0.0.1:4783`.

| | **Desktop** | **Android** |
|---|-------------|----------------|
| UI | Served by local `inertia-api` | Same |
| API | Sidecar (`externalBin`) | `libinertia_api.so` via jniLibs + FGS |
| Data | OS app data dir (`INERTIA_DATA_DIR`) | App files dir (`inertia/data`) |
| Web assets | Bundled `resources/web` | Extracted from APK assets |

Same Svelte app ([`apps/web`](../apps/web)). No forked product UI across web, desktop, and Android.

Official Tauri sidecars are desktop-only. Android ships `inertia-api` as `libinertia_api.so` under `jniLibs`, runs it with `ProcessBuilder`, keeps a foreground service, waits for health, then opens the WebView at `http://127.0.0.1:4783`.

## Prerequisites

### All platforms

- [Node.js](https://nodejs.org/) 20 LTS+
- Rust toolchain (`rustup`)
- From repo root: workspace builds `inertia-api` with `cargo build --release -p inertia-api`

### Linux (desktop)

Install Tauri system libraries ([prerequisites](https://tauri.app/start/prerequisites/)):

```bash
# Fedora
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel gcc
```

### Windows (desktop)

- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (C++ workload)
- WebView2 (usually preinstalled on Windows 10/11)

macOS packaging is deferred.

### Android

- [Android Studio](https://developer.android.com/studio) (SDK + NDK)
- `npm run android:sdk` (platform-tools, Android 35/36, NDK 26.3, `local.properties`)
- Rust target `aarch64-linux-android` + `cargo-ndk`
- JDK **17 or 21** (Fedora default Java 25 breaks Gradle)

## Commands (repo root)

### Desktop

```bash
npm run desktop:dev
npm run desktop:build
npm run desktop:package
```

`desktop:build` sets `NO_STRIP=true` and `ARCH` on Linux so AppImage/linuxdeploy works on Fedora.

Optional bundle filter (used by release CI):

```bash
npm run desktop:build -- --bundles nsis
npm run desktop:build -- --bundles rpm,appimage
```

### Android

```bash
# Cross-compile API, build web, package jniLibs/assets, build debug APK
npm run android:install

# Same prep, then install/run on a connected device
npm run android:run

# Open the Tauri Android project in Android Studio
npm run android:open
```

`android:install` runs:

1. `android:api:build` - `cargo ndk` arm64 → `dist/android-arm64/inertia-api`
2. `web:build` - Svelte static UI
3. `android:package` - copy into `apps/desktop/src-tauri/gen/android/.../{jniLibs,assets}`
4. `android:apk` - `tauri android build --debug --apk --target aarch64`

Layout:

```
apps/desktop/
  splash/                 # brief page before navigate to API
  src-tauri/
    gen/android/          # Tauri Android Studio project (committed)
    binaries/             # desktop sidecar (gitignored)
    resources/web/        # desktop web copy (gitignored)
    src/lib.rs            # desktop sidecar vs mobile health-wait
    tauri.conf.json
    tauri.android.conf.json
```

### Android lifecycle

1. **SplashActivity** starts **InertiaApiService** (foreground notification).
2. Extracts web assets, runs `libinertia_api.so` with `INERTIA_DATA_DIR` / `INERTIA_WEB_DIR`.
3. Waits for `GET /api/health`.
4. **MainActivity** (Tauri) opens; Rust navigates to `http://127.0.0.1:4783/` (or pending invite URL).

Deep links: `inertia://invite/…` and `http://127.0.0.1:4783/invite…`.

## GitHub Releases

Tagging a stable cut (`./scripts/release-tag.sh`) runs [`.github/workflows/release.yml`](../.github/workflows/release.yml):

| Asset | Notes |
|-------|--------|
| `Inertia-<version>-windows-x64-setup.exe` | NSIS installer |
| `Inertia-<version>-linux-x86_64.rpm` | Fedora / RHEL-family |
| `Inertia-<version>-linux-x86_64.AppImage` | Portable Linux |
| `inertia-windows-x64.zip` | Portable zip |
| `Inertia-<version>-android-arm64-debug.apk` | Tauri Android debug APK (sideload) |

Version sync: [`scripts/sync-desktop-version.mjs`](../scripts/sync-desktop-version.mjs) and [`scripts/sync-android-version.mjs`](../scripts/sync-android-version.mjs).

## Smoke checklist

### Desktop (Linux)

- [x] `npm run desktop:dev` opens a native window
- [x] Health + UI at `http://127.0.0.1:4783`; data under OS app data
- [x] Closing the window stops `inertia-api`
- [x] `npm run desktop:build` produces rpm/deb/AppImage (with `NO_STRIP` + `ARCH`)
- [ ] Invite / Messages / Settings relay like the Windows zip flow

### Android

- [x] `npm run android:install` produces an arm64 debug APK
- [ ] Splash → API healthy → UI at `127.0.0.1:4783` (device smoke)
- [ ] Invite deep link / paste accept
- [ ] Messages + relay status

## Security notes

- Cleartext `http://127.0.0.1` only. See [SECURITY-TODO.md](./SECURITY-TODO.md).
- Do not run zip `inertia-api` and the Tauri desktop sidecar on the same port at once.

## Notes

- Optional override for Android Studio: `ANDROID_STUDIO_PATH` (legacy `CAPACITOR_ANDROID_STUDIO_PATH` still accepted).
- Android project path is Tauri's layout: `apps/desktop/src-tauri/gen/android` (same crate as desktop).
