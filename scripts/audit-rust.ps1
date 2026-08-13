$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoLock = Join-Path $repoRoot "core/Cargo.lock"
$auditCommand = Get-Command cargo-audit -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Source

if (-not $auditCommand -and ($env:OS -eq "Windows_NT")) {
    $version = "0.22.2"
    $expectedSha256 = "0a7316540862c13d954f648917ceacca593747baed6eec180fafa590be2710ab"
    $cacheRoot = Join-Path ([System.IO.Path]::GetTempPath()) "netlang-cargo-audit-$version"
    $archive = Join-Path $cacheRoot "cargo-audit-x86_64-pc-windows-msvc-v$version.zip"
    $extractRoot = Join-Path $cacheRoot "bin"
    $download = "https://github.com/rustsec/rustsec/releases/download/cargo-audit%2Fv$version/cargo-audit-x86_64-pc-windows-msvc-v$version.zip"
    New-Item -ItemType Directory -Path $cacheRoot -Force | Out-Null
    if (-not (Test-Path -LiteralPath $archive -PathType Leaf)) {
        Invoke-WebRequest -Uri $download -OutFile $archive
    }
    $actual = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expectedSha256) { throw "cargo-audit release asset checksum mismatch" }
    if (-not (Test-Path -LiteralPath $extractRoot -PathType Container)) {
        Expand-Archive -LiteralPath $archive -DestinationPath $extractRoot
    }
    $auditCommand = Get-ChildItem -LiteralPath $extractRoot -Recurse -Filter cargo-audit.exe |
        Select-Object -First 1 -ExpandProperty FullName
}

if (-not $auditCommand) {
    throw "cargo-audit 0.22.2 is required. Install it with: cargo install cargo-audit --version 0.22.2 --locked"
}
& $auditCommand audit --file $cargoLock
if ($LASTEXITCODE -ne 0) { throw "RustSec audit failed with exit code $LASTEXITCODE" }
Write-Host "RustSec vulnerability audit PASS. Informational warnings remain visible above and are tracked in docs/security_audit.md."
