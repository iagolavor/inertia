#!/usr/bin/env bash
# Opens a release PR: development -> master with a structured body.
# Usage: ./scripts/release-pr.sh 0.2.0
set -euo pipefail

VERSION="${1:?Usage: $0 <version> (e.g. 0.2.0 or v0.2.0)}"
if [[ "$VERSION" =~ ^v ]]; then
  tag="$VERSION"
else
  tag="v$VERSION"
fi
title="Release $tag"

git fetch origin master development 2>/dev/null || true

range="origin/master..origin/development"
ahead="$(git rev-list --count "$range")"
if [[ "$ahead" -eq 0 ]]; then
  echo "Nothing to release: development is not ahead of master."
  exit 1
fi

pr_lines=()
while IFS= read -r subject; do
  [[ -z "$subject" ]] && continue
  if [[ "$subject" =~ Merge\ pull\ request\ \#([0-9]+) ]]; then
    num="${BASH_REMATCH[1]}"
    if gh pr view "$num" >/dev/null 2>&1; then
      pr_title="$(gh pr view "$num" --json title -q .title)"
      pr_url="$(gh pr view "$num" --json url -q .url)"
      label="$(gh pr view "$num" --json labels -q '.labels[0].name // ""')"
      if [[ -n "$label" ]]; then
        pr_lines+=("- #${num} [${label}] ${pr_title} (${pr_url})")
      else
        pr_lines+=("- #${num} ${pr_title} (${pr_url})")
      fi
    else
      pr_lines+=("- #${num} (see GitHub)")
    fi
  fi
done < <(git log "$range" --merges --pretty=format:%s 2>/dev/null || true)

last_tag="$(git describe --tags --abbrev=0 origin/master 2>/dev/null || true)"
if [[ -n "$last_tag" ]]; then
  since_line="Since ${last_tag} on master."
else
  since_line="First tagged release."
fi

if [[ ${#pr_lines[@]} -gt 0 ]]; then
  changes="$(printf '%s\n' "${pr_lines[@]}")"
else
  changes="- (no merge commits parsed — list highlights manually)"
fi

body="$(cat <<EOF
## Summary

Promote development to master — **${tag}**.

${since_line}

**${ahead}** commit(s) on development not yet on master.

## Included changes

${changes}

## Pre-merge checklist

- [ ] development is green / smoke-tested locally
- [ ] No known blockers for a stable cut

## After merge

On master after pulling: \`./scripts/release-tag.sh ${tag}\` or \`powershell -ExecutionPolicy Bypass -File scripts/release-tag.ps1 -Version ${tag}\`
EOF
)"

echo "Creating release PR: $title"
gh pr create --base master --head development --title "$title" --label release --body "$body"
