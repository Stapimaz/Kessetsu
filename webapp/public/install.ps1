param([string]$Version = 'latest', [string]$InstallDir = '', [switch]$NoPath)

# Kessetsu per-user installer. Source: github.com/Stapimaz/Kessetsu.
# No elevation, execution-policy changes, telemetry, or simulator downloads.
function Get-KessetsuPlatform([string]$System = $env:OS, [string]$Architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()) {
    if ($System -ne 'Windows_NT') { throw 'Use https://kessetsu.com/install.sh on macOS or Linux.' }
    if ($Architecture -ne 'X64') {
        throw 'Windows installation currently supports x86-64 only. Use the Web Hub on other architectures.'
    }
    return 'windows-x86_64'
}

function Get-KessetsuRelease([string]$RequestedVersion) {
    $endpoint = 'https://api.github.com/repos/Stapimaz/Kessetsu/releases/'
    if ($RequestedVersion -eq 'latest') { $endpoint += 'latest' }
    elseif ($RequestedVersion -match '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') {
        $endpoint += 'tags/v' + $RequestedVersion
    } else { throw 'Version must be latest or a stable version such as 1.0.0.' }
    return Invoke-RestMethod -Uri $endpoint -Headers @{ 'User-Agent' = 'Kessetsu-Installer' } -TimeoutSec 60
}

function Receive-KessetsuFile([string]$Url, [string]$Destination) {
    Invoke-WebRequest -Uri $Url -OutFile $Destination -UseBasicParsing -TimeoutSec 180
}

function Merge-KessetsuPath([string]$ExistingPath, [string]$BinDir) {
    $entries = @($ExistingPath -split ';' | Where-Object { $_ })
    if ($entries -notcontains $BinDir) { $entries += $BinDir }
    return $entries -join ';'
}

function Set-KessetsuPath([string]$BinDir) {
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $updated = Merge-KessetsuPath $userPath $BinDir
    if ($updated -cne $userPath) { [Environment]::SetEnvironmentVariable('Path', $updated, 'User') }
    if (@($env:Path -split ';') -notcontains $BinDir) { $env:Path += ';' + $BinDir }
}

