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
    Invoke-NativeStep 'Web lint' { & $npmCommand run lint }
    Invoke-NativeStep 'Web unit tests' { & $npmCommand run test:unit }
    Invoke-NativeStep 'Web production build' { & $npmCommand run build:web }
    Invoke-NativeStep 'Web runtime integrity and notices' { & $npmCommand run 'verify:runtime' }
    Invoke-NativeStep 'Web deployment paths, CSP, and license bundle' { & $npmCommand run 'verify:deployment' }
    Invoke-NativeStep 'Web browser E2E' { & $npmCommand run test:e2e }
}
finally {
    Pop-Location
}

Invoke-NativeStep 'Release dependency/license/artifact audit' { & (Join-Path $repoRoot 'scripts/audit-release.ps1') }
Invoke-NativeStep 'RustSec vulnerability audit' { & (Join-Path $repoRoot 'scripts/audit-rust.ps1') }
Invoke-NativeStep 'Replayable external-agent evaluation' { & (Join-Path $repoRoot 'scripts/replay-agent-eval.ps1') }

$architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
$releaseTarget = if ($env:OS -eq 'Windows_NT' -and $architecture -eq 'X64') {
    'windows-x86_64'
} elseif ($IsLinux -and $architecture -eq 'X64') {
    'linux-x86_64'
} elseif ($IsMacOS -and $architecture -eq 'X64') {
    'macos-x86_64'
} elseif ($IsMacOS -and $architecture -eq 'Arm64') {
    'macos-aarch64'
} else {
    throw "Canonical verification does not have a release target for this host: $architecture"
}
Invoke-NativeStep 'Package host release artifact' { & (Join-Path $repoRoot 'scripts/package-release.ps1') -Target $releaseTarget -Version '0.1.0' }
$releaseArchive = Get-ChildItem (Join-Path $repoRoot 'release-artifacts') -File |
    Where-Object { $_.Name -like "netlang-v0.1.0-$releaseTarget.*" -and $_.Name -notlike '*.sha256' } |
    Select-Object -First 1 -ExpandProperty FullName
Invoke-NativeStep 'Clean release artifact simulation smoke' { & (Join-Path $repoRoot 'scripts/smoke-release.ps1') -Archive $releaseArchive }
if ($env:OS -eq 'Windows_NT') {
    Invoke-NativeStep 'Installed EDA application smoke' { & (Join-Path $repoRoot 'scripts/verify-eda-exports.ps1') }
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
