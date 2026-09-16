param([Parameter(Mandatory = $true)][string]$Archive)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$installer = Join-Path $repoRoot 'webapp/public/install.ps1'
$parseErrors = $null
$tokens = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($installer, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors) { throw ($parseErrors | Out-String) }
# Load only functions; never run the installer's default entry point on the developer's account.
foreach ($definition in $ast.FindAll({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] }, $false)) {
    . ([scriptblock]::Create($definition.Extent.Text))
}
function Expect-Failure([scriptblock]$Action, [string]$Message) {
    $failed = $false
    try { & $Action } catch { if ($_.Exception.Message -notlike "*$Message*") { throw }; $failed = $true }
    if (-not $failed) { throw "Expected failure: $Message" }
}
Expect-Failure { Get-KessetsuPlatform -System Windows_NT -Architecture Arm64 } 'x86-64'
Expect-Failure { Get-KessetsuPlatform -System Linux -Architecture X64 } 'install.sh'
if ((Merge-KessetsuPath 'C:\Existing' 'C:\Kessetsu\bin') -ne 'C:\Existing;C:\Kessetsu\bin') { throw 'PATH append failed' }
if ((Merge-KessetsuPath 'C:\Existing;C:\Kessetsu\bin' 'c:\kessetsu\bin') -ne 'C:\Existing;C:\Kessetsu\bin') { throw 'PATH idempotence failed' }
Expect-Failure { Get-KessetsuRelease '../../bad' } 'Version must'
$fixtureArchive = [IO.Path]::GetFullPath($Archive)
$fixtureName = Split-Path -Leaf $fixtureArchive
if ($fixtureName -notmatch '^kessetsu-v([0-9]+\.[0-9]+\.[0-9]+)-windows-x86_64.zip$') { throw 'Use a Windows release fixture archive' }
$fixtureVersion = $Matches[1]
$testRoot = Join-Path $repoRoot ('.artifacts/installer-contracts-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $testRoot | Out-Null
$corrupt = $false
$pathCalls = 0
function Get-KessetsuRelease([string]$RequestedVersion) {
    return [pscustomobject]@{ tag_name = "v$fixtureVersion"; draft = $false; prerelease = $false; assets = @(
        [pscustomobject]@{ name = $fixtureName; browser_download_url = "https://github.com/Stapimaz/Kessetsu/releases/download/v$fixtureVersion/$fixtureName" },
        [pscustomobject]@{ name = "$fixtureName.sha256"; browser_download_url = "https://github.com/Stapimaz/Kessetsu/releases/download/v$fixtureVersion/$fixtureName.sha256" }
    ) }
}
function Receive-KessetsuFile([string]$Url, [string]$Destination) {
    if ($Url.EndsWith('.sha256')) {
        $hash = if ($corrupt) { '0' * 64 } else { (Get-FileHash -LiteralPath $fixtureArchive -Algorithm SHA256).Hash.ToLowerInvariant() }
        Set-Content -LiteralPath $Destination -Value "$hash  $fixtureName" -Encoding ascii
    } else { Copy-Item -LiteralPath $fixtureArchive -Destination $Destination }
}
function Set-KessetsuPath([string]$BinDir) { $script:pathCalls++ }
$managed = Join-Path $testRoot 'managed'
Install-Kessetsu -RequestedVersion $fixtureVersion -Root $managed
$launcher = Join-Path $managed 'bin/kess.cmd'
$initial = Get-Content -LiteralPath $launcher -Raw
Install-Kessetsu -RequestedVersion $fixtureVersion -Root $managed
$updated = Get-Content -LiteralPath $launcher -Raw
if ($initial -eq $updated -or $pathCalls -ne 2) { throw 'Update or PATH adapter was not exercised' }
$corrupt = $true
Expect-Failure { Install-Kessetsu -RequestedVersion $fixtureVersion -Root $managed } 'checksum mismatch'
if ((Get-Content -LiteralPath $launcher -Raw) -ne $updated) { throw 'Corruption replaced the working launcher' }
$unmanaged = Join-Path $testRoot 'unmanaged'
New-Item -ItemType Directory -Path $unmanaged | Out-Null
Set-Content -LiteralPath (Join-Path $unmanaged 'keep.txt') -Value 'keep'
Expect-Failure { Install-Kessetsu -Root $unmanaged } 'unmanaged directory'
if ((Get-Content -LiteralPath (Join-Path $unmanaged 'keep.txt')).Trim() -ne 'keep') { throw 'Unmanaged data changed' }
Write-Host 'Installer contracts PASS: platform/version validation, PATH merge/idempotence, install, update, corrupt-download preservation, and unmanaged-directory refusal. User PATH was not modified.'
