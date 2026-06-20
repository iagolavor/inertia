# Windows setup

Download **[inertia-windows-x64.zip](https://github.com/iagolavor/inertia/releases/latest)** from GitHub Releases, extract anywhere, then double-click **`run.cmd`**.

| Step | Action |
|------|--------|
| **Run** | Double-click `run.cmd` → [http://127.0.0.1:4783](http://127.0.0.1:4783) |
| **Data** | Stored in `data/` next to the exe (back it up to keep your profile and friends) |
| **Update** | Double-click `update.cmd` — downloads the latest release zip, keeps `data/` |

Nothing else to install — no Rust, Node, Git, or winget.

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| Nothing happens when you double-click | Right-click `run.cmd` → **Run**, or open cmd in the folder and type `run.cmd` |
| Port already in use | Close other Inertia windows; Task Manager → end `inertia-api.exe` |
| Update says no zip on release | Download the zip manually from [Releases](https://github.com/iagolavor/inertia/releases) |
| Blank page after update | Run `update.cmd` again, or re-extract a fresh zip and copy your `data/` folder over |

---

## Developing on Windows

Install [Rust](https://rustup.rs/) and [Node.js](https://nodejs.org/), clone the repo, then follow **Get started → Developers** in the [README](../README.md#get-started).
