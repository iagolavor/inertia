# Capacitor — Android shell

Two modes:

| | **Stage A** (dev) | **Stage B** (self-contained) |
|---|-------------------|------------------------------|
| UI | Bundled in APK | Served by local `inertia-api` |
| API | Dev PC (`adb reverse` / LAN) | On phone (`127.0.0.1:4783`) |
| Layout | Capacitor `webDir` | Same as Windows zip: `inertia-api` + `web/` + `data/` |

Stage B mirrors [scripts/windows/run.ps1](../scripts/windows/run.ps1): one process serves UI and `/api`.

## Prerequisites

- [Node.js](https://nodejs.org/) 20 LTS+
- [Android Studio](https://developer.android.com/studio) (SDK + NDK for Stage B)
- Rust toolchain (Stage B cross-compile)

### Android Studio + SDK (Windows)

```powershell
winget install Google.AndroidStudio --accept-package-agreements --accept-source-agreements
npm run android:sdk
```

`android:sdk` installs **platform-tools**, **Android 35**, **build-tools 35.0.0**, **NDK 26.3**, writes `local.properties`, and sets `ANDROID_HOME` / `JAVA_HOME`.

Stage B requires **minSdk 24** (Android 7.0) because libp2p uses `getifaddrs`, which Bionic only exposes from API 24. `android:api:build` passes `-P 24` to `cargo-ndk`.

Restart terminals after the script.

SDK path: `%LOCALAPPDATA%\Android\Sdk`

## Stage B — build and run (API on device)

One-shot from repo root:

```powershell
npm run android:stage-b
npm run android:run
```

This runs:

1. `android:api:build` — cross-compile `inertia-api` for arm64 → `dist/android-arm64/inertia-api`
2. `web:build` — Svelte static UI
3. `android:package` — copy API binary to `jniLibs/` (as `.so`) + `web/` into assets
4. `android:sync` — Capacitor sync into the Gradle project

The Rust binary is shipped as `libinertia_api.so` under `jniLibs/arm64-v8a/` so Android installs it with execute permission (required on API 29+). Web static files stay in assets and extract to app storage on first launch.

On launch:

1. **SplashActivity** starts **InertiaApiService** (foreground notification)
2. Extracts assets to app storage, runs `inertia-api` with `INERTIA_DATA_DIR` / `INERTIA_WEB_DIR`
3. Waits for `GET /api/health`
4. **MainActivity** WebView loads `http://127.0.0.1:4783/` (same origin → `/api` in the UI)

No PC API, no `adb reverse`.

### Stage B scripts

| Command | Purpose |
|---------|---------|
| `npm run android:api:build` | `cargo ndk` → `dist/android-arm64/inertia-api` |
| `npm run android:package` | Copy API + web into APK assets |
| `npm run android:stage-b` | Full Stage B pipeline + sync |

Packaged assets are gitignored; build them before installing a Stage B APK.

## Stage A — dev loop (API on PC)

**Terminal 1 — API on PC:**

```powershell
npm run api:release
```

**Terminal 2 — sync bundled UI (no `android:package`):**

```powershell
npm run android:sync
npm run android:run
```

**Terminal 3 — port forward (emulator or USB):**

```powershell
npm run android:reverse
```

If APK assets do **not** contain `inertia/inertia-api`, the app stays in Stage A (bundled UI, API on PC).

### Emulator

```powershell
npm run api:release
adb reverse tcp:4783 tcp:4783   # after each cold boot
```

Alternative: `$env:VITE_INERTIA_API_BASE = "http://10.0.2.2:4783/api"` then `npm run android:sync`.

### Wi‑Fi (LAN)

```powershell
$env:VITE_INERTIA_API_BASE = "http://192.168.1.42:4783/api"
npm run web:build
npm run android:sync
```

API must listen on `0.0.0.0:4783`.

## Cleartext HTTP

Both stages use `http://127.0.0.1:4783` on-device (Stage B) or via reverse (Stage A). `usesCleartextTraffic="true"` is set in the manifest. Tighten before wide public release (see [SECURITY-TODO.md](./SECURITY-TODO.md)).

## Not yet implemented

- Play Store / release signing in CI
- API auth on localhost
- Private keys in Android Keystore
- iOS shell
