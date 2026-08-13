param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("windows-x86_64", "linux-x86_64", "macos-x86_64", "macos-aarch64")]
    [string]$Target,
    [string]$Version = "0.1.0",
    [string]$OutputDirectory = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $repoRoot "release-artifacts" }
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
$repoFull = [System.IO.Path]::GetFullPath($repoRoot)
if (-not $outputRoot.StartsWith($repoFull, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Release output must remain inside the repository workspace: $outputRoot"
}

$isWindowsTarget = $Target -eq "windows-x86_64"
$binaryName = if ($isWindowsTarget) { "netlang.exe" } else { "netlang" }
$binary = Join-Path $repoRoot "core/target/release/$binaryName"
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw "Release binary is missing: $binary" }

$bundleName = "netlang-v$Version-$Target"
$stage = Join-Path $outputRoot $bundleName
if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
New-Item -ItemType Directory -Path $stage -Force | Out-Null
Copy-Item -LiteralPath $binary -Destination (Join-Path $stage $binaryName)
Copy-Item -LiteralPath (Join-Path $repoRoot "README.md") -Destination (Join-Path $stage "README.md")
Copy-Item -LiteralPath (Join-Path $repoRoot "docs/supported_domain.md") -Destination (Join-Path $stage "SUPPORTED_DOMAIN.md")
$noticeDirectory = Join-Path $stage "THIRD_PARTY_NOTICES"
New-Item -ItemType Directory -Path $noticeDirectory | Out-Null
Copy-Item -LiteralPath (Join-Path $repoRoot "core/assets/fonts/RobotoMono-OFL.txt") -Destination (Join-Path $noticeDirectory "RobotoMono-OFL.txt")
$cargoMetadata = (& cargo metadata --manifest-path (Join-Path $repoRoot "core/Cargo.toml") --format-version 1 --locked | Out-String) | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw "cargo metadata failed while producing license inventory" }
$rustLicenses = [ordered]@{
    schema_version = "netlang.rust-licenses.v1"
    packages = @($cargoMetadata.packages | Where-Object { $_.name -ne "netlang-core" } | Sort-Object name, version | ForEach-Object {
        [ordered]@{ name = $_.name; version = $_.version; license = $_.license; repository = $_.repository }
    })
}
$rustLicenses | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $noticeDirectory "rust-inventory.json") -Encoding UTF8

$simulatorPolicy = "system-ngspice"
if ($isWindowsTarget) {
    $simulatorPolicy = "bundled-ngspice-46"
    $toolsDirectory = Join-Path $stage "tools"
    New-Item -ItemType Directory -Path $toolsDirectory | Out-Null
    Copy-Item -LiteralPath (Join-Path $repoRoot "core/tools/ngspice") -Destination (Join-Path $toolsDirectory "ngspice") -Recurse
}

$binaryHash = (Get-FileHash -LiteralPath (Join-Path $stage $binaryName) -Algorithm SHA256).Hash.ToLowerInvariant()
$manifest = [ordered]@{
    schema_version = "netlang.release.v1"
    version = $Version
    target = $Target
    executable = $binaryName
    executable_sha256 = $binaryHash
    simulator = [ordered]@{
        policy = $simulatorPolicy
        override = "NETLANG_NGSPICE"
        version_probe = "ngspice -v"
    }
    generated_from = if ($env:GITHUB_SHA) { $env:GITHUB_SHA } else { (& git -C $repoRoot rev-parse HEAD).Trim() }
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $stage "release-manifest.json") -Encoding UTF8

$install = if ($isWindowsTarget) {
@"
NetLang $Version / $Target

Run: .\netlang.exe --version
Simulation runtime: bundled tools\ngspice\bin\ngspice_con.exe (Ngspice 46).
The full upstream license inventory is under tools\ngspice\docs.
Override only with a trusted executable: `$env:NETLANG_NGSPICE='C:\full\path\ngspice_con.exe'
"@
} else {
@"
NetLang $Version / $Target

Run: ./netlang --version
Install Ngspice from the operating-system package manager, then verify: ngspice -v
NetLang discovers `ngspice` on PATH. Override only with a trusted full path via NETLANG_NGSPICE.
The simulator executable and reported version are included in each simulation result.
"@
}
$install | Set-Content -LiteralPath (Join-Path $stage "INSTALL.txt") -Encoding UTF8

New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null
if ($isWindowsTarget) {
    $archive = Join-Path $outputRoot "$bundleName.zip"
    if (Test-Path -LiteralPath $archive) { Remove-Item -LiteralPath $archive -Force }
    Compress-Archive -LiteralPath $stage -DestinationPath $archive -CompressionLevel Optimal
} else {
    $archive = Join-Path $outputRoot "$bundleName.tar.gz"
    if (Test-Path -LiteralPath $archive) { Remove-Item -LiteralPath $archive -Force }
    & tar -C $outputRoot -czf $archive $bundleName
    if ($LASTEXITCODE -ne 0) { throw "tar failed with exit code $LASTEXITCODE" }
}
$archiveHash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
"$archiveHash  $([System.IO.Path]::GetFileName($archive))" | Set-Content -LiteralPath "$archive.sha256" -Encoding ascii
Write-Host "Release artifact: $archive"
Write-Host "SHA-256: $archiveHash"
