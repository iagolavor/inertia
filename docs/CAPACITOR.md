# Capacitor — Android shell (Stage A)

Wraps the SvelteKit static build in a native WebView. **Stage A** keeps `inertia-api` on your dev PC; the phone UI talks to it over HTTP. **Stage B** (API on device) is a separate epic.

## Prerequisites

- [Node.js](https://nodejs.org/) 20 LTS+
- [Android Studio](https://developer.android.com/studio) (SDK + emulator or USB device)
- Rust API running on your PC (`npm run api:release`)

### Android Studio + SDK (Windows)

```powershell
winget install Google.AndroidStudio --accept-package-agreements --accept-source-agreements
npm run android:sdk
```

`android:sdk` runs [scripts/setup-android-sdk.ps1](../scripts/setup-android-sdk.ps1): downloads SDK command-line tools, installs **platform-tools**, **Android 35**, **build-tools 35.0.0**, accepts licenses, writes `apps/web/android/local.properties`, and sets user env vars (`ANDROID_HOME`, `JAVA_HOME`, `CAPACITOR_ANDROID_STUDIO_PATH`).

Restart terminals (and Android Studio) after the script. If `npm run android:open` still cannot find Studio:

```powershell
$env:CAPACITOR_ANDROID_STUDIO_PATH = "C:\Program Files\Android\Android Studio\bin\studio64.exe"
```

SDK path: `%LOCALAPPDATA%\Android\Sdk`

## One-time setup

```bash
cd apps/web
npm install
npm run build
npx cap add android   # already done in repo after first merge
```

## Daily loop

**Terminal 1 — API on PC** (bind LAN if not using adb reverse):

```powershell
$env:INERTIA_API_ADDR = "0.0.0.0:4783"
npm run api:release
```

**Terminal 2 — sync web build into Android project:**

```bash
npm run android:sync
```

**Terminal 3 — run on device/emulator:**

```bash
npm run android:open    # Android Studio → Run
# or
npm run android:run     # CLI if SDK tools on PATH
```

## Pointing the app at your PC

Default native base URL: `http://127.0.0.1:4783/api` (your PC’s API, not the phone itself).

### Emulator (Android Studio AVD)

The app cannot reach your PC until you forward the port. **Before tapping Retry in the app:**

```powershell
# API running on PC (default 127.0.0.1:4783 is fine)
npm run api:release

# In another terminal — required after each emulator cold boot
adb reverse tcp:4783 tcp:4783
```

Then **Retry** in the app. The banner’s “Start API” buttons only copy desktop commands; they do not start anything on the emulator.

Alternative: build with `$env:VITE_INERTIA_API_BASE = "http://10.0.2.2:4783/api"` then `npm run android:sync` (no `adb reverse` needed).

**Note:** The Capacitor config uses `androidScheme: 'http'` so the WebView can call the plain-HTTP API. With `https` localhost, Android blocks `http://127.0.0.1` requests even when `adb reverse` is set.

### USB + adb reverse (physical device)

From repo root (works without `adb` on PATH):

```powershell
npm run android:reverse
# or with a specific device serial:
npm run android:reverse -- RQCX302579V
```

Or call adb directly if `platform-tools` is on PATH:

```bash
adb reverse tcp:4783 tcp:4783
```

Default SDK location: `%LOCALAPPDATA%\Android\Sdk\platform-tools\adb.exe`

Rebuild/sync only when UI changes; **reverse is cleared when USB disconnects** — re-run after replugging the cable.

### Wi‑Fi (LAN)

Build with your PC’s LAN IP:

```powershell
cd apps/web
$env:VITE_INERTIA_API_BASE = "http://192.168.1.42:4783/api"
npm run build
npm run cap:sync
```

API must listen on `0.0.0.0:4783` and Windows firewall must allow inbound 4783.

## Scripts

| Command | Where | Purpose |
|---------|-------|---------|
| `npm run android:sync` | repo root | `web:build` + `cap sync` |
| `npm run android:open` | repo root | Open Android Studio |
| `npm run cap:sync` | `apps/web` | Build + sync only |

## Cleartext HTTP

Stage A uses plain HTTP to the dev PC. `AndroidManifest.xml` sets `usesCleartextTraffic="true"` for local development. Stage B should use HTTPS or a localhost sidecar.

## What’s not in Stage A

- `inertia-api` bundled on the phone
- Push notifications, camera plugins
- Play Store release signing (use debug keystore for now)
