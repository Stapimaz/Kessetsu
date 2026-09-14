param(
    [Parameter(Mandatory = $true)]
    [string]$Archive
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$archivePath = [System.IO.Path]::GetFullPath($Archive)
if (-not (Test-Path -LiteralPath $archivePath -PathType Leaf)) { throw "Archive is missing: $archivePath" }
$smokeRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("kessetsu-release-smoke-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $smokeRoot | Out-Null
try {
    if ($archivePath.EndsWith(".zip", [System.StringComparison]::OrdinalIgnoreCase)) {
        Expand-Archive -LiteralPath $archivePath -DestinationPath $smokeRoot
    } else {
        & tar -C $smokeRoot -xzf $archivePath
        if ($LASTEXITCODE -ne 0) { throw "tar extraction failed with exit code $LASTEXITCODE" }
    }
    $manifestPath = Get-ChildItem -LiteralPath $smokeRoot -Filter release-manifest.json -Recurse | Select-Object -First 1 -ExpandProperty FullName
    if (-not $manifestPath) { throw "Archive does not contain release-manifest.json" }
    $bundleRoot = Split-Path -Parent $manifestPath
    $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $binary = Join-Path $bundleRoot $manifest.executable
    $actualHash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualHash -ne $manifest.executable_sha256) { throw "Executable checksum mismatch" }

    $versionOutput = (& $binary --version | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) { throw "CLI version probe failed" }
    if ($versionOutput -ne "kess $($manifest.version)") {
        throw "CLI version '$versionOutput' does not match release manifest '$($manifest.version)'"
    }
    if ($manifest.simulator.policy -eq "bundled-ngspice-46") {
        $env:KESSETSU_NGSPICE = Join-Path $bundleRoot "tools/ngspice/bin/ngspice_con.exe"
    }
    $source = Join-Path $repoRoot "core/tests/fixtures/benchmarks/power_amplifier.kess"
    $output = Join-Path $smokeRoot "power-amplifier.spice"
    $json = (& $binary test $source --output $output --format json | Out-String) | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or $json.status -ne "success" -or $json.assertions.summary.passed -ne 12) {
        throw "Clean release smoke did not pass all 12 power-amplifier assertions"
    }
    if (-not $json.debug -and -not $json.domain_versions.simulation) { throw "Simulation provenance contract is missing" }
    Write-Host "Release smoke PASS: $($manifest.target), $($json.assertions.summary.passed)/12 assertions."
}
finally {
    Remove-Item -LiteralPath $smokeRoot -Recurse -Force -ErrorAction SilentlyContinue
}
