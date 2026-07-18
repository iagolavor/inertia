# Release process

Stable cuts promote **development** → **master** via a labeled PR, then a git tag and GitHub Release.

See also [GIT-WORKFLOW.md](./GIT-WORKFLOW.md) for branch rules.

## 1. Prepare

- Merge all intended work into `development` first (feature PRs).
- Smoke-test locally (`npm run api:release`, `npm run web:build`, `npm run web:preview`, or your usual checklist).
- Ensure `development` is ahead of `master`:

```bash
git fetch origin
git rev-list --count origin/master..origin/development
```

## 2. Open the release PR

Use the helper script (requires [GitHub CLI](https://cli.github.com/)):

**Bash (Linux / macOS / Git Bash)**

```bash
./scripts/release-pr.sh 0.2.0
```

**PowerShell (Windows)**

```powershell
powershell -ExecutionPolicy Bypass -File scripts/release-pr.ps1 -Version 0.2.0
```

This opens a PR:

- **Base:** `master` ← **Head:** `development`
- **Title:** `Release v0.2.0`
- **Label:** `release`
- **Body:** summary, commits since last tag, checklist, merged PR list

Review the generated PR body and edit if anything is missing.

### Release theme (optional)

For cuts with a clear narrative, add a **theme** line to the PR title and a short summary block in the body. Example for v0.13.0:

- **Title:** `Release v0.13.0 - Event-driven live sync`
- **Summary:** SSE replaces interval polling for content and P2P aliveness; see [LIVE-SYNC.md](./LIVE-SYNC.md).

Use **User-visible** and **Developer** subsections in the PR body when the theme is architectural (not just a bugfix batch).

### Manual alternative

**Bash**

```bash
gh pr create --base master --head development \
  --title "Release v0.2.0" \
  --label release \
  --body-file path/to/body.md
```

**PowerShell**

```powershell
gh pr create --base master --head development `
  --title "Release v0.2.0" `
  --label release `
  --body-file path/to/body.md
```

## 3. Merge

- Use a **merge commit** (not squash) to keep branch history aligned with git-flow.
- Do not delete `development` after merge. **`master` stays protected** (release PRs only); `development` is open for direct push while the project is solo.

## 4. Tag and GitHub Release

After the release PR is merged:

**Bash**

```bash
git checkout master
git pull origin master
./scripts/release-tag.sh 0.2.0
```

**PowerShell**

```powershell
git checkout master
git pull origin master
powershell -ExecutionPolicy Bypass -File scripts/release-tag.ps1 -Version 0.2.0
```

This will:

1. Create an annotated tag `v0.2.0` on `master`
2. Push the tag to GitHub
3. Run **GitHub Actions** (`.github/workflows/release.yml`) to build release assets and publish a [GitHub Release](https://github.com/iagolavor/inertia/releases) with auto-generated notes

Track the workflow: [Actions](https://github.com/iagolavor/inertia/actions). Assets appear on the release page when all jobs finish.

### Release assets

| Asset | Platform |
|-------|----------|
| `Inertia-<version>-windows-x64-setup.exe` | Windows desktop installer (NSIS / Tauri) |
| `inertia-windows-x64.zip` | Windows portable zip (`run.cmd`) |
| `Inertia-<version>-linux-x86_64.rpm` | Fedora / RHEL-family |
| `Inertia-<version>-linux-x86_64.AppImage` | Portable Linux |
| `Inertia-<version>-android-arm64.apk` | Android arm64 (release-signed when CI secrets are set) |
| `Inertia-<version>-android-arm64-debug.apk` | Android arm64 sideload (debug-signed; used until Play signing secrets exist) |

CI syncs desktop and Android package versions from the git tag. Desktop: `npm run desktop:build` with `--bundles nsis` (Windows) or `--bundles rpm,appimage` (Linux). Android: `npm run android:install` then Gradle `assembleRelease` (if keystore secrets) or `assembleDebug`. See [TAURI.md](./TAURI.md) and [CAPACITOR.md](./CAPACITOR.md).

### Local packaging (maintainers)

```powershell
# Windows zip
npm run package:windows
# → dist/inertia-windows-x64.zip

# Desktop installers (same as CI, all bundles)
npm run desktop:build
# → apps/desktop/src-tauri/target/release/bundle/

# Android Stage B APK (debug)
npm run android:install
cd apps/web/android && ./gradlew assembleDebug
# → app/build/outputs/apk/debug/app-debug.apk
```

### Android release signing (optional CI secrets)

When these repository secrets are set, the release workflow publishes a release-signed APK instead of the debug one:

| Secret | Purpose |
|--------|---------|
| `ANDROID_KEYSTORE_BASE64` | Base64-encoded `.jks` / `.keystore` |
| `ANDROID_KEYSTORE_PASSWORD` | Keystore password |
| `ANDROID_KEY_ALIAS` | Key alias |
| `ANDROID_KEY_PASSWORD` | Key password |

Until then, GitHub Releases ship `Inertia-*-android-arm64-debug.apk` for sideload (same trust model as local `android:run`).

## Versioning

Early project: use **semver** loosely (`v0.1.0`, `v0.2.0`). Bump:

- **Patch** - bugfixes only
- **Minor** - features, refactors, docs batches worth a stable cut
- **Major** - breaking API/storage/P2P protocol changes (rare pre-1.0)

## What visitors see

GitHub's default branch should be **development** (active README and docs). **master** + tags reflect the latest stable release.

End users should download installers from Releases - not clone the repo. See [WINDOWS-SETUP.md](./WINDOWS-SETUP.md), [LINUX-SETUP.md](./LINUX-SETUP.md), and [CAPACITOR.md](./CAPACITOR.md) for Android.
