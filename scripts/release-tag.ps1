# Tags master and publishes a GitHub Release after a release PR is merged.
# Usage: powershell -File scripts/release-tag.ps1 -Version v0.2.0
param(
    [Parameter(Mandatory = $true)]
    [string]$Version
)

$ErrorActionPreference = "Stop"

function Invoke-InertiaGit {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$GitArgs)
    $prev = $ErrorActionPreference
    $ErrorActionPreference = "SilentlyContinue"
    $out = & git @GitArgs 2>&1
    $code = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($code -ne 0) { throw "git $($GitArgs -join ' ') failed (exit $code)" }
    return $out
}

function Test-InertiaGitRef {
    param([string]$Ref)
    $prev = $ErrorActionPreference
    $ErrorActionPreference = "SilentlyContinue"
    & git rev-parse $Ref 2>$null | Out-Null
    $ok = ($LASTEXITCODE -eq 0)
    $ErrorActionPreference = $prev
    return $ok
}

$tag = if ($Version -match '^v') { $Version } else { "v$Version" }

Invoke-InertiaGit fetch origin master
Invoke-InertiaGit checkout master
Invoke-InertiaGit pull origin master

if (Test-InertiaGitRef $tag) {
    Write-Host "Tag $tag already exists locally."
    exit 1
}

Invoke-InertiaGit tag -a $tag -m "Release $tag"
Invoke-InertiaGit push origin $tag

Write-Host "Tag $tag pushed."
Write-Host "GitHub Actions builds inertia-windows-x64.zip and publishes the release."
Write-Host "Track: https://github.com/iagolavor/inertia/actions"
Write-Host "Release: https://github.com/iagolavor/inertia/releases/tag/$tag"
