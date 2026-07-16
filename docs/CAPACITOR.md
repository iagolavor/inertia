# Capacitor - Android shell

Default path: **API + UI on the phone** (same idea as the Windows zip: one local process serves UI and `/api`).

| | **Android install** (default) | **Dev: API on PC** (optional) |
|---|--------------------------------|-------------------------------|
| UI | Served by on-device `inertia-api` | Bundled Capacitor `webDir` |
| API | On phone (`127.0.0.1:4783`) | Dev PC (`adb reverse` / LAN) |
| Layout | `inertia-api` + `web/` + `data/` in app storage | Capacitor assets only |

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

No PC API, no `adb reverse`.

### Scripts

| Command | Purpose |
|---------|---------|
| `npm run android:api:build` | `cargo ndk` → `dist/android-arm64/inertia-api` |
| `npm run android:package` | Copy API + web into APK assets |
| `npm run android:install` | Full package pipeline + Capacitor sync |
| `npm run android:run` | Build APK and install on device |

`android:stage-b` remains as a deprecated alias for `android:install`.

Packaged assets are gitignored; run `android:install` before shipping or sideloading an APK.

## Dev loop (UI on phone, API on PC)

**Terminal 1 - API on PC:**

```powershell
npm run api:release
```

**Terminal 2 - sync bundled UI (no `android:package`):**

```powershell
npm run android:sync
npm run android:run
```

**Terminal 3 - port forward (emulator or USB):**

```powershell
npm run android:reverse
```

If APK assets do **not** contain the bundled API, the app expects the PC API (bundled UI only).

### Emulator

```powershell
npm run api:release
adb reverse tcp:4783 tcp:4783   # after each cold boot
```

Alternative: `$env:VITE_INERTIA_API_BASE = "http://10.0.2.2:4783/api"` then `npm run android:sync`.

### Wi-Fi (LAN)

```powershell
$env:VITE_INERTIA_API_BASE = "http://192.168.1.42:4783/api"
npm run web:build
npm run android:sync
```

API must listen on `0.0.0.0:4783`.

## Cleartext HTTP

Default install uses `http://127.0.0.1:4783` on-device. The PC-API path uses the same via `adb reverse`. `usesCleartextTraffic="true"` is set in the manifest. Tighten before wide public release (see [SECURITY-TODO.md](./SECURITY-TODO.md)).

## Status (v0.10+)

**On-device Android is the supported phone path.** A physical arm64 phone can run Inertia from the APK: local API, local SQLite, P2P via relay, and cross-device invite accept (paste payload - do not tap SMS links).

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
- [ ] **PC-API path** (`adb reverse`) - supported but not re-smoked recently
- [ ] **arm64 only** - no x86 emulator ABI in `android:api:build`
- [ ] **Relay OK** is not enough for invite create: inviter needs inbound **reservation** (Friends UI uses `/invite/readiness`); accepter needs relay session + inviter online

### Resume checklist (next session)

1. `git checkout development && git pull`
2. Rebuild if UI or Rust changed: `npm run android:install` (not bare `npx cap sync` from repo root - use `npm run android:sync`)
3. Install from Android Studio or `npm run android:run`
4. PC inviter: `npm run api:release`, relay connected + reservation active (Generate enabled), **Copy for phone** (payload only)
5. Phone: **⋯ → Accept invite** → paste → Preview → Accept

Pick up polish from the **Resume next** list in [AGENTS.md](../AGENTS.md) (invite avatar dot, PC-API smoke).

## Not yet implemented

- Play Store / release signing in CI
- API auth on localhost
- Private keys in Android Keystore
- iOS shell
