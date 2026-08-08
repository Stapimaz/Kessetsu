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

function Get-WorktreeSnapshot {
    $statusLines = & git -C $repoRoot status --porcelain=v1 --untracked-files=all
    if ($LASTEXITCODE -ne 0) {
        throw "Could not read Git worktree status (exit code $LASTEXITCODE)"
    }
    return $statusLines -join "`n"
}

foreach ($requiredCommand in @('cargo', 'git', 'wasm-pack', $npmCommand)) {
    if (-not (Get-Command $requiredCommand -ErrorAction SilentlyContinue)) {
        throw "Required command is not available: $requiredCommand"
    }
}

$worktreeBefore = Get-WorktreeSnapshot

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
    Invoke-NativeStep 'Web production dependency audit' { & $npmCommand audit --omit=dev }
    Invoke-NativeStep 'Web lint' { & $npmCommand run lint }
    Invoke-NativeStep 'Web production build' { & $npmCommand run build:web }
}
finally {
    Pop-Location
}

$worktreeAfter = Get-WorktreeSnapshot
if ($worktreeAfter -ne $worktreeBefore) {
    Write-Host 'Worktree before verification:'
    Write-Host $worktreeBefore
    Write-Host 'Worktree after verification:'
    Write-Host $worktreeAfter
    throw 'Verification generated or modified non-ignored worktree files.'
}

Write-Host 'All verification steps passed.'
