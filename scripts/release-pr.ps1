# Opens a release PR: development -> master with a structured body.
# Usage: powershell -ExecutionPolicy Bypass -File scripts/release-pr.ps1 -Version 0.2.0
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

$tag = if ($Version -match '^v') { $Version } else { "v$Version" }
$title = "Release $tag"

Invoke-InertiaGit fetch origin master development

$range = "origin/master..origin/development"
$ahead = Invoke-InertiaGit rev-list --count $range
if ([int]$ahead -eq 0) {
    Write-Host "Nothing to release: development is not ahead of master."
    exit 1
}

$prLines = @()
$mergeSubjects = Invoke-InertiaGit log $range --merges --pretty=format:%s
foreach ($subject in $mergeSubjects) {
    if ($subject -match 'Merge pull request #(\d+)') {
        $num = $Matches[1]
        $pr = gh pr view $num --json title,url,labels 2>$null | ConvertFrom-Json
        if ($pr) {
            $label = ($pr.labels | Select-Object -First 1).name
            $labelSuffix = if ($label) { " [$label]" } else { "" }
            $prLines += "- #$num$labelSuffix $($pr.title) ($($pr.url))"
        } else {
            $prLines += "- #$num (see GitHub)"
        }
    }
}

$lastTag = git describe --tags --abbrev=0 origin/master 2>$null
$sinceLine = if ($lastTag) { "Since $lastTag on master." } else { "First tagged release." }
$changes = if ($prLines.Count -gt 0) { $prLines -join "`n" } else { "- (no merge commits parsed - list highlights manually)" }

$body = @"
## Summary

Promote development to master - **$tag**.

$sinceLine

**$ahead** commit(s) on development not yet on master.

## Included changes

$changes

## Pre-merge checklist

- [ ] development is green / smoke-tested locally
- [ ] No known blockers for a stable cut

## After merge

On master after pulling: `./scripts/release-tag.sh $tag` or `powershell -ExecutionPolicy Bypass -File scripts/release-tag.ps1 -Version $tag`
"@

Write-Host "Creating release PR: $title"
gh pr create --base master --head development --title $title --label release --body $body
