# Linux setup

Download a desktop installer from **[GitHub Releases](https://github.com/iagolavor/inertia/releases/latest)**. Prefer the RPM on Fedora; use the AppImage on other distros or for a portable run.

| Asset | When |
|-------|------|
| **`Inertia-<version>-linux-x86_64.rpm`** | Fedora / RHEL-family |
| **`Inertia-<version>-linux-x86_64.AppImage`** | Portable binary (any modern x86_64 Linux with WebKitGTK 4.1) |

## RPM (Fedora)

```bash
sudo dnf install ./Inertia-*-linux-x86_64.rpm
```

Then open **Inertia** from the app menu. Data lives under the OS app data dir (typically `~/.local/share/social.inertia.app`).

## AppImage

```bash
chmod +x ./Inertia-*-linux-x86_64.AppImage
./Inertia-*-linux-x86_64.AppImage
```

## Notes

- The desktop shell starts local `inertia-api` and opens the UI in one window (same as [TAURI.md](./TAURI.md)).
- Installers are unsigned for now; your desktop may warn on first open.
- Building from source: see [TAURI.md](./TAURI.md).
