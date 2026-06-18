# Release process

Stable cuts promote **development** → **master** via a labeled PR, then a git tag and GitHub Release.

See also [GIT-WORKFLOW.md](./GIT-WORKFLOW.md) for branch rules.

## 1. Prepare

- Merge all intended work into `development` first (feature PRs).
- Smoke-test locally (`npm run api`, `npm run web`, or your usual checklist).
- Ensure `development` is ahead of `master`:

```bash
git fetch origin
git rev-list --count origin/master..origin/development
```

## 2. Open the release PR

Use the helper script (requires [GitHub CLI](https://cli.github.com/)):

```bash
./scripts/release-pr.sh 0.2.0
```

This opens a PR:

- **Base:** `master` ← **Head:** `development`
- **Title:** `Release v0.2.0`
- **Label:** `release`
- **Body:** summary, commits since last tag, checklist, merged PR list

Review the generated PR body and edit if anything is missing.

### Manual alternative

```bash
gh pr create --base master --head development \
  --title "Release v0.2.0" \
  --label release \
  --body-file path/to/body.md
```

## 3. Merge

- Use a **merge commit** (not squash) to keep branch history aligned with git-flow.
- Do not delete `development` after merge — both branches stay protected.

## 4. Tag and GitHub Release

After the release PR is merged:

```bash
git checkout master
git pull origin master
./scripts/release-tag.sh 0.2.0
```

This will:

1. Create an annotated tag `v0.2.0` on `master`
2. Push the tag to GitHub
3. Create a [GitHub Release](https://github.com/iagolavor/inertia/releases) with auto-generated notes

## Versioning

Early project: use **semver** loosely (`v0.1.0`, `v0.2.0`). Bump:

- **Patch** — bugfixes only
- **Minor** — features, refactors, docs batches worth a stable cut
- **Major** — breaking API/storage/P2P protocol changes (rare pre-1.0)

## What visitors see

GitHub’s default branch should be **development** (active README and docs). **master** + tags reflect the latest stable release.