function Install-Kessetsu([string]$RequestedVersion = 'latest', [string]$Root = '', [switch]$SkipPath) {
    $ErrorActionPreference = 'Stop'
    $platform = Get-KessetsuPlatform
    if (-not $Root) {
        if (-not $env:LOCALAPPDATA) { throw 'LOCALAPPDATA is unavailable. Specify -InstallDir explicitly.' }
        $Root = Join-Path $env:LOCALAPPDATA 'Kessetsu'
    }
    $Root = [IO.Path]::GetFullPath($Root).TrimEnd('\', '/')
    foreach ($broadPath in @([IO.Path]::GetPathRoot($Root), $env:USERPROFILE, $env:LOCALAPPDATA)) {
        if ($broadPath -and $Root -eq $broadPath.TrimEnd('\', '/')) { throw 'Choose a dedicated Kessetsu install directory.' }
    }
    $marker = Join-Path $Root '.kessetsu-install'
    if (Test-Path -LiteralPath $Root) {
        if ((Get-Item -LiteralPath $Root).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Install directory must not be a link.' }
        if (-not (Test-Path -LiteralPath $marker) -or (Get-Content -LiteralPath $marker -Raw).Trim() -ne 'kessetsu.install.v1') {
            throw "Refusing to overwrite an unmanaged directory: $Root. Choose another -InstallDir."
        }
    } else {
        New-Item -ItemType Directory -Path $Root | Out-Null
        Set-Content -LiteralPath $marker -Value 'kessetsu.install.v1' -Encoding ascii
    }
    $lock = $null
    $scratch = $null
    try {
        $lock = [IO.File]::Open((Join-Path $Root '.install-lock'), 'OpenOrCreate', 'ReadWrite', 'None')
        Write-Host 'Finding the official Kessetsu release...'
        $release = Get-KessetsuRelease $RequestedVersion
        if ($release.draft -or $release.prerelease -or $release.tag_name -notmatch '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') {
            throw 'The release is not a published stable version.'
        }
        $resolvedVersion = $release.tag_name.Substring(1)
        if ($RequestedVersion -ne 'latest' -and $resolvedVersion -ne $RequestedVersion) { throw 'Release version mismatch.' }
        $bundle = "kessetsu-v$resolvedVersion-$platform"
        $archiveName = "$bundle.zip"
        $scratch = Join-Path ([IO.Path]::GetTempPath()) ('kessetsu-install-' + [Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $scratch | Out-Null
        foreach ($name in @($archiveName, "$archiveName.sha256")) {
            $assets = @($release.assets | Where-Object name -eq $name)
            $expectedUrl = "https://github.com/Stapimaz/Kessetsu/releases/download/v$resolvedVersion/$name"
            if ($assets.Count -ne 1 -or $assets[0].browser_download_url -cne $expectedUrl) { throw "Missing or unexpected release asset: $name" }
            Receive-KessetsuFile $expectedUrl (Join-Path $scratch $name)
        }
        $archive = Join-Path $scratch $archiveName
        $checksum = (Get-Content -LiteralPath "$archive.sha256" -Raw).Trim()
        if ($checksum -notmatch ('^([a-fA-F0-9]{64})\s+' + [regex]::Escape($archiveName) + '$')) { throw 'Invalid archive checksum record.' }
        $expectedHash = $Matches[1]
        if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $expectedHash) { throw 'Archive checksum mismatch; previous installation was not changed.' }
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $zip = [IO.Compression.ZipFile]::OpenRead($archive)
        try {
            foreach ($entry in $zip.Entries) {
                if (-not $entry.FullName.StartsWith($bundle + '/', [StringComparison]::Ordinal) -or
                    $entry.FullName -match '(^|/)\.\.(/|$)|\\|:') { throw 'Unsafe archive path.' }
            }
        } finally { $zip.Dispose() }
        Expand-Archive -LiteralPath $archive -DestinationPath (Join-Path $scratch 'unpacked')
        $stage = Join-Path $scratch "unpacked/$bundle"
        $manifest = Get-Content -LiteralPath (Join-Path $stage 'release-manifest.json') -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($manifest.schema_version -ne 'kessetsu.release.v1' -or $manifest.version -ne $resolvedVersion -or
            $manifest.target -ne $platform -or $manifest.executable -ne 'kess.exe' -or $manifest.simulator.policy -ne 'bundled-ngspice-46') {
            throw 'Release manifest does not match the requested platform/version.'
        }
        $binary = Join-Path $stage 'kess.exe'
        if ((Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant() -cne $manifest.executable_sha256) { throw 'Executable checksum mismatch.' }
        if (-not (Test-Path -LiteralPath (Join-Path $stage 'tools/ngspice/bin/ngspice_con.exe'))) { throw 'Bundled simulator is missing.' }
        $probe = (& $binary --version | Out-String).Trim()
        if ($LASTEXITCODE -ne 0 -or $probe -cne "kess $resolvedVersion") { throw 'Installed CLI could not be verified.' }
        $releases = Join-Path $Root 'releases'
        $bin = Join-Path $Root 'bin'
        foreach ($dir in @($releases, $bin)) {
            if (Test-Path -LiteralPath $dir) {
                if ((Get-Item -LiteralPath $dir).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Managed directories must not be links.' }
            } else { New-Item -ItemType Directory -Path $dir | Out-Null }
        }
        $installId = "v$resolvedVersion-" + [Guid]::NewGuid().ToString('N')
        Move-Item -LiteralPath $stage -Destination (Join-Path $releases $installId)
        $launcher = Join-Path $bin 'kess.cmd'
        $pending = Join-Path $bin ('kess-' + [Guid]::NewGuid().ToString('N') + '.cmd')
        Set-Content -LiteralPath $pending -Encoding ascii -Value "@echo off`r`n`"%~dp0..\releases\$installId\kess.exe`" %*"
        if (-not $SkipPath) { Set-KessetsuPath $bin }
        if (Test-Path -LiteralPath $launcher) {
            [IO.File]::Replace($pending, $launcher, (Join-Path $bin ('previous-' + [Guid]::NewGuid().ToString('N') + '.cmd')))
        } else { [IO.File]::Move($pending, $launcher) }
        Write-Host "Installed Kessetsu $resolvedVersion. Ngspice is included."
        Write-Host "Location: $Root"
        if ($SkipPath) { Write-Host "Run: & '$launcher' --version" }
        else { Write-Host 'Open a NEW terminal (restart VS Code if needed), then run: kess --version' }
        Write-Host 'Update: run the same installation command again. Previous bundles are kept for recovery.'
    } finally {
        if ($lock) { $lock.Dispose() }
        if ($scratch) {
            $scratchFull = [IO.Path]::GetFullPath($scratch)
            $tempFull = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
            if ($scratchFull.StartsWith($tempFull, [StringComparison]::OrdinalIgnoreCase) -and
                [IO.Path]::GetFileName($scratchFull) -match '^kessetsu-install-[a-f0-9]{32}$') {
                Remove-Item -LiteralPath $scratchFull -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }
}

try { Install-Kessetsu -RequestedVersion $Version -Root $InstallDir -SkipPath:$NoPath }
catch { throw "Kessetsu installation failed: $($_.Exception.Message) See https://kessetsu.com/install/ for help." }
