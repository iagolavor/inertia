# Git workflow

Inertia uses a simple **git-flow** variant: one integration branch, short-lived feature branches, `master` stays releasable.

## Branches

| Branch | Purpose |
|--------|---------|
| `master` | Stable, releasable history. Tagged releases (when we tag). |
| `development` | Integration branch — merge features here first. |
| `feature/*` | New work. Branch from `development`, merge back to `development`. |
| `fix/*` | Bug fixes. Branch from `development` (or `master` for hotfixes). |
| `docs/*` | Documentation-only changes. |
| `chore/*` | Tooling, CI, deps — no product logic. |

## Naming convention

```
<type>/<short-kebab-description>
```

Examples:

- `feature/phase0-p2p-reliability`
- `feature/vps-relay-binary`
- `fix/invite-expiry-timezone`
- `docs/vps-relay-playbook`
- `chore/vite-host-flag`

Keep names **short**, **lowercase**, **hyphen-separated**. No issue numbers required (add if you use a tracker: `feature/42-p2p-relay`).

## Pull requests

**Default integration branch: `development`** — not `master`.

While the project is **solo**, you can **push directly to `development`** (docs, typos, small fixes). Use feature branches + PRs when you want a review checkpoint or a larger change set — not required for every commit.

**`master`** stays protected: stable cuts only via a **release PR** (`development` → `master`).

### Optional: feature branch + PR

1. Push your feature branch: `git push -u origin feature/my-feature`
2. Open a PR: **base** `development` ← **compare** `feature/my-feature`
3. Merge via PR (prefer **merge commit** to match `--no-ff` locally)
4. Delete the feature branch after merge

### Direct to `development` (solo, small changes)

```bash
git checkout development
git pull origin development
# … edit …
git commit -am "docs: fix README wording"
git push origin development
```

With GitHub CLI (when you do use a PR):

```bash
gh pr create --base development --head feature/my-feature \
  --title "Short title" \
  --body "## Summary\n- …\n\n## Test plan\n- [ ] …"
```

Release PRs are the exception: `development` → `master` when cutting a stable release.

## Labels

The repo uses **only these labels**. Apply **one primary label** per PR:

| Label | Color | Use when | Typical branch |
|-------|-------|----------|----------------|
| `feature` | green | New capability or user-facing behavior | `feature/*` |
| `bugfix` | red | Fixes incorrect behavior | `fix/*` |
| `docs` | blue | Documentation only | `docs/*` |
| `refactor` | purple | Restructure without intended behavior change | `refactor/*` or structural `chore/*` |
| `release` | amber | Promote `development` → `master` (stable cut) | `development` → `master` PR |

```bash
gh pr edit <number> --add-label feature
```

Release example (prefer the script — see [RELEASE.md](./RELEASE.md)):

```bash
./scripts/release-pr.sh 0.2.0
```

```powershell
powershell -ExecutionPolicy Bypass -File scripts/release-pr.ps1 -Version 0.2.0
```

Manual:

```bash
gh pr create --base master --head development \
  --title "Release v0.x.x" \
  --label release
```

## Branch protection

| Branch | Policy |
|--------|--------|
| `master` | **Protected** — no direct pushes; changes land only via merged PR (release flow: `development` → `master`). |
| `development` | **Open** — direct push while solo; optional PRs for larger work |

## Day-to-day

```bash
# Start new work
git checkout development
git pull origin development
git checkout -b feature/my-feature

# … commit …

# Finish (open a PR — base: development)
git push -u origin feature/my-feature
gh pr create --base development --head feature/my-feature

# Or merge locally after review
git checkout development
git merge --no-ff feature/my-feature
git push origin development
```

When `development` is ready for a release:

```bash
git checkout development
git pull origin development
./scripts/release-pr.sh 0.2.0
# merge the release PR on GitHub, then on master:
./scripts/release-tag.sh 0.2.0
```

```powershell
git checkout development
git pull origin development
powershell -ExecutionPolicy Bypass -File scripts/release-pr.ps1 -Version 0.2.0
# merge the release PR on GitHub, then on master:
powershell -ExecutionPolicy Bypass -File scripts/release-tag.ps1 -Version 0.2.0
```

Full details: [RELEASE.md](./RELEASE.md).

## Hotfix (rare)

Urgent fix on `master` without waiting for other `development` work:

```bash
git checkout master
git pull origin master
git checkout -b fix/critical-bug
# … fix …
gh pr create --base master --head fix/critical-bug --label bugfix
# after merge to master:
git checkout development && git merge --no-ff fix/critical-bug
```

## Rules

- **Never force-push** `master` or `development` without team agreement.
- **One feature per branch** — easier review and revert.
- **Merge with `--no-ff`** when integrating features (keeps branch history visible).
- Commits: imperative subject, body explains *why* when not obvious.

## Current milestone

VPS relay work is tracked in [MILESTONE-VPS-RELAY.md](./MILESTONE-VPS-RELAY.md). Phase 0 branches use the `feature/phase0-*` prefix.
