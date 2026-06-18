# Tags master and publishes a GitHub Release after a release PR is merged.
# Usage: powershell -File scripts/release-tag.ps1 -Version v0.2.0
param(
    [Parameter(Mandatory = $true)]
    [string]$Version
)

$ErrorActionPreference = "Stop"
$tag = if ($Version -match '^v') { $Version } else { "v$Version" }

git fetch origin master 2>$null | Out-Null
git checkout master
git pull origin master

if (git rev-parse $tag 2>$null) {
    Write-Host "Tag $tag already exists locally."
    exit 1
}

git tag -a $tag -m "Release $tag"
git push origin $tag

Write-Host "Creating GitHub release for $tag..."
gh release create $tag --title $tag --generate-notes

Write-Host "Done. Release: https://github.com/iagolavor/inertia/releases/tag/$tag"
