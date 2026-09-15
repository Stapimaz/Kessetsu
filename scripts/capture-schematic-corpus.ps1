param(
    [string]$OutputDirectory,
    [switch]$SkipBuild,
    [switch]$IncludeWeb
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$corePath = Join-Path $repoRoot 'core'
$webPath = Join-Path $repoRoot 'webapp'
$isWindowsHost = $env:OS -eq 'Windows_NT'
$cliName = if ($isWindowsHost) { 'kess.exe' } else { 'kess' }
$cliPath = Join-Path $corePath "target/release/$cliName"
$npmCommand = if ($isWindowsHost) { 'npm.cmd' } else { 'npm' }

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $repoRoot '.artifacts/schematic-quality'
}
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)

function Invoke-Checked {
    param(
        [string]$Name,
        [scriptblock]$Command
    )
    Write-Host "==> $Name"
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE"
    }
}

if (-not $SkipBuild -or -not (Test-Path -LiteralPath $cliPath)) {
    Push-Location $corePath
    try {
        Invoke-Checked 'Build release CLI' { cargo build --release }
    }
    finally {
        Pop-Location
    }
}

if (-not (Test-Path -LiteralPath $cliPath)) {
    throw "Kessetsu CLI was not found at $cliPath"
}

New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null

$corpus = @(
    @{ name = 'minimal'; source = 'core/tests/fixtures/valid/minimal.kess'; family = 'minimal' },
    @{ name = 'rc-filter'; source = 'core/tests/fixtures/benchmarks/rc_filter.kess'; family = 'passive-filter' },
    @{ name = 'wheatstone'; source = 'examples/wheatstone_bridge.kess'; family = 'bridge' },
    @{ name = 'gain-stage'; source = 'core/tests/fixtures/benchmarks/gain_stage.kess'; family = 'feedback' },
    @{ name = 'high-fanout'; source = 'core/tests/fixtures/schematic/high_fanout.kess'; family = 'fanout' },
    @{ name = 'power-amplifier'; source = 'core/tests/fixtures/benchmarks/power_amplifier.kess'; family = 'multi-stage' },
    @{ name = 'inverting-amplifier'; source = 'core/tests/fixtures/schematic/inverting_amplifier.kess'; family = 'feedback' },
    @{ name = 'differential-pair'; source = 'core/tests/fixtures/schematic/differential_pair.kess'; family = 'symmetric' },
    @{ name = 'mosfet-common-source'; source = 'core/tests/fixtures/schematic/mosfet_common_source.kess'; family = 'transistor-stage' },
    @{ name = 'rlc-ladder'; source = 'core/tests/fixtures/schematic/rlc_ladder.kess'; family = 'passive-ladder' },
    @{ name = 'diode-clamp'; source = 'core/tests/fixtures/schematic/diode_clamp.kess'; family = 'clamp' },
    @{ name = 'bjt-common-emitter'; source = 'core/tests/fixtures/schematic/bjt_common_emitter.kess'; family = 'transistor-stage' },
    @{ name = 'summing-amplifier'; source = 'core/tests/fixtures/schematic/summing_amplifier.kess'; family = 'multi-input-feedback' }
)

