[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidatePattern('^v\d+\.\d+\.\d+$')]
    [string]$Tag
)

$ErrorActionPreference = 'Stop'

$branch = (git branch --show-current).Trim()
if ($branch -ne 'main') {
    throw "Release tags may only be created from main. Current branch: $branch"
}

if ((git status --porcelain)) {
    throw 'Working tree must be clean before creating a release tag.'
}

$repository = (gh repo view --json nameWithOwner --jq '.nameWithOwner').Trim()
if (-not $repository) {
    throw 'Unable to resolve the GitHub repository with gh CLI.'
}

$mainSha = (gh api "repos/$repository/branches/main" --jq '.commit.sha').Trim()
$headSha = (git rev-parse HEAD).Trim()
if ($headSha -ne $mainSha) {
    throw "HEAD must equal origin/main before tagging. HEAD=$headSha origin/main=$mainSha"
}

$packageVersion = ((cargo metadata --locked --no-deps --format-version 1 | ConvertFrom-Json).packages[0].version)
if ($Tag -ne "v$packageVersion") {
    throw "Tag $Tag does not match Cargo package version v$packageVersion."
}

$remoteTag = git ls-remote --tags origin "refs/tags/$Tag"
if ($LASTEXITCODE -eq 0 -and $remoteTag) {
    throw "Remote tag $Tag already exists. Release tags are immutable."
}

# Create the ref through GitHub CLI so the tag points exactly at the verified
# main commit. The tagged-release workflow then validates the same ancestry.
gh api --method POST "repos/$repository/git/refs" `
    -f "ref=refs/tags/$Tag" `
    -f "sha=$headSha"

Write-Host "Created $Tag from main commit $headSha."
