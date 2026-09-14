param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("windows-x86_64", "linux-x86_64", "macos-x86_64", "macos-aarch64")]
    [string]$Target,
    [string]$Version = "",
    [string]$OutputDirectory = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$canonicalVersion = (Get-Content -LiteralPath (Join-Path $repoRoot "VERSION") -Raw).Trim()
if (-not $Version) { $Version = $canonicalVersion }
& (Join-Path $PSScriptRoot "verify-version.ps1") -ExpectedVersion $Version | Out-Null
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $repoRoot "release-artifacts" }
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
$repoFull = [System.IO.Path]::GetFullPath($repoRoot)
if (-not $outputRoot.StartsWith($repoFull, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Release output must remain inside the repository workspace: $outputRoot"
}

$isWindowsTarget = $Target -eq "windows-x86_64"
$binaryName = if ($isWindowsTarget) { "kess.exe" } else { "kess" }
$binary = Join-Path $repoRoot "core/target/release/$binaryName"
if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) { throw "Release binary is missing: $binary" }

$bundleName = "kessetsu-v$Version-$Target"
$stage = Join-Path $outputRoot $bundleName
if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
New-Item -ItemType Directory -Path $stage -Force | Out-Null
Copy-Item -LiteralPath $binary -Destination (Join-Path $stage $binaryName)
Copy-Item -LiteralPath (Join-Path $repoRoot "README.md") -Destination (Join-Path $stage "README.md")
Copy-Item -LiteralPath (Join-Path $repoRoot "CHANGELOG.md") -Destination (Join-Path $stage "CHANGELOG.md")
Copy-Item -LiteralPath (Join-Path $repoRoot "CONTRIBUTING.md") -Destination (Join-Path $stage "CONTRIBUTING.md")
Copy-Item -LiteralPath (Join-Path $repoRoot "SECURITY.md") -Destination (Join-Path $stage "SECURITY.md")
Copy-Item -LiteralPath (Join-Path $repoRoot "AGENTS.md") -Destination (Join-Path $stage "AGENTS.md")
Copy-Item -LiteralPath (Join-Path $repoRoot ".nvmrc") -Destination (Join-Path $stage ".nvmrc")
Copy-Item -LiteralPath (Join-Path $repoRoot "LICENSE") -Destination (Join-Path $stage "LICENSE")
Copy-Item -LiteralPath (Join-Path $repoRoot "NOTICE") -Destination (Join-Path $stage "NOTICE")
Copy-Item -LiteralPath (Join-Path $repoRoot "COMMERCIAL_LICENSE.md") -Destination (Join-Path $stage "COMMERCIAL_LICENSE.md")
Copy-Item -LiteralPath (Join-Path $repoRoot "docs/supported_domain.md") -Destination (Join-Path $stage "SUPPORTED_DOMAIN.md")
Copy-Item -LiteralPath (Join-Path $repoRoot "docs") -Destination (Join-Path $stage "docs") -Recurse
$examplesDirectory = Join-Path $stage "examples"
New-Item -ItemType Directory -Path $examplesDirectory | Out-Null
Copy-Item -Path (Join-Path $repoRoot "examples/*.kess") -Destination $examplesDirectory
$benchmarkDirectory = Join-Path $stage "core/tests/fixtures/benchmarks"
New-Item -ItemType Directory -Path $benchmarkDirectory -Force | Out-Null
Copy-Item -Path (Join-Path $repoRoot "core/tests/fixtures/benchmarks/*.kess") -Destination $benchmarkDirectory
$webDocumentationDirectory = Join-Path $stage "webapp"
New-Item -ItemType Directory -Path (Join-Path $webDocumentationDirectory "public") -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $repoRoot "webapp/README.md") -Destination (Join-Path $webDocumentationDirectory "README.md")
Copy-Item -LiteralPath (Join-Path $repoRoot "webapp/public/THIRD_PARTY_NOTICES.md") -Destination (Join-Path $webDocumentationDirectory "public/THIRD_PARTY_NOTICES.md")
$runtimeDocumentationDirectory = Join-Path $stage "core/tools/ngspice"
New-Item -ItemType Directory -Path $runtimeDocumentationDirectory -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $repoRoot "core/tools/ngspice/README.md") -Destination (Join-Path $runtimeDocumentationDirectory "README.md")

