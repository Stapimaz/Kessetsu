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

    $example = Join-Path $bundleRoot "examples/rc_low_pass.kess"
    if (-not (Test-Path -LiteralPath $example -PathType Leaf)) { throw "Release is missing the newcomer RC example" }
    $check = (& $binary check $example --format json | Out-String) | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or $check.status -ne "success") { throw "Packaged newcomer example did not pass check" }

    $newcomerSpice = Join-Path $smokeRoot "rc-low-pass.spice"
    $newcomer = (& $binary test $example --output $newcomerSpice --format json | Out-String) | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or $newcomer.status -ne "success" -or $newcomer.assertions.summary.passed -ne 5) {
        throw "Packaged newcomer example did not pass all 5 assertions"
    }
    $newcomerSvg = Join-Path $smokeRoot "rc-low-pass.svg"
    & $binary render $example --output $newcomerSvg --format json | Out-Null
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $newcomerSvg -PathType Leaf)) {
        throw "Packaged newcomer SVG export failed"
    }
    $newcomerKicad = Join-Path $smokeRoot "rc-low-pass.kicad_sch"
    & $binary export $example --target kicad --output $newcomerKicad --format json | Out-Null
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $newcomerKicad -PathType Leaf)) {
        throw "Packaged newcomer KiCad export failed"
    }

    $agentDesign = Join-Path $bundleRoot "examples/agent_rc_design.kess"
    $agentRequirements = Join-Path $bundleRoot "examples/agent_rc_requirements.kessreq"
    $requirementsHash = (Get-FileHash -LiteralPath $agentRequirements -Algorithm SHA256).Hash.ToLowerInvariant()
    $agentOutput = Join-Path $smokeRoot "agent-rc.spice"
    $agent = (& $binary test $agentDesign --requirements $agentRequirements --requirements-sha256 $requirementsHash --output $agentOutput --format json | Out-String) | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or $agent.status -ne "success" -or $agent.requirements.hash_pinned -ne $true -or $agent.assertions.summary.passed -ne 5) {
        throw "Packaged evaluator-owned requirements workflow failed"
    }
    Write-Host "Newcomer smoke PASS: check, 5/5 inline assertions, SVG, KiCad, and 5/5 hash-pinned external requirements."

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
