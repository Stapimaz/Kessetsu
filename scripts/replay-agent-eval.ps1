param(
    [string]$Binary = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$sourcePath = Join-Path $repoRoot "core/tests/fixtures/benchmarks/power_amplifier.kess"
if (-not $Binary) {
    $name = if ($IsWindows -or $env:OS -eq "Windows_NT") { "kess.exe" } else { "kess" }
    $Binary = Join-Path $repoRoot "core/target/release/$name"
}
if (-not (Test-Path -LiteralPath $Binary -PathType Leaf)) {
    throw "Kessetsu release binary not found at $Binary. Run cargo build --release first."
}

$finalSource = Get-Content -LiteralPath $sourcePath -Raw -Encoding UTF8
$candidate = $finalSource.Replace("resistor RL 8", "resistor RL 16")
$candidateJson = ($candidate | & $Binary test - --format json | Out-String) | ConvertFrom-Json
if ($candidateJson.status -ne "test_failed" -or $candidateJson.assertions.summary.failed -ne 1) {
    throw "Agent eval iteration 0 no longer produces exactly one structured failure."
}
$failed = @($candidateJson.assertions.assertions | Where-Object { $_.status -eq "FAIL" })
if ($failed.Count -ne 1 -or $failed[0].code -ne "KES-T003" -or $failed[0].metric -ne "output_power") {
    throw "Agent eval iteration 0 failure identity changed."
}

$finalJson = ($finalSource | & $Binary test - --format json | Out-String) | ConvertFrom-Json
if ($finalJson.status -ne "success" -or $finalJson.assertions.summary.passed -ne 12) {
    throw "Agent eval final source did not pass all 12 assertions."
}
Write-Host "Agent eval PASS: 16 ohm candidate fails KES-T003; 8 ohm revision passes 12/12."
