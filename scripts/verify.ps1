param(
    [switch]$SkipNpmInstall
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$corePath = Join-Path $repoRoot 'core'
$webPath = Join-Path $repoRoot 'webapp'
$npmCommand = if ($env:OS -eq 'Windows_NT') { 'npm.cmd' } else { 'npm' }
$releaseVersion = (Get-Content -LiteralPath (Join-Path $repoRoot 'VERSION') -Raw).Trim()

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

foreach ($requiredCommand in @('cargo', 'git', 'python', 'wasm-pack', $npmCommand)) {
    if (-not (Get-Command $requiredCommand -ErrorAction SilentlyContinue)) {
        throw "Required command is not available: $requiredCommand"
    }
}

$worktreeBefore = Get-WorktreeSnapshot

Invoke-NativeStep 'Product version consistency' { & (Join-Path $repoRoot 'scripts/verify-version.ps1') | Out-Null }
Invoke-NativeStep 'Version-scoped release notes' {
    & (Join-Path $repoRoot 'scripts/extract-release-notes.ps1') -Version $releaseVersion -Output (Join-Path $repoRoot "release-artifacts/release-notes-v$releaseVersion.md")
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

$cliName = if ($env:OS -eq 'Windows_NT') { 'kess.exe' } else { 'kess' }
Invoke-NativeStep 'Python adapter and headless research notebook' {
    & (Join-Path $repoRoot 'scripts/verify-python.ps1') -CliPath (Join-Path $corePath "target/release/$cliName")
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
Invoke-NativeStep 'Independent U1 evaluator contracts' { node --test (Join-Path $repoRoot 'scripts/evals/passive-filter.test.mjs') }
Invoke-NativeStep 'Independent U2 evaluator contracts' { node --test (Join-Path $repoRoot 'scripts/evals/active-filter.test.mjs') }
Invoke-NativeStep 'Independent U3 evaluator contracts' { node --test (Join-Path $repoRoot 'scripts/evals/common-emitter.test.mjs') }
Invoke-NativeStep 'Independent U4 evaluator contracts' { node --test (Join-Path $repoRoot 'scripts/evals/load-driver.test.mjs') }
Invoke-NativeStep 'Independent U5 evaluator contracts' { node --test (Join-Path $repoRoot 'scripts/evals/power-amplifier.test.mjs') }
Invoke-NativeStep 'Independent U6 evaluator contracts' { node --test (Join-Path $repoRoot 'scripts/evals/manufacturer-opamp.test.mjs') }
Invoke-NativeStep 'Evaluator verification-scope contracts' { node --test (Join-Path $repoRoot 'scripts/evals/verification-scope.test.mjs') }

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
Invoke-NativeStep 'Package host release artifact' { & (Join-Path $repoRoot 'scripts/package-release.ps1') -Target $releaseTarget -Version $releaseVersion }
$releaseArchive = Get-ChildItem (Join-Path $repoRoot 'release-artifacts') -File |
    Where-Object { $_.Name -like "kessetsu-v$releaseVersion-$releaseTarget.*" -and $_.Name -notlike '*.sha256' } |
    Select-Object -First 1 -ExpandProperty FullName
Invoke-NativeStep 'Clean release artifact simulation smoke' { & (Join-Path $repoRoot 'scripts/smoke-release.ps1') -Archive $releaseArchive }
if ($env:OS -eq 'Windows_NT') {
    Invoke-NativeStep 'Windows installer contracts (isolated, no user PATH writes)' { & (Join-Path $repoRoot 'scripts/test-installer.ps1') -Archive $releaseArchive }
} else {
    Invoke-NativeStep 'POSIX installer contracts' { node --test (Join-Path $repoRoot 'scripts/installer.test.mjs') }
}
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
