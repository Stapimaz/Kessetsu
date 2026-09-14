param(
    [Parameter(Mandatory = $true)]
    [string]$Version,
    [Parameter(Mandatory = $true)]
    [string]$Output
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
& (Join-Path $PSScriptRoot "verify-version.ps1") -ExpectedVersion $Version | Out-Null
$changelog = Get-Content -LiteralPath (Join-Path $repoRoot "CHANGELOG.md") -Raw
$escaped = [regex]::Escape($Version)
$match = [regex]::Match($changelog, "(?ms)^## \[$escaped\][^\r\n]*\r?\n(?<body>.*?)(?=^## \[|\z)")
if (-not $match.Success) { throw "CHANGELOG.md has no release section for $Version" }
$notes = $match.Groups['body'].Value.Trim()
if (-not $notes) { throw "CHANGELOG.md release section for $Version is empty" }
$outputPath = if ([System.IO.Path]::IsPathRooted($Output)) {
    [System.IO.Path]::GetFullPath($Output)
} else {
    [System.IO.Path]::GetFullPath((Join-Path $repoRoot $Output))
}
$outputParent = Split-Path -Parent $outputPath
if ($outputParent) { New-Item -ItemType Directory -Path $outputParent -Force | Out-Null }
[System.IO.File]::WriteAllText($outputPath, "$notes`n", [System.Text.UTF8Encoding]::new($false))
Write-Host "Release notes for ${Version}: $outputPath"
