# Windows setup

Fresh Windows installs often block PowerShell scripts, and may not have Git, Node, or Rust yet. This guide gets you from zero to a running Inertia instance.

---

## Fastest path (recommended)

1. **Get the project folder**
   - With Git: clone the repo (see [README](../README.md)).
   - Without Git: download the [latest release ZIP](https://github.com/iagolavor/inertia/releases/latest) (or [development.zip](https://github.com/iagolavor/inertia/archive/refs/heads/development.zip) for bleeding edge), extract it, and open a terminal in the extracted folder.

2. **Install dependencies** (one time) — open **PowerShell** or **cmd** in the project folder:

   ```powershell
   npm run setup:windows -- -InstallDeps
   ```

   This uses [winget](https://learn.microsoft.com/en-us/windows/package-manager/winget/) to install Node.js LTS, Rust, and Git when they are missing, then builds the web UI and release API.

   **No npm yet?** Install Node first (see [Manual install](#manual-install-without-winget)), open a **new** terminal, then run the command above.

3. **Start Inertia**

   ```powershell
   npm run run:windows
   ```

   Two windows open: **inertia-api** (release) and **inertia-web** (static build + preview). Open [http://localhost:4173](http://localhost:4173).

   This uses `api:release` and `web:build` + `web:preview` — not the Vite dev server — so RAM stays low (~25 MB API + small static server).

---

## Double-click (no terminal typing)

If PowerShell blocks scripts, use the `.cmd` wrappers — they always run with `-ExecutionPolicy Bypass`:

| File | What it does |
|------|----------------|
| `scripts\setup-windows-install.cmd` | Install Node/Rust/Git via winget (if missing), then build |
| `scripts\setup-windows.cmd` | Build only (tools must already be installed) |
| `scripts\run-windows.cmd` | Start release API + build + static web preview (low RAM) |
| `scripts\update-windows.cmd` | Download latest release, rebuild (keeps your data) |
| `scripts\update-and-run-windows.cmd` | Update, rebuild, then start Inertia |

From File Explorer: right-click `setup-windows.cmd` → **Run as administrator** only if winget installs fail (usually not required).

---

## “Scripts are disabled on this system”

Windows defaults can block `.ps1` files. You do **not** need to change system policy if you use:

- **`npm run …`** — root `package.json` already passes `-ExecutionPolicy Bypass` for API/stop/setup scripts.
- **`.cmd` files** in `scripts/` — same bypass, no policy change.

To allow scripts for your user account (optional):

```powershell
Set-ExecutionPolicy -Scope CurrentUser RemoteSigned
```

---

## Manual install (without winget)

Install these, then **open a new terminal** so `PATH` updates:

| Tool | Download | Verify |
|------|----------|--------|
| **Node.js 20 LTS** | [nodejs.org](https://nodejs.org/) | `node -v` and `npm -v` |
| **Rust** | [rustup.rs](https://rustup.rs/) | `cargo -v` |
| **Git** (optional) | [git-scm.com](https://git-scm.com/download/win) | `git -v` |

Then from the project folder:

```powershell
npm run setup:windows
npm run run:windows
```

If `node` works in a new Windows Terminal tab but not in Cursor, run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/refresh-path.ps1
```

---

## Daily use (after first setup)

| Command | Description |
|---------|-------------|
| `npm run run:windows` | Release API + `web:build` + static preview (two windows, low RAM) |
| `npm run api:window` | API only in a separate window |
| `npm run web:preview` | Static UI at `:4173` (run `npm run web:build` after UI changes) |
| `npm run api:stop` | Free port 4783 if rebuild says “Access is denied” |

**Low memory:** prefer `api:release` + `web:preview` over `npm run api` + `npm run web` (Vite dev).

---

## Getting updates (no Git required)

When a new version is released, update without opening GitHub or running git commands:

```powershell
npm run update:windows
```

Or double-click **`scripts\update-windows.cmd`**.

What it does:

1. Stops the API if it is running
2. Downloads the **latest GitHub release** (default) or pulls source if you cloned with Git
3. Keeps your **`data/`** folder and **`.env`** untouched
4. Rebuilds the web UI and API
5. Skips download if you are already on that version (use `-Force` to rebuild anyway)

Update and start in one step:

```powershell
npm run update:windows -- -Start
```

Or double-click **`scripts\update-and-run-windows.cmd`**.

| Command | Description |
|---------|-------------|
| `npm run update:windows` | Latest **release** + rebuild |
| `npm run update:windows -- -Channel development` | Bleeding-edge **development** branch |
| `npm run update:windows -- -Start` | Update, rebuild, then start Inertia |
| `npm run update:windows -- -Force` | Rebuild even if version matches |

**Developers** on a feature branch should use `git pull`, not `update:windows` (the script will warn if you are on `feature/*`).

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `'npm' is not recognized` | Install Node; open a **new** terminal |
| `'cargo' is not recognized` | Install Rust via rustup; open a **new** terminal |
| `Access is denied` on `inertia-api.exe` | `npm run api:stop`, then start again |
| Web shows “API offline” | Start API (`npm run api:window` or `run:windows`); click **Retry** in the app |
| First Rust build is slow | Normal — release build can take 5–15 minutes on first run |
| winget not found | Use [Manual install](#manual-install-without-winget) or update Windows App Installer from the Microsoft Store |

---

## Cursor / VS Code

After setup, use the **run** task (release API + web preview) from **Terminal → Run Task…**. See [README](../README.md#vs-code--cursor).