$brokenDocumentationLinks = @()
foreach ($markdown in Get-ChildItem -LiteralPath $stage -Filter "*.md" -File -Recurse) {
    $content = Get-Content -LiteralPath $markdown.FullName -Raw
    foreach ($match in [regex]::Matches($content, '!?\[[^\]]*\]\((?<target>[^)\s]+)(?:\s+"[^"]*")?\)')) {
        $linkTarget = $match.Groups['target'].Value.Trim('<', '>')
        if ($linkTarget -match '^(?:https?://|mailto:|#)') { continue }
        $pathOnly = ($linkTarget -split '#', 2)[0]
        if (-not $pathOnly) { continue }
        $resolved = [System.IO.Path]::GetFullPath((Join-Path $markdown.DirectoryName $pathOnly))
        if (-not (Test-Path -LiteralPath $resolved)) {
            $brokenDocumentationLinks += "$($markdown.FullName.Substring($stage.Length + 1)) -> $linkTarget"
        }
    }
}
if ($brokenDocumentationLinks.Count -gt 0) {
    throw "Release documentation has broken local links: $($brokenDocumentationLinks -join '; ')"
}
$noticeDirectory = Join-Path $stage "THIRD_PARTY_NOTICES"
New-Item -ItemType Directory -Path $noticeDirectory | Out-Null
Copy-Item -LiteralPath (Join-Path $repoRoot "core/assets/fonts/RobotoMono-OFL.txt") -Destination (Join-Path $noticeDirectory "RobotoMono-OFL.txt")
Copy-Item -LiteralPath (Join-Path $repoRoot "webapp/public/licenses/inter-OFL.txt") -Destination (Join-Path $noticeDirectory "Inter-OFL.txt")
$cargoMetadata = (& cargo metadata --manifest-path (Join-Path $repoRoot "core/Cargo.toml") --format-version 1 --locked | Out-String) | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw "cargo metadata failed while producing license inventory" }
$rustLicenses = [ordered]@{
    schema_version = "kessetsu.rust-licenses.v1"
    packages = @($cargoMetadata.packages | Where-Object { $_.name -ne "kessetsu-core" } | Sort-Object name, version | ForEach-Object {
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
    schema_version = "kessetsu.release.v1"
    version = $Version
    target = $Target
    executable = $binaryName
    executable_sha256 = $binaryHash
    simulator = [ordered]@{
        policy = $simulatorPolicy
        override = "KESSETSU_NGSPICE"
        version_probe = "ngspice -v"
    }
    generated_from = if ($env:GITHUB_SHA) { $env:GITHUB_SHA } else { (& git -C $repoRoot rev-parse HEAD).Trim() }
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $stage "release-manifest.json") -Encoding UTF8

$install = if ($isWindowsTarget) {
@"
Kessetsu $Version / $Target

1. Extract the complete archive to a normal folder. Keep kess.exe and tools together.
2. Open PowerShell in that folder and verify: .\kess.exe --version
3. Check the included example: .\kess.exe check .\examples\demo_circuit.kess
4. Run its assertions: .\kess.exe test .\examples\demo_circuit.kess

Simulation runtime: bundled tools\ngspice\bin\ngspice_con.exe (Ngspice 46); no separate simulator install is required.
The full upstream license inventory is under tools\ngspice\docs.
Override only with a trusted executable: `$env:KESSETSU_NGSPICE='C:\full\path\ngspice_con.exe'

Optional PATH setup: move this entire extracted folder to a permanent location, then add that folder (not the executable itself) to your user PATH. Open a new terminal and run: kess --version
Start with README.md and docs\tutorial.md. Supported engineering limits are in SUPPORTED_DOMAIN.md.
"@
} else {
@"
Kessetsu $Version / $Target

1. Extract the archive: tar -xzf kessetsu-v$Version-$Target.tar.gz
2. Enter the extracted directory and verify: ./kess --version
3. Install Ngspice, then verify it: ngspice -v
   Ubuntu/Debian: sudo apt-get install ngspice
   macOS/Homebrew: brew install ngspice
4. Check the included example: ./kess check ./examples/demo_circuit.kess
5. Run its assertions: ./kess test ./examples/demo_circuit.kess

Kessetsu discovers `ngspice` on PATH. Override only with a trusted full path via KESSETSU_NGSPICE.
The simulator executable and reported version are included in each simulation result.

Optional PATH setup: place `kess` in a directory already on PATH, or add this extracted directory to PATH. Then open a new terminal and run: kess --version
Start with README.md and docs/tutorial.md. Supported engineering limits are in SUPPORTED_DOMAIN.md.
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
