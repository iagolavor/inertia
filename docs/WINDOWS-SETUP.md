# Windows setup

Download **`inertia-windows-x64.zip`** from the [latest GitHub Release](https://github.com/iagolavor/inertia/releases/latest), extract anywhere, then double-click **`run.cmd`**.

- Opens **http://127.0.0.1:4783** (one app, ~25 MB RAM)
- Your data lives in **`data/`** next to the exe
- **Update:** double-click **`update.cmd`** (keeps `data/` and `.env`)

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| Nothing happens when you double-click | Right-click `run.cmd` → Run, or open cmd in the folder and run `run.cmd` |
| Port already in use | Close any other Inertia window; open Task Manager and end `inertia-api.exe` |
| Update says no zip on release | That release was tagged before CI packaging — download the zip manually from Releases |
| Blank page or errors after update | Run `update.cmd` again, or re-download the zip and copy your `data/` folder over |

---

## Developing Inertia on Windows

Use the normal dev setup in [README](../README.md): install Rust + Node, clone the repo, then:

```powershell
npm run api          # terminal 1
npm run web          # terminal 2 → http://localhost:5173
```

No special Windows scripts required.
