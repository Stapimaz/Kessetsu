param([string]$RepositoryRoot = '')

$ErrorActionPreference = 'Stop'
if (-not $RepositoryRoot) { $RepositoryRoot = Split-Path -Parent $PSScriptRoot }
$sourceRoot = [System.IO.Path]::GetFullPath($RepositoryRoot)
$manifest = Get-Content -LiteralPath (Join-Path $sourceRoot 'docs/public-documents.json') -Raw | ConvertFrom-Json
$listed = @($manifest.files)
if ($manifest.schema_version -ne 'kessetsu.public-documents.v1' -or
    -not $listed.Count -or @($listed | Select-Object -Unique).Count -ne $listed.Count) {
    throw 'Invalid public-document manifest.'
}
foreach ($entry in $listed) {
    if ($entry -notmatch '^docs/[A-Za-z0-9_./-]+$' -or $entry -match '(?:^|/)\.{1,2}(?:/|$)') {
        throw 'Invalid public-document path.'
    }
    $absolute = [System.IO.Path]::GetFullPath((Join-Path $sourceRoot $entry))
    if (-not $absolute.StartsWith($sourceRoot + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw 'Public-document path escapes the source root.'
    }
    if (-not (Test-Path -LiteralPath $absolute -PathType Leaf)) { throw "Missing public document: $entry" }
    if ((Get-Item -LiteralPath $absolute).Attributes -band [System.IO.FileAttributes]::ReparsePoint) {
        throw "Public document must not be a link: $entry"
    }
}
foreach ($file in Get-ChildItem -LiteralPath (Join-Path $sourceRoot 'docs') -Recurse -File) {
    $relative = $file.FullName.Substring($sourceRoot.Length + 1).Replace('\', '/')
    if ($listed -notcontains $relative) { throw "Unreviewed document: $relative" }
}
$tracked = @(& git -C $sourceRoot ls-files)
if ($LASTEXITCODE -ne 0) { throw 'Cannot audit tracked public files.' }
$markdownCount = 0
$linkCount = 0
foreach ($entry in $tracked) {
    if ($entry -match '(?:^|/)(?:\.env(?:\..*)?|id_rsa|id_ed25519|credentials\.json)$' -or
        $entry -match '(?i)(?:^|/)(?:private|internal|Kessetsu-Private)(?:/|$)') {
        throw "Private-looking tracked path requires review: $entry"
    }
    $absolute = Join-Path $sourceRoot $entry
    if (-not $entry.EndsWith('.md') -or -not (Test-Path -LiteralPath $absolute -PathType Leaf)) { continue }
    $markdownCount++
    $text = Get-Content -LiteralPath $absolute -Raw -Encoding UTF8
    # Never echo suspected secrets or private-plan paths.
    if ($text -match '(?i)(?:C:[\\/]Users[\\/]|/Users/|/home/)[A-Za-z0-9_.-]+[\\/]' -or
        $text -match "(?i)owner(?:'s)?[^\r\n]{0,100}(?:quota|allowance|usage limit|ChatGPT plan)" -or
        $text -match '(?:ghp_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{60,}|sk-proj-[A-Za-z0-9_-]{40,}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----)') {
        throw "Publication-sensitive prose requires review: $entry"
    }
    foreach ($match in [regex]::Matches($text, '!?\[[^\]]*\]\((?<target>[^)\s]+)(?:\s+"[^"]*")?\)')) {
        $target = $match.Groups['target'].Value.Trim('<', '>')
        if ($target -match '^(?:https?://|mailto:|#)') { continue }
        $path = ($target -split '#', 2)[0]
        if (-not $path) { continue }
        $resolved = [System.IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $absolute) $path))
        if (-not (Test-Path -LiteralPath $resolved)) { throw "Broken link in ${entry}: $target" }
        $linkCount++
    }
}
Write-Output "Public-document audit passed: $($listed.Count) listed files, $markdownCount Markdown files, $linkCount local links."
