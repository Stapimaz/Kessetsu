param(
    [Parameter(Mandatory = $true)]
    [string]$CliPath
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$artifactRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot '.artifacts'))
$environment = [System.IO.Path]::GetFullPath((Join-Path $artifactRoot 'python-verify'))
if (-not $environment.StartsWith($artifactRoot + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'Python verification environment escaped the artifact directory.'
}
if (Test-Path -LiteralPath $environment) {
    Remove-Item -LiteralPath $environment -Recurse -Force
}
New-Item -ItemType Directory -Path $artifactRoot -Force | Out-Null

python -m venv $environment
if ($LASTEXITCODE -ne 0) { throw 'Could not create isolated Python environment.' }
$python = if ($env:OS -eq 'Windows_NT') {
    Join-Path $environment 'Scripts/python.exe'
} else {
    Join-Path $environment 'bin/python'
}

& $python -m pip install --disable-pip-version-check --no-input "$repoRoot/python[verify]"
if ($LASTEXITCODE -ne 0) { throw 'Could not install the Python/notebook package.' }

$previousCli = $env:KESSETSU_TEST_CLI
$previousAdapter = $env:KESSETSU_CLI
$previousMatplotlibBackend = $env:MPLBACKEND
try {
    $env:KESSETSU_TEST_CLI = [System.IO.Path]::GetFullPath($CliPath)
    $env:KESSETSU_CLI = $env:KESSETSU_TEST_CLI
    $env:MPLBACKEND = 'Agg'
    & $python -m unittest discover -s (Join-Path $repoRoot 'python/tests') -p 'test_*.py'
    if ($LASTEXITCODE -ne 0) { throw 'Python adapter tests failed.' }
    & $python (Join-Path $repoRoot 'python/tests/run_notebook.py') `
        (Join-Path $repoRoot 'examples/notebooks/research-data-workflow.ipynb') `
        --working-directory $repoRoot `
        --output (Join-Path $artifactRoot 'research-data-workflow.executed.ipynb')
    if ($LASTEXITCODE -ne 0) { throw 'Headless research notebook failed.' }
    & $python (Join-Path $repoRoot 'python/tests/run_notebook.py') `
        (Join-Path $repoRoot 'examples/notebooks/finite-parameter-fit.ipynb') `
        --working-directory $repoRoot `
        --output (Join-Path $artifactRoot 'finite-parameter-fit.executed.ipynb')
    if ($LASTEXITCODE -ne 0) { throw 'Headless fitting notebook failed.' }
    & $python (Join-Path $repoRoot 'python/tests/run_notebook.py') `
        (Join-Path $repoRoot 'examples/notebooks/memristor-pulse-protocol.ipynb') `
        --working-directory $repoRoot `
        --output (Join-Path $artifactRoot 'memristor-pulse-protocol.executed.ipynb')
    if ($LASTEXITCODE -ne 0) { throw 'Headless memristor protocol notebook failed.' }
    & $python -m pip check
    if ($LASTEXITCODE -ne 0) { throw 'Python dependency check failed.' }
}
finally {
    $env:KESSETSU_TEST_CLI = $previousCli
    $env:KESSETSU_CLI = $previousAdapter
    $env:MPLBACKEND = $previousMatplotlibBackend
}

Write-Output 'Python adapter and headless notebook verifications passed.'
