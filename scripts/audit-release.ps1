$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$corePath = Join-Path $repoRoot "core"
$webPath = Join-Path $repoRoot "webapp"
$npmCommand = if ($env:OS -eq "Windows_NT") { "npm.cmd" } else { "npm" }

$metadata = (& cargo metadata --manifest-path (Join-Path $corePath "Cargo.toml") --format-version 1 --locked | Out-String) | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw "cargo metadata failed" }
$rustDependencies = @($metadata.packages | Where-Object { $_.name -ne "netlang-core" })
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

$forbiddenPatterns = @("*.spice", "*.asc", "*.kicad_sch", "*.netlang.json")
$trackedGenerated = @()
foreach ($pattern in $forbiddenPatterns) {
    $trackedGenerated += @(& git -C $repoRoot ls-files $pattern | Where-Object { $_ -and -not $_.StartsWith("core/tests/fixtures/") })
}
if ($trackedGenerated.Count -gt 0) { throw "Generated circuit artifacts are tracked unexpectedly: $($trackedGenerated -join ', ')" }
Write-Host "Generated-artifact audit PASS."