$manifestEntries = @()
foreach ($entry in $corpus) {
    $sourcePath = Join-Path $repoRoot $entry.source
    $svgPath = Join-Path $OutputDirectory "$($entry.name).svg"
    $pngPath = Join-Path $OutputDirectory "$($entry.name).png"
    $jsonPath = Join-Path $OutputDirectory "$($entry.name).schematic.json"

    Invoke-Checked "Render $($entry.name) SVG" {
        & $cliPath render $sourcePath --output $svgPath --background white --force
    }
    Invoke-Checked "Render $($entry.name) PNG" {
        & $cliPath render $sourcePath --output $pngPath --scale 2 --background white --force
    }
    Invoke-Checked "Export $($entry.name) Schematic JSON" {
        & $cliPath export $sourcePath --target schematic-json --output $jsonPath --force
    }

    $schematic = Get-Content -Raw -LiteralPath $jsonPath | ConvertFrom-Json
    $boundsWidth = $schematic.bounds.max.x - $schematic.bounds.min.x
    $boundsHeight = $schematic.bounds.max.y - $schematic.bounds.min.y
    $connectedPins = [int]$schematic.connectivity.expected_connected_pins
    $labelRatio = if ($connectedPins -eq 0) { 0.0 } else { [double]$schematic.labels.Count / $connectedPins }
    $manifestEntries += [ordered]@{
        name = $entry.name
        family = $entry.family
        source = $entry.source
        artifacts = [ordered]@{
            svg = [System.IO.Path]::GetFileName($svgPath)
            png = [System.IO.Path]::GetFileName($pngPath)
            schematic_json = [System.IO.Path]::GetFileName($jsonPath)
        }
        counts = [ordered]@{
            components = $schematic.components.Count
            nets = $schematic.nets.Count
            wires = $schematic.wires.Count
            labels = $schematic.labels.Count
            junctions = $schematic.junctions.Count
            crossings = $schematic.crossings.Count
            bends = $schematic.quality.bends
        }
        geometry = [ordered]@{
            width = $boundsWidth
            height = $boundsHeight
            aspect_ratio = if ($boundsHeight -eq 0) { 0.0 } else { [Math]::Round($boundsWidth / $boundsHeight, 3) }
            labels_per_connected_pin = [Math]::Round($labelRatio, 3)
        }
        connectivity_verified = $schematic.connectivity.verified
        current_quality_passed = $schematic.quality.passed
        current_quality_issues = @($schematic.quality.issues)
        visual_review = [ordered]@{
            status = 'unreviewed'
            signal_flow = $null
            stage_grouping = $null
            wiring = $null
            label_use = $null
            power_convention = $null
            typography = $null
            composition = $null
            notes = @()
        }
    }
}

$manifest = [ordered]@{
    schema_version = 'kessetsu.schematic-quality-corpus.v1'
    generated_at_utc = [DateTime]::UtcNow.ToString('o')
    cli = $cliPath
    output_directory = $OutputDirectory
    entries = $manifestEntries
}
$manifest | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $OutputDirectory 'manifest.json') -Encoding utf8

$indexLines = @(
    '# Kessetsu schematic quality capture',
    '',
    'Generated artifacts are diagnostic evidence, not accepted visual goldens.',
    ''
)
foreach ($entry in $manifestEntries) {
    $indexLines += "## $($entry.name)"
    $indexLines += ''
    $indexLines += "- Family: $($entry.family)"
    $indexLines += "- Current gate: connectivity=$($entry.connectivity_verified), quality=$($entry.current_quality_passed)"
    $indexLines += "- Geometry: $($entry.geometry.width) x $($entry.geometry.height), aspect=$($entry.geometry.aspect_ratio)"
    $indexLines += "- Wires/labels/bends: $($entry.counts.wires) / $($entry.counts.labels) / $($entry.counts.bends)"
    $indexLines += ''
    $indexLines += "![$($entry.name)]($($entry.artifacts.png))"
    $indexLines += ''
}
$indexLines | Set-Content -LiteralPath (Join-Path $OutputDirectory 'index.md') -Encoding utf8

if ($IncludeWeb) {
    Push-Location $webPath
    try {
        $previousCapture = $env:KESSETSU_CAPTURE_VISUALS
        $env:KESSETSU_CAPTURE_VISUALS = '1'
        try {
            Invoke-Checked 'Capture fixed-viewport Web schematic corpus' {
                & $npmCommand run test:e2e -- schematic-corpus.spec.ts
            }
        }
        finally {
            $env:KESSETSU_CAPTURE_VISUALS = $previousCapture
        }
        $webCapture = Join-Path $webPath 'test-results/schematic-corpus'
        if (Test-Path -LiteralPath $webCapture) {
            Copy-Item -Recurse -Force -LiteralPath $webCapture -Destination (Join-Path $OutputDirectory 'web')
        }
    }
    finally {
        Pop-Location
    }
}

Write-Host "Schematic quality corpus captured at $OutputDirectory"
