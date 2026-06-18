# Inertia

Ephemeral, local-first P2P social network. No accounts database, no ads, no tracking. See [docs/VISION.md](docs/VISION.md).

## Structure

```
inertia/
  crates/
    inertia-core/   # Rust: identity, invites, P2P, storage, expiry
    inertia-api/    # Local HTTP bridge (runs on your device)
  apps/
    web/             # SvelteKit frontend (PWA)
  docs/
    VISION.md
```

## Prerequisites

- [Rust](https://rustup.rs/) (1.75+) — install with `winget install Rustlang.Rustup`, then open a **new** terminal
- [Node.js](https://nodejs.org/) (20+ LTS)

### Windows: Rust / Cargo on PATH

After installing Rust, `cargo` and `rustc` live here:

```
C:\Users\iago-\.cargo\bin\cargo.exe
C:\Users\iago-\.cargo\bin\rustc.exe
```

Rustup usually adds that folder to your user PATH automatically. In a **new** PowerShell window:

```powershell
cargo --version
rustc --version
```

Run the API from the repo root:

```powershell
cd C:\Users\iago-\Workspace\inertia
cargo run -p inertia-api
```

First build downloads dependencies and can take several minutes.

### Windows: Node on PATH

Node is installed at `C:\Program Files\nodejs\` and is on your Windows **user** and **system** PATH.

In a **new** PowerShell window (outside Cursor), these should work immediately:

```powershell
node --version
npm --version
```

**Cursor / VS Code terminals** load environment variables when the editor starts. If `node` is still not found:

1. **Best fix:** fully quit Cursor and reopen the project (not just a new terminal tab).
2. **This repo** prepends Node in [`.vscode/settings.json`](.vscode/settings.json) — open a **new** integrated terminal after that file exists.
3. **Current tab only** — paste this one-liner (no script file needed):

```powershell
$env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [Environment]::GetEnvironmentVariable("Path", "User")
```

Or from repo root, run [`scripts/refresh-path.ps1`](scripts/refresh-path.ps1). If PowerShell says *"running scripts is disabled"*, allow local scripts once for your user account (normal for dev machines):

```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

## Quick start

### 1. Start the local API

```bash
cargo run -p inertia-api
```

Listens on `http://127.0.0.1:4783`. Data stays in `./data` on your machine.

### 2. Install frontend dependencies (once)

From the repo root on Windows (if `npm` is not on PATH, use the full path shown above):

```powershell
cd apps\web
npm install
```

This installs **Vite**, **SvelteKit**, **Svelte 5**, TypeScript, QR code support, and the rest of `package.json` into `apps\web\node_modules`.

### 3. Start the web UI

```powershell
cd apps\web
npm run dev
```

Open `http://localhost:5173`.

### 4. Connect with a friend

1. Both create a local identity (display name only).
2. One person taps **Generate invite link** on the Friends page.
3. Share the link or QR via SMS, iMessage, or in person — **you must stay online** while they accept.
4. Friend confirms the safety code and taps **Accept** (link expires in **15 minutes**, **single-use**).
5. Message each other when both devices are online (strict P2P).

No central servers. No phone number registry. No global user IDs.

## Development

```bash
cargo test -p inertia-core
cargo build --workspace
```

## License

MIT
