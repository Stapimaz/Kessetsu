param(
    [switch]$SkipNpmInstall
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$corePath = Join-Path $repoRoot 'core'
$webPath = Join-Path $repoRoot 'webapp'
$npmCommand = if ($env:OS -eq 'Windows_NT') { 'npm.cmd' } else { 'npm' }

function Invoke-NativeStep {
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

foreach ($requiredCommand in @('cargo', 'wasm-pack', $npmCommand)) {
    if (-not (Get-Command $requiredCommand -ErrorAction SilentlyContinue)) {
        throw "Required command is not available: $requiredCommand"
    }
}

Push-Location $corePath
try {
    Invoke-NativeStep 'Rust format check' { cargo fmt -- --check }
    Invoke-NativeStep 'Rust Clippy' { cargo clippy --all-targets -- -D warnings }
    Invoke-NativeStep 'Rust tests' { cargo test }
    Invoke-NativeStep 'Rust release build' { cargo build --release }
    Invoke-NativeStep 'WASM package build' { wasm-pack build . --target web --out-dir pkg --release }
}
finally {
    Pop-Location
}

Push-Location $webPath
try {
    if (-not $SkipNpmInstall) {
        Invoke-NativeStep 'Web dependency install' { & $npmCommand ci }
    }
    Invoke-NativeStep 'Web lint' { & $npmCommand run lint }
    Invoke-NativeStep 'Web production build' { & $npmCommand run build:web }
}
finally {
    Pop-Location
}

Write-Host 'All verification steps passed.'
