$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$corePath = Join-Path $repoRoot "core"
$webPath = Join-Path $repoRoot "webapp"
$npmCommand = if ($env:OS -eq "Windows_NT") { "npm.cmd" } else { "npm" }

& (Join-Path $PSScriptRoot "verify-version.ps1") | Out-Null
& (Join-Path $PSScriptRoot "audit-public-docs.ps1") -RepositoryRoot $repoRoot | Out-Null

$metadata = (& cargo metadata --manifest-path (Join-Path $corePath "Cargo.toml") --format-version 1 --locked | Out-String) | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw "cargo metadata failed" }
$projectPackage = $metadata.packages | Where-Object { $_.name -eq "kessetsu-core" } | Select-Object -First 1
if (-not $projectPackage -or $projectPackage.license -ne "AGPL-3.0-only") { throw "Kessetsu package license must be AGPL-3.0-only" }
foreach ($projectNotice in @("LICENSE", "NOTICE", "COMMERCIAL_LICENSE.md")) {
    if (-not (Test-Path -LiteralPath (Join-Path $repoRoot $projectNotice) -PathType Leaf)) {
        throw "Project license artifact is missing: $projectNotice"
    }
}
$rootLicenseHash = (Get-FileHash -LiteralPath (Join-Path $repoRoot "LICENSE") -Algorithm SHA256).Hash
$coreLicenseHash = (Get-FileHash -LiteralPath (Join-Path $corePath "LICENSE") -Algorithm SHA256).Hash
if ($rootLicenseHash -ne $coreLicenseHash) { throw "core/LICENSE must exactly match the canonical root LICENSE" }
$pythonLicenseHash = (Get-FileHash -LiteralPath (Join-Path $repoRoot "python/LICENSE") -Algorithm SHA256).Hash
if ($rootLicenseHash -ne $pythonLicenseHash) { throw "python/LICENSE must exactly match the canonical root LICENSE" }
$pythonProject = Get-Content -LiteralPath (Join-Path $repoRoot "python/pyproject.toml") -Raw
if ($pythonProject -notmatch '(?m)^license\s*=\s*"AGPL-3\.0-only"\s*$') {
    throw "Kessetsu Python package license must be AGPL-3.0-only"
}
Write-Host "Project license audit PASS: AGPL-3.0-only plus explicit commercial-license notice."
$rustDependencies = @($metadata.packages | Where-Object { $_.name -ne "kessetsu-core" })
$missingRustLicenses = @($rustDependencies | Where-Object { -not $_.license })
$copyleftRustLicenses = @($rustDependencies | Where-Object { $_.license -match "(^|[^A-Z])(AGPL|GPL|SSPL)(-|\b)" })
if ($missingRustLicenses.Count -gt 0) { throw "Rust dependencies without license metadata: $($missingRustLicenses.name -join ', ')" }
if ($copyleftRustLicenses.Count -gt 0) { throw "Rust dependencies require copyleft review: $($copyleftRustLicenses.name -join ', ')" }
Write-Host "Rust license audit PASS: $($rustDependencies.Count) dependency packages."

Push-Location $webPath
try {
    & node scripts/audit-licenses.mjs
    if ($LASTEXITCODE -ne 0) { throw "npm license audit failed" }
    & $npmCommand audit --omit=dev
    if ($LASTEXITCODE -ne 0) { throw "npm production security audit failed" }
}
finally { Pop-Location }

$forbiddenPatterns = @("*.spice", "*.asc", "*.kicad_sch", "*.kessetsu.json")
$trackedGenerated = @()
foreach ($pattern in $forbiddenPatterns) {
    $trackedGenerated += @(& git -C $repoRoot ls-files $pattern | Where-Object { $_ -and -not $_.StartsWith("core/tests/fixtures/") })
}
if ($trackedGenerated.Count -gt 0) { throw "Generated circuit artifacts are tracked unexpectedly: $($trackedGenerated -join ', ')" }
Write-Host "Generated-artifact audit PASS."
