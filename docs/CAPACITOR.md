# Capacitor - Android shell

Default (and only supported) path: **API + UI on the phone** (same idea as the Windows zip: one local process serves UI and `/api`).

| | **Android install** |
|---|---------------------|
| UI | Served by on-device `inertia-api` |
| API | On phone (`127.0.0.1:4783`) |
| Layout | `inertia-api` + `web/` + `data/` in app storage |

## Prerequisites

- [Node.js](https://nodejs.org/) 20 LTS+
- [Android Studio](https://developer.android.com/studio) (SDK + NDK)
- Rust toolchain (arm64 cross-compile)

### Android Studio + SDK (Windows)

```powershell
winget install Google.AndroidStudio --accept-package-agreements --accept-source-agreements
npm run android:sdk
```

### Android Studio + SDK (Linux)

Install [Android Studio](https://developer.android.com/studio) (or OpenJDK 17+), then from repo root:

```bash
npm run android:sdk
```

Default SDK path: `~/Android/Sdk` (override with `ANDROID_HOME`).

**JDK:** Gradle needs **Java 17 or 21**. Fedora's default Java 25 will fail (`Unsupported class file major version 69`). `~/.bashrc.d/android.sh` prefers OpenJDK 21 or Android Studio's bundled JBR.

`npm run android:open` auto-detects Android Studio (tarball paths and Flatpak). Override only if needed: `CAPACITOR_ANDROID_STUDIO_PATH`.

`android:sdk` installs **platform-tools**, **Android 35**, **build-tools 35.0.0**, **NDK 26.3**, writes `local.properties`, and sets `ANDROID_HOME` / `JAVA_HOME`.

On-device builds require **minSdk 24** (Android 7.0) because libp2p uses `getifaddrs`, which Bionic only exposes from API 24. `android:api:build` passes `-P 24` to `cargo-ndk`.

Restart terminals after the script.

SDK path: `%LOCALAPPDATA%\Android\Sdk` (Windows) or `~/Android/Sdk` (Linux).

## Build and install (API on device)

One-shot from repo root:

```powershell
npm run android:install
npm run android:run
```

`android:install` runs:

1. `android:api:build` - cross-compile `inertia-api` for arm64 → `dist/android-arm64/inertia-api`
2. `web:build` - Svelte static UI
3. `android:package` - copy API binary to `jniLibs/` (as `.so`) + `web/` into assets
4. `android:sync` - Capacitor sync into the Gradle project

Then `android:run` builds the APK and installs it on a connected device/emulator.

The Rust binary is shipped as `libinertia_api.so` under `jniLibs/arm64-v8a/` so Android installs it with execute permission (required on API 29+). Web static files stay in assets and extract to app storage on first launch.

On launch:

1. **SplashActivity** starts **InertiaApiService** (foreground notification)
2. Extracts assets to app storage, runs `inertia-api` with `INERTIA_DATA_DIR` / `INERTIA_WEB_DIR`
3. Waits for `GET /api/health`
4. **MainActivity** WebView loads `http://127.0.0.1:4783/` (same origin → `/api` in the UI)

No PC API required.

### Scripts

| Command | Purpose |
|---------|---------|
| `npm run android:api:build` | `cargo ndk` → `dist/android-arm64/inertia-api` |
| `npm run android:package` | Copy API + web into APK assets |
| `npm run android:install` | Full package pipeline + Capacitor sync |
| `npm run android:run` | Build APK and install on device |

Packaged assets are gitignored; run `android:install` before shipping or sideloading an APK.

### GitHub Releases

Tagging a stable cut publishes `Inertia-<version>-android-arm64-debug.apk` (debug-signed sideload; same as local `android:run`). Play / release-keystore signing can be added later.

## Cleartext HTTP

The app uses `http://127.0.0.1:4783` on-device. `usesCleartextTraffic="true"` is set in the manifest. Tighten before wide public release (see [SECURITY-TODO.md](./SECURITY-TODO.md)).

## Status (v0.10+)

**On-device Android is the supported phone path.** A physical arm64 phone can run Inertia from the APK: local API, local SQLite, P2P via relay, and cross-device invite accept (paste payload - do not tap SMS links).

USB debugging is only for developer install (`android:run` / Android Studio), not for routing API traffic to a PC.

### Verified working

- [x] Cross-compile `inertia-api` for **arm64-v8a** only (`scripts/build-android-api.ps1` / `.sh`)
- [x] Package API + web into APK (`scripts/package-android.ps1` / `.sh`); assets gitignored until built
- [x] Foreground service + splash health gate + WebView on `http://127.0.0.1:4783/`
- [x] Separate phone profile / DB from desktop
- [x] P2P relay connect + reservation; invite create (PC) → paste accept (phone)
- [x] `GET /invite/readiness` gates Generate until inbound circuit slot is ready
- [x] In-app invite handling (`inertia://`, `InertiaWebViewClient` - no Chrome handoff)
- [x] Accept waits for relay + inviter libp2p session before redemption
- [x] Header **P2pStatus** tap panel for touch / mobile status details

### Known rough edges

- [ ] **Invite preview** shows red offline dot on inviter avatar (`ProfileHeader` default) - misleading
- [ ] **arm64 only** - no x86 emulator ABI in `android:api:build`
- [ ] **Relay OK** is not enough for invite create: inviter needs inbound **reservation** (Friends UI uses `/invite/readiness`); accepter needs relay session + inviter online

### Resume checklist (next session)

1. `git checkout development && git pull`
2. Rebuild if UI or Rust changed: `npm run android:install` (not bare `npx cap sync` from repo root - use `npm run android:sync`)
3. Install from Android Studio or `npm run android:run`
4. PC inviter: `npm run api:release`, relay connected + reservation active (Generate enabled), **Copy for phone** (payload only)
5. Phone: **⋯ → Accept invite** → paste → Preview → Accept

Pick up polish from the **Resume next** list in [AGENTS.md](../AGENTS.md) (invite avatar dot).

## Not yet implemented

- Play Store / release-keystore signing in CI (debug APK already ships on `v*` tags; see [RELEASE.md](./RELEASE.md))
- API auth on localhost
- Private keys in Android Keystore
- iOS shell
