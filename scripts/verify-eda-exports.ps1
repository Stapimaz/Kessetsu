param(
    [switch]$RequireApplications,
    [string[]]$Fixtures = @(
        'rc_filter', 'gain_stage', 'power_amplifier',
        'external_comparator', 'external_memristor',
        'loaded_filter', 'transistor_driver', 'reusable_filters', 'reusable_amplifiers'
    )
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$corePath = Join-Path $repoRoot 'core'
$resultPath = Join-Path $repoRoot 'webapp/test-results/eda-smoke'
$kess = Join-Path $corePath 'target/release/kess.exe'
. (Join-Path $PSScriptRoot 'compare-eda-netlists.ps1')

$kicad = Get-Command 'kicad-cli' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -First 1
if (-not $kicad) {
    $candidate = Get-ChildItem 'C:\Program Files\KiCad' -Recurse -Filter 'kicad-cli.exe' -ErrorAction SilentlyContinue |
        Sort-Object FullName -Descending | Select-Object -ExpandProperty FullName -First 1
    $kicad = $candidate
}
$ltspiceCandidates = @(
    (Join-Path $env:LOCALAPPDATA 'Programs/ADI/LTspice/LTspice.exe'),
    'C:\Program Files\ADI\LTspice\LTspice.exe'
)
$ltspice = $ltspiceCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1

if (-not $kicad -or -not $ltspice) {
    $message = "EDA smoke skipped: KiCad CLI=$([bool]$kicad), LTspice=$([bool]$ltspice)."
    if ($RequireApplications) { throw $message }
    Write-Host $message
    exit 0
}
Push-Location $corePath
try { cargo build --release } finally { Pop-Location }
if ($LASTEXITCODE -ne 0) { throw 'Kessetsu release build failed.' }

New-Item -ItemType Directory -Force $resultPath | Out-Null
foreach ($name in $Fixtures) {
    $benchmarkSource = Join-Path $corePath "tests/fixtures/benchmarks/$name.kess"
    $exampleSource = Join-Path $repoRoot "examples/$name.kess"
    $source = if (Test-Path -LiteralPath $benchmarkSource) { $benchmarkSource } else { $exampleSource }
    if ($name.StartsWith('external_')) {
        $source = Join-Path $resultPath "$name.kess"
        Copy-Item -LiteralPath (Join-Path $repoRoot "examples/$name.kess") -Destination $source -Force
        New-Item -ItemType Directory -Force (Join-Path $resultPath 'models') | Out-Null
        foreach ($modelFile in @('comparator.lib', 'memristor.lib', 'MEMRISTOR_NOTICE.md')) {
            Copy-Item -LiteralPath (Join-Path $repoRoot "examples/models/$modelFile") -Destination (Join-Path $resultPath "models/$modelFile") -Force
        }
    }
    if (-not (Test-Path -LiteralPath $source)) { throw "Unknown EDA fixture: $name" }
    $kicadSchematic = Join-Path $resultPath "$name.kicad_sch"
    $kicadNetlist = Join-Path $resultPath "$name.kicad.xml"
    $kicadErc = Join-Path $resultPath "$name.erc.rpt"
    $ltspiceSchematic = Join-Path $resultPath "$name.asc"
    $ltspiceNetlist = Join-Path $resultPath "$name.net"
    $schematicJson = Join-Path $resultPath "$name.schematic.json"
    $canonicalSpice = Join-Path $resultPath "$name.spice"

    & $kess export $source --target kicad --output $kicadSchematic --force
    if ($LASTEXITCODE -ne 0) { throw "$name KiCad export failed." }
    & $kicad sch export netlist --format kicadxml --output $kicadNetlist $kicadSchematic
    if ($LASTEXITCODE -ne 0) { throw "$name did not open in KiCad." }
    & $kicad sch erc --output $kicadErc $kicadSchematic
    if ($LASTEXITCODE -ne 0) { throw "$name KiCad ERC invocation failed." }
    $ercText = Get-Content -LiteralPath $kicadErc -Raw -Encoding UTF8
    $ercSummary = [regex]::Match($ercText, '\*\* ERC messages:\s+(?<total>\d+)\s+Errors\s+(?<errors>\d+)\s+Warnings\s+(?<warnings>\d+)')
    if (-not $ercSummary.Success) { throw "$name KiCad ERC report has no parseable summary." }
    $ercErrors = [int]$ercSummary.Groups['errors'].Value
    $ercWarnings = [int]$ercSummary.Groups['warnings'].Value
    $ercViolations = @([regex]::Matches($ercText, '(?m)^\[([^\]]+)\]:') | ForEach-Object { $_.Groups[1].Value })
    $unexpectedViolations = @($ercViolations | Where-Object { $_ -ne 'lib_symbol_issues' })
    if ($ercErrors -ne 0 -or $unexpectedViolations.Count -ne 0) {
        throw "$name KiCad ERC found $ercErrors error(s) or unexpected violation types: $($unexpectedViolations -join ', ')."
    }
    if ([int]$ercSummary.Groups['total'].Value -ne $ercViolations.Count) {
        throw "$name KiCad ERC summary/violation count mismatch."
    }
    Write-Host "KiCad ERC accepted: 0 errors; $ercWarnings embedded-symbol library warning(s)."

    & $kess export $source --target ltspice --output $ltspiceSchematic --force
    if ($LASTEXITCODE -ne 0) { throw "$name LTspice export failed." }
    $process = Start-Process -FilePath $ltspice -ArgumentList @('-netlist', $ltspiceSchematic) -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $ltspiceNetlist)) {
        throw "$name did not open in LTspice (exit $($process.ExitCode))."
    }

    & $kess export $source --target schematic-json --output $schematicJson --force
    if ($LASTEXITCODE -ne 0) { throw "$name canonical schematic export failed." }
    & $kess export $source --target spice --output $canonicalSpice --force
    if ($LASTEXITCODE -ne 0) { throw "$name canonical SPICE export failed." }
    $schematic = Get-Content -LiteralPath $schematicJson -Raw -Encoding UTF8 | ConvertFrom-Json
    [xml]$kicadXml = Get-Content -LiteralPath $kicadNetlist -Raw -Encoding UTF8
    Assert-EdaNetlists $schematic $kicadXml (Get-Content -LiteralPath $ltspiceNetlist -Raw -Encoding UTF8) (Get-Content -LiteralPath $canonicalSpice -Raw -Encoding UTF8) (Get-Content -LiteralPath $ltspiceSchematic -Raw -Encoding UTF8)
    Write-Host "EDA smoke passed: $name"
}
