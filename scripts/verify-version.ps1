param(
    [string]$ExpectedVersion = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$version = (Get-Content -LiteralPath (Join-Path $repoRoot "VERSION") -Raw).Trim()
if ($version -notmatch '^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$') {
    throw "VERSION is not a valid Semantic Version: '$version'"
}
if ($ExpectedVersion -and $ExpectedVersion -ne $version) {
    throw "Requested version '$ExpectedVersion' does not match canonical VERSION '$version'"
}

$cargo = (& cargo metadata --manifest-path (Join-Path $repoRoot "core/Cargo.toml") --format-version 1 --locked --no-deps | Out-String) | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw "cargo metadata failed while verifying the product version" }
$cargoVersion = ($cargo.packages | Where-Object { $_.name -eq "kessetsu-core" } | Select-Object -First 1).version
$npmPackageText = Get-Content -LiteralPath (Join-Path $repoRoot "webapp/package.json") -Raw
$npmLockText = Get-Content -LiteralPath (Join-Path $repoRoot "webapp/package-lock.json") -Raw
$npmPackageVersion = [regex]::Match($npmPackageText, '"version"\s*:\s*"([^\"]+)"').Groups[1].Value
$npmLockVersions = [regex]::Matches($npmLockText, '"version"\s*:\s*"([^\"]+)"')
$pythonProjectText = Get-Content -LiteralPath (Join-Path $repoRoot "python/pyproject.toml") -Raw
$pythonModuleText = Get-Content -LiteralPath (Join-Path $repoRoot "python/src/kessetsu/__init__.py") -Raw
$pythonProjectVersion = [regex]::Match($pythonProjectText, '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
$pythonModuleVersion = [regex]::Match($pythonModuleText, '(?m)^__version__\s*=\s*"([^"]+)"').Groups[1].Value
if (-not $npmPackageVersion -or $npmLockVersions.Count -lt 2) {
    throw "Could not read npm product versions"
}
$versions = [ordered]@{
    VERSION = $version
    Cargo = $cargoVersion
    NpmPackage = $npmPackageVersion
    NpmLock = $npmLockVersions[0].Groups[1].Value
    NpmLockRoot = $npmLockVersions[1].Groups[1].Value
    PythonProject = $pythonProjectVersion
    PythonModule = $pythonModuleVersion
}
$mismatches = @($versions.GetEnumerator() | Where-Object { $_.Value -ne $version })
if ($mismatches.Count -gt 0) {
    $details = ($mismatches | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join ', '
    throw "Product version mismatch: $details"
}

$changelog = Get-Content -LiteralPath (Join-Path $repoRoot "CHANGELOG.md") -Raw
if ($changelog -notmatch '(?m)^## \[Unreleased\]') { throw "CHANGELOG.md must contain an [Unreleased] section" }
if ($changelog -notmatch "(?m)^## \[$([regex]::Escape($version))\](?:\s|$)") {
    throw "CHANGELOG.md has no section for canonical version $version"
}

Write-Host "Product version consistency PASS: $version"
Write-Output $version
