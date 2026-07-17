# Windows setup

Download from **[GitHub Releases](https://github.com/iagolavor/inertia/releases/latest)**.

| Asset | When |
|-------|------|
| **`Inertia-<version>-windows-x64-setup.exe`** | Recommended: desktop shell (Tauri). Double-click to install. |
| **`inertia-windows-x64.zip`** | Portable: extract anywhere, run `run.cmd` (browser UI). |

## Desktop installer (recommended)

1. Download **`Inertia-*-windows-x64-setup.exe`**
2. Run the installer (current-user install; no admin required)
3. Launch **Inertia** from the Start menu

Data is stored in the OS app data directory (not next to the exe). See [TAURI.md](./TAURI.md).

Windows may show a SmartScreen / unknown publisher warning until releases are code-signed. Choose **More info → Run anyway** if you trust the GitHub release asset.

## Portable zip

Download **`inertia-windows-x64.zip`**, extract anywhere, then double-click **`run.cmd`**.

| Step | Action |
|------|--------|
| **Run** | Double-click `run.cmd` → [http://127.0.0.1:4783](http://127.0.0.1:4783) |
| **Data** | Stored in `data/` next to the exe (back it up to keep your profile and friends) |
| **Update** | Double-click `update.cmd`. Downloads the latest release zip and keeps `data/`. |

Nothing else to install: no Rust, Node, Git, or winget.

### Troubleshooting (zip)

| Problem | Fix |
|---------|-----|
| Nothing happens when you double-click | Right-click `run.cmd` → **Run**, or open cmd in the folder and type `run.cmd` |
| Port already in use | Close other Inertia windows; Task Manager → end `inertia-api.exe` |
| Update says no zip on release | Download the zip manually from [Releases](https://github.com/iagolavor/inertia/releases) |
| Blank page after update | Run `update.cmd` again, or re-extract a fresh zip and copy your `data/` folder over |

---

## Developing on Windows

Install [Rust](https://rustup.rs/) and [Node.js](https://nodejs.org/), clone the repo, then follow **Get started → Developers** in the [README](../README.md#get-started).
