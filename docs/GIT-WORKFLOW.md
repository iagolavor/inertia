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

**Default base branch: `development`** — not `master`.

1. Push your feature branch: `git push -u origin feature/my-feature`
2. Open a PR: **base** `development` ← **compare** `feature/my-feature`
3. Merge via PR (squash or merge commit — prefer **merge commit** to match `--no-ff` locally)
4. Delete the feature branch after merge

With GitHub CLI:

```bash
gh pr create --base development --head feature/my-feature \
  --title "Short title" \
  --body "## Summary\n- …\n\n## Test plan\n- [ ] …"
```

Release PRs are the exception: `development` → `master` when cutting a stable release.

## Labels

Use **one primary label** per PR (and issue, when relevant). Names match branch intent:

| Label | Color | Use when | Typical branch |
|-------|-------|----------|----------------|
| `feature` | green | New capability or user-facing behavior | `feature/*` |
| `bugfix` | red | Fixes incorrect behavior | `fix/*` |
| `docs` | blue | Documentation only | `docs/*` |
| `refactor` | purple | Restructure without intended behavior change | `refactor/*` or structural `chore/*` |

```bash
gh pr edit <number> --add-label feature
```

**Prefer these four** over default GitHub labels (`enhancement`, `bug`, `documentation`). Legacy labels may remain on the repo but should not be used for new work.

Optional later: `chore` (tooling/CI/deps), `release` (`development` → `master`).

## Branch protection

| Branch | Policy |
|--------|--------|
| `master` | **Protected** — no direct pushes; changes land only via merged PR (release flow: `development` → `master`). |
| `development` | Open for direct push while the project is solo; switch to PR-only when collaborators join. |

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
gh pr create --base master --head development --title "Release v0.x.x"
# merge via GitHub — direct push to master is blocked
git tag v0.x.x   # optional, after merge
git push origin v0.x.x
```

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
