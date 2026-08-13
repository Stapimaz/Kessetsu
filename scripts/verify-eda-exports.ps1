param(
    [switch]$RequireApplications
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$corePath = Join-Path $repoRoot 'core'
$resultPath = Join-Path $repoRoot 'webapp/test-results/eda-smoke'
$kess = Join-Path $corePath 'target/release/kess.exe'

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
$fixtures = @('rc_filter', 'gain_stage', 'power_amplifier')
foreach ($name in $fixtures) {
    $source = Join-Path $corePath "tests/fixtures/benchmarks/$name.kess"
    $kicadSchematic = Join-Path $resultPath "$name.kicad_sch"
    $kicadNetlist = Join-Path $resultPath "$name.kicad.xml"
    $kicadErc = Join-Path $resultPath "$name.erc.rpt"
    $ltspiceSchematic = Join-Path $resultPath "$name.asc"
    $ltspiceNetlist = Join-Path $resultPath "$name.net"

    & $kess export $source --target kicad --output $kicadSchematic --force
    if ($LASTEXITCODE -ne 0) { throw "$name KiCad export failed." }
    & $kicad sch export netlist --format kicadxml --output $kicadNetlist $kicadSchematic
    if ($LASTEXITCODE -ne 0) { throw "$name did not open in KiCad." }
    & $kicad sch erc --output $kicadErc $kicadSchematic
    if ($LASTEXITCODE -ne 0) { throw "$name KiCad ERC invocation failed." }

    & $kess export $source --target ltspice --output $ltspiceSchematic --force
    if ($LASTEXITCODE -ne 0) { throw "$name LTspice export failed." }
    $process = Start-Process -FilePath $ltspice -ArgumentList @('-netlist', $ltspiceSchematic) -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $ltspiceNetlist)) {
        throw "$name did not open in LTspice (exit $($process.ExitCode))."
    }

    $sourceText = Get-Content -LiteralPath $source
    $references = $sourceText | ForEach-Object {
        if ($_ -match '^\s*(?:resistor|capacitor|inductor|diode|source|current_source|bjt|mosfet|opamp)\s+(\w+)') {
            $Matches[1]
        }
    }
    $kicadText = Get-Content -LiteralPath $kicadNetlist -Raw
    $ltspiceText = Get-Content -LiteralPath $ltspiceNetlist -Raw
    foreach ($reference in $references) {
        $kicadReference = '<comp ref="' + $reference + '">'
        if ($kicadText -notmatch [regex]::Escape($kicadReference)) {
            throw "${name}: KiCad netlist lost component $reference."
        }
        if ($ltspiceText -notmatch "(?m)^\S*$([regex]::Escape($reference))\s") {
            throw "${name}: LTspice netlist lost component $reference."
        }
    }
    if ($ltspiceText -notmatch '(?m)^\.end\s*$') { throw "${name}: LTspice netlist is incomplete." }
    Write-Host "EDA smoke passed: $name"
}
