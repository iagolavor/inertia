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
3. Run **GitHub Actions** (`.github/workflows/release.yml`) to build **`inertia-windows-x64.zip`** and publish a [GitHub Release](https://github.com/iagolavor/inertia/releases) with auto-generated notes

Track the workflow: [Actions](https://github.com/iagolavor/inertia/actions). The zip appears on the release page in a few minutes.

### Windows zip (maintainers, local test)

After `cargo build --release -p inertia-api` and `npm run web:build`:

```powershell
npm run package:windows
# → dist/inertia-windows-x64.zip
```

## Versioning

Early project: use **semver** loosely (`v0.1.0`, `v0.2.0`). Bump:

- **Patch** — bugfixes only
- **Minor** — features, refactors, docs batches worth a stable cut
- **Major** — breaking API/storage/P2P protocol changes (rare pre-1.0)

## What visitors see

GitHub’s default branch should be **development** (active README and docs). **master** + tags reflect the latest stable release.

**Windows users** should download **`inertia-windows-x64.zip`** from Releases — not clone the repo. See [WINDOWS-SETUP.md](./WINDOWS-SETUP.md).
