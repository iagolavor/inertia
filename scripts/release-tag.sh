#!/usr/bin/env bash
# Tags master and publishes a GitHub Release after a release PR is merged.
# Usage: ./scripts/release-tag.sh 0.2.0
set -euo pipefail

VERSION="${1:?Usage: $0 <version> (e.g. 0.2.0 or v0.2.0)}"
if [[ "$VERSION" =~ ^v ]]; then
  tag="$VERSION"
else
  tag="v$VERSION"
fi

git fetch origin master 2>/dev/null || true
git checkout master
git pull origin master

if git rev-parse "$tag" >/dev/null 2>&1; then
  echo "Tag $tag already exists locally."
  exit 1
fi

git tag -a "$tag" -m "Release $tag"
git push origin "$tag"

echo "Tag $tag pushed."
echo "GitHub Actions builds Windows zip + desktop installers (NSIS, RPM, AppImage) + Android APK and publishes the release."
echo "Track: https://github.com/iagolavor/inertia/actions"
echo "Release: https://github.com/iagolavor/inertia/releases/tag/$tag"
