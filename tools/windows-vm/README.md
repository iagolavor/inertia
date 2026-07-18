# Windows 11 VM (Docker + KVM)

Run a Windows 11 VM on Fedora to smoke-test the Tauri desktop shell.

## Prerequisites

- Fedora with working Docker (`moby-engine`, `runc`, `docker compose`)
- KVM: `/dev/kvm` present
- ~8 GB free RAM for the VM, ~64 GB+ free disk for the image

## Start

```bash
cd tools/windows-vm
mkdir -p data
docker compose up -d
```

On Fedora, bind mounts use the SELinux `:Z` flag. If you see "Storage folder is not writeable", ensure `./data` exists and is owned by your user.

Open **http://127.0.0.1:8006** (web viewer). First boot downloads Windows and installs unattended.

RDP: `localhost:3389` - user `Docker` / password `admin`.

Host readiness check:

```bash
./tools/windows-vm/host-smoke-check.sh
```

## Shared folder (repo drop-in)

[`docker-compose.yml`](./docker-compose.yml) binds the **Inertia repo root** to `/shared`:

```yaml
- ../..:/shared:rw,Z
```

dockur serves that path over **Samba** as:

| In Windows | Meaning |
|------------|---------|
| Desktop → **Shared** | Shortcut to the share |
| `\\host.lan\Data` | UNC path (same files) |

You should see `package.json`, `apps/`, `crates/`, `tools/` at the Shared root.

**Always copy onto NTFS** (`C:\Users\Docker\src\inertia`) before building. Do not run cargo/npm on the Samba share.

After changing compose volumes:

```bash
cd tools/windows-vm
docker compose up -d --force-recreate
```

(`./data` keeps the VM disk.)

## Stop

```bash
cd tools/windows-vm
docker compose down
```

VM disk stays in `tools/windows-vm/data/` (gitignored).

## One-time toolchain (inside the VM)

As user `Docker` / password `admin`, open PowerShell on Shared (or `\\host.lan\Data`):

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\windows-vm\guest-setup.ps1
```

[`guest-setup.ps1`](./guest-setup.ps1) uses winget to install Node LTS, Rust (MSVC), and VS 2022 Build Tools (C++).

Manual alternatives: [Node 20](https://nodejs.org/), [rustup](https://rustup.rs/), [VS Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (Desktop development with C++). WebView2 is usually already on Windows 11.

Close and reopen PowerShell after setup, then verify `node`, `npm`, `rustc`, `cargo`.

## Build Tauri standalone (repeatable)

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\windows-vm\guest-build.ps1
```

[`guest-build.ps1`](./guest-build.ps1) will:

1. Resolve Shared (`\\host.lan\Data` or Desktop\Shared)
2. `robocopy` into `C:\Users\Docker\src\inertia` (excludes `node_modules`, `target`, `data`, `*.img` / the VM disk, etc.)
3. `npm install` in `apps/web` + `apps/desktop`, then `npm run desktop:build`

If a previous run failed copying `tools\windows-vm\data\data.img`, free space first:

```powershell
Remove-Item -Recurse -Force C:\Users\Docker\src\inertia -ErrorAction SilentlyContinue
```

Then re-run `guest-build.ps1` (the script no longer copies that image).

**Output:** NSIS/MSI under

`C:\Users\Docker\src\inertia\apps\desktop\src-tauri\target\release\bundle\`

Copy the installer back into Shared (e.g. `Shared\dist\`) to pick it up on Fedora under the repo tree.

See also [docs/TAURI.md](../../docs/TAURI.md).

## Notes

- Do not expect `/data/inertia` in Explorer (old container-only path).
- Do not reuse Linux `target/` or `node_modules` from the Fedora host.
- dockur may warn about **btrfs** under `/storage`; if Windows Setup fails, move `./data` to an ext4 volume.
