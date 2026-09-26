[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('kessetsu', 'direct')]
    [string]$Arm,

    [Parameter(Mandatory = $true)]
    [ValidateRange(1, 3)]
    [int]$Attempt,

    [ValidateSet('U1', 'U2', 'U3', 'U4', 'U5', 'U6', 'M1')]
    [string]$Task = 'U1',

    [ValidateSet('original', 'u6-external-v1', 'agent-workflow-v2')]
    [string]$Protocol = 'original',

    [string]$Model = 'gpt-5.6-sol',

    [ValidateSet('low', 'medium', 'high')]
    [string]$ReasoningEffort = 'medium',

    [ValidateRange(1, 30)]
    [int]$TimeoutMinutes = 30,

    [ValidateRange(1, 60)]
    [int]$ToolCallLimit = 60,

    [string]$CodexBinary = '',

    [string]$U6ModelPath = $env:KESSETSU_U6_MODEL
)

$ErrorActionPreference = 'Stop'
$originalU6ModelEnv = $env:KESSETSU_U6_MODEL
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if ($Protocol -eq 'u6-external-v1' -and ($Task -ne 'U6' -or $Arm -ne 'kessetsu')) {
    throw 'u6-external-v1 is a Kessetsu-only U6 follow-up; use -Task U6 -Arm kessetsu.'
}
if ($Protocol -eq 'agent-workflow-v2' -and $Task -ne 'M1') {
    throw 'agent-workflow-v2 currently defines only task M1.'
}
if ($Task -eq 'M1' -and $Protocol -ne 'agent-workflow-v2') {
    throw 'Task M1 requires -Protocol agent-workflow-v2.'
}
$protocolPath = switch ($Protocol) {
    'u6-external-v1' { Join-Path $repoRoot 'scripts/evals/specs/unseen-design-u6-followup-v1.md' }
    'agent-workflow-v2' { Join-Path $repoRoot 'scripts/evals/specs/agent-workflow-v2.md' }
    default { Join-Path $repoRoot 'scripts/evals/specs/unseen-design-v1.md' }
}
$harnessPath = if ($Protocol -in @('u6-external-v1', 'agent-workflow-v2')) { $protocolPath } else {
    Join-Path $repoRoot 'scripts/evals/specs/agent-comparison-harness-v1.md'
}
$evidenceTask = if ($Protocol -eq 'u6-external-v1') { 'U6-followup-v1' } else { $Task }
$evidenceVersion = if ($Protocol -eq 'agent-workflow-v2') { 'agent-comparison-v2' } else { 'agent-comparison-v1' }
$evidenceRoot = Join-Path $repoRoot ".artifacts/$evidenceVersion/$evidenceTask/$Arm/attempt-$Attempt"

if (Test-Path -LiteralPath $evidenceRoot) {
    throw "Evidence directory already exists and will not be overwritten: $evidenceRoot"
}

if (-not $CodexBinary) {
    $command = Get-Command codex -ErrorAction SilentlyContinue
    if (-not $command) { throw 'Codex CLI was not found. Pass -CodexBinary explicitly.' }
    $CodexBinary = $command.Source
}
$CodexBinary = (Resolve-Path -LiteralPath $CodexBinary).Path

$kessBinary = Join-Path $repoRoot 'core/target/release/kess.exe'
$ngspiceRoot = Join-Path $repoRoot 'core/tools/ngspice'
$ngspiceBinary = Join-Path $ngspiceRoot 'bin/ngspice_con.exe'
$modelPath = switch ($Task) {
    'U2' { Join-Path $repoRoot 'scripts/evals/models/KESSETSU_OPAMP_V1.lib' }
    'U3' { Join-Path $repoRoot 'scripts/evals/models/2N3904.lib' }
    'U4' { Join-Path $repoRoot 'scripts/evals/models/IRF540.lib' }
    'U5' { Join-Path $repoRoot 'scripts/evals/models/KESSETSU_POWER_AMPLIFIER_V1.lib' }
    'U6' { if ($U6ModelPath) { (Resolve-Path -LiteralPath $U6ModelPath).Path } else { $null } }
    default { $null }
}
$evaluator = switch ($Task) {
    'U1' { Join-Path $repoRoot 'scripts/evals/passive-filter.mjs' }
    'U2' { Join-Path $repoRoot 'scripts/evals/active-filter.mjs' }
    'U3' { Join-Path $repoRoot 'scripts/evals/common-emitter.mjs' }
    'U4' { Join-Path $repoRoot 'scripts/evals/load-driver.mjs' }
    'U5' { Join-Path $repoRoot 'scripts/evals/power-amplifier.mjs' }
    'U6' { if ($Protocol -eq 'u6-external-v1') {
        Join-Path $repoRoot 'scripts/evals/manufacturer-opamp-followup.mjs'
    } else { Join-Path $repoRoot 'scripts/evals/manufacturer-opamp.mjs' } }
    'M1' { Join-Path $repoRoot 'scripts/evals/robust-divider.mjs' }
}
$expectedU6ModelHash = 'fc5b020e63346e511bd808bf41c856b0150b000bcf8a41fe00eeececb1f422a5'
if ($Task -eq 'U6' -and -not $modelPath) {
    throw 'U6 requires -U6ModelPath or KESSETSU_U6_MODEL pointing to the locally acquired official OPAx197.LIB.'
}
$requiredFiles = @($protocolPath, $harnessPath, $CodexBinary, $ngspiceBinary, $evaluator)
if ($modelPath) { $requiredFiles += $modelPath }
foreach ($required in $requiredFiles) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Required file is missing: $required" }
}
if ($Task -eq 'U6') {
    $actualU6ModelHash = (Get-FileHash -LiteralPath $modelPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualU6ModelHash -ne $expectedU6ModelHash) {
        throw "U6 model integrity error: expected $expectedU6ModelHash, got $actualU6ModelHash"
    }
}
if ($Arm -eq 'kessetsu' -and -not (Test-Path -LiteralPath $kessBinary -PathType Leaf)) {
    throw "Required Kessetsu binary is missing: $kessBinary"
}

function Get-Sha256([string]$Path) {
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Quote-ProcessArgument([string]$Value) {
    if ($Value -notmatch '[\s"]') { return $Value }
    return '"' + ($Value -replace '(\\*)"', '$1$1\"' -replace '(\\+)$', '$1$1') + '"'
}

function Read-HarnessSection([string]$Heading, [string]$NextHeading) {
    $text = Get-Content -LiteralPath $harnessPath -Raw
    $startMarker = "## $Heading"
    $start = $text.IndexOf($startMarker, [StringComparison]::Ordinal)
    if ($start -lt 0) { throw "Harness section not found: $Heading" }
    $start += $startMarker.Length
    if ($NextHeading) {
        $end = $text.IndexOf("## $NextHeading", $start, [StringComparison]::Ordinal)
        if ($end -lt 0) { throw "Harness section boundary not found: $NextHeading" }
    } else { $end = $text.Length }
    return $text.Substring($start, $end - $start).Trim()
}

$commonHeading = if ($Protocol -in @('u6-external-v1', 'agent-workflow-v2')) { 'Common prompt' } elseif ($Task -eq 'U1') { 'Common U1 prompt' } else { "Common $Task prompt" }
$commonEnd = if ($Protocol -in @('u6-external-v1', 'agent-workflow-v2')) { 'Kessetsu tool reference' } elseif ($Task -eq 'U1') { 'Kessetsu arm tool reference' } else { "$Task Kessetsu arm tool reference" }
$commonPrompt = Read-HarnessSection $commonHeading $commonEnd
$commonPrompt = $commonPrompt.Trim().TrimStart('>').Trim()
if ($Protocol -eq 'u6-external-v1') {
    $armHeading = 'Kessetsu tool reference'
    $nextHeading = 'Evidence retained per attempt'
} elseif ($Protocol -eq 'agent-workflow-v2') {
    $armHeading = if ($Arm -eq 'kessetsu') { 'Kessetsu tool reference' } else { 'Direct tool reference' }
    $nextHeading = if ($Arm -eq 'kessetsu') { 'Direct tool reference' } else { 'Scoring and evidence' }
} elseif ($Task -eq 'U1') {
    $armHeading = if ($Arm -eq 'kessetsu') { 'Kessetsu arm tool reference' } else { 'Direct arm tool reference' }
    $nextHeading = if ($Arm -eq 'kessetsu') { 'Direct arm tool reference' } else { 'Common U2 prompt' }
} else {
    $armHeading = if ($Arm -eq 'kessetsu') { "$Task Kessetsu arm tool reference" } else { "$Task Direct arm tool reference" }
    $nextHeading = if ($Arm -eq 'kessetsu') { "$Task Direct arm tool reference" } elseif ($Task -eq 'U2') { 'Common U3 prompt' } elseif ($Task -eq 'U3') { 'Common U4 prompt' } elseif ($Task -eq 'U4') { 'Common U5 prompt' } elseif ($Task -eq 'U5') { 'Common U6 prompt' } else { 'Evidence retained per attempt' }
}
$toolReference = Read-HarnessSection $armHeading $nextHeading
$prompt = @"
You are attempt $Attempt of an audited circuit-design comparison. This is a fresh-context run. The hard limit is $ToolCallLimit tool calls and $TimeoutMinutes minutes. Do not use network access or inspect anything outside the supplied workspace.

$commonPrompt

Your supplied workflow:
$toolReference
"@

$runToken = [Guid]::NewGuid().ToString('N')
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "kessetsu-agent-eval-$runToken"
$workspace = Join-Path $tempRoot 'workspace'
New-Item -ItemType Directory -Path $workspace -Force | Out-Null
New-Item -ItemType Directory -Path $evidenceRoot -Force | Out-Null

try {
    New-Item -ItemType Directory -Path (Join-Path $workspace 'tools') -Force | Out-Null
    Copy-Item -LiteralPath $ngspiceRoot -Destination (Join-Path $workspace 'tools/ngspice') -Recurse
    if ($Arm -eq 'kessetsu') { Copy-Item -LiteralPath $kessBinary -Destination (Join-Path $workspace 'kess.exe') }
    if ($modelPath) {
        New-Item -ItemType Directory -Path (Join-Path $workspace 'models') -Force | Out-Null
        $workspaceModelPath = Join-Path (Join-Path $workspace 'models') (Split-Path -Leaf $modelPath)
        Copy-Item -LiteralPath $modelPath -Destination $workspaceModelPath
    }
    if ($Task -eq 'U6') { $env:KESSETSU_U6_MODEL = $workspaceModelPath }
    Set-Content -LiteralPath (Join-Path $workspace 'PROMPT.md') -Value $prompt -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $workspace 'AGENTS.md') -Encoding UTF8 -Value @"
# Isolated circuit-design attempt

- Work only inside this directory. Do not inspect parent or unrelated filesystem paths.
- Do not use network access, APIs, prior attempts, repository examples, or evaluator implementation.
- Follow PROMPT.md exactly. The immutable requirements cannot be edited.
- Keep materially different candidate revisions under revisions/.
- Stop before exceeding $ToolCallLimit tool calls.
"@

    $tracePath = Join-Path $evidenceRoot 'codex-events.jsonl'
    $stderrPath = Join-Path $evidenceRoot 'codex-stderr.log'
    $lastMessagePath = Join-Path $evidenceRoot 'codex-final.txt'
    $startUtc = [DateTime]::UtcNow

    $arguments = @(
        'exec', '--approve-for-me', '--ephemeral', '--json', '--ignore-user-config',
        '-m', $Model,
        '-c', "model_reasoning_effort='$ReasoningEffort'",
        '-C', $workspace, '--skip-git-repo-check',
        '-o', $lastMessagePath, '-'
    )
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $CodexBinary
    $psi.Arguments = (($arguments | ForEach-Object { Quote-ProcessArgument $_ }) -join ' ')
    $psi.WorkingDirectory = $workspace
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $psi
    if (-not $process.Start()) { throw 'Codex CLI process did not start.' }
    $promptBytes = (New-Object System.Text.UTF8Encoding($false)).GetBytes($prompt)
    $process.StandardInput.BaseStream.Write($promptBytes, 0, $promptBytes.Length)
    $process.StandardInput.Close()

    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $completed = $process.WaitForExit($TimeoutMinutes * 60 * 1000)
    $timedOut = -not $completed
    if ($timedOut) {
        $process.Kill()
        $process.WaitForExit()
    }
    $stdout = $stdoutTask.Result
    $stderr = $stderrTask.Result
    Set-Content -LiteralPath $tracePath -Value $stdout -Encoding UTF8
    Set-Content -LiteralPath $stderrPath -Value $stderr -Encoding UTF8
    $exitCode = if ($timedOut) { $null } else { $process.ExitCode }
    $endUtc = [DateTime]::UtcNow

    $workspaceEvidence = Join-Path $evidenceRoot 'workspace'
    New-Item -ItemType Directory -Path $workspaceEvidence -Force | Out-Null
    Get-ChildItem -LiteralPath $workspace -Force | Where-Object { $_.Name -notin @('tools', 'kess.exe') } | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $workspaceEvidence -Recurse -Force
    }

    $candidateName = if ($Arm -eq 'kessetsu') { 'candidate.kess' } else { 'candidate.cir' }
    $candidatePath = Join-Path $workspace $candidateName
    $evaluationPath = Join-Path $evidenceRoot 'evaluation.json'
    $evaluationExitCode = $null
    if (Test-Path -LiteralPath $candidatePath -PathType Leaf) {
        $node = (Get-Command node -ErrorAction Stop).Source
        $evaluationOutput = & $node $evaluator $Arm $candidatePath
        $evaluationExitCode = $LASTEXITCODE
        Set-Content -LiteralPath $evaluationPath -Value $evaluationOutput -Encoding UTF8
    }

    $completedToolCallCount = 0
    $reportedUsage = $null
    foreach ($line in ($stdout -split "`r?`n")) {
        if (-not $line.Trim()) { continue }
        try {
            $event = $line | ConvertFrom-Json
            if ($event.type -eq 'item.completed' -and $event.item.type -in @('command_execution', 'mcp_tool_call', 'file_change')) {
                $completedToolCallCount++
            }
            if ($event.type -eq 'turn.completed' -and $event.usage) {
                $reportedUsage = $event.usage
            }
        } catch { }
    }
    $rejectedToolCallCount = ([regex]::Matches($stderr, 'tools::router: error=')).Count
    $toolCallCount = $completedToolCallCount + $rejectedToolCallCount

    $metadata = [ordered]@{
        schema_version = 'kessetsu.agent-attempt.v2'
        task = $evidenceTask
        protocol = $Protocol
        arm = $Arm
        attempt = $Attempt
        model = $Model
        reasoning_effort = $ReasoningEffort
        authentication = 'chatgpt'
        api_key_used = $false
        network_search_enabled = $false
        started_at_utc = $startUtc.ToString('o')
        ended_at_utc = $endUtc.ToString('o')
        elapsed_seconds = [Math]::Round(($endUtc - $startUtc).TotalSeconds, 3)
        timeout_minutes = $TimeoutMinutes
        timed_out = $timedOut
        tool_call_limit = $ToolCallLimit
        observed_tool_calls = $toolCallCount
        human_interventions = 0
        codex_exit_code = $exitCode
        evaluator_exit_code = $evaluationExitCode
        candidate_present = (Test-Path -LiteralPath $candidatePath -PathType Leaf)
        reported_token_usage = $reportedUsage
        reported_cost = $null
        prompt_sha256 = (Get-FileHash -LiteralPath (Join-Path $workspace 'PROMPT.md') -Algorithm SHA256).Hash.ToLowerInvariant()
        spec_sha256 = Get-Sha256 $protocolPath
        codex = [ordered]@{ version = (& $CodexBinary --version | Out-String).Trim(); sha256 = Get-Sha256 $CodexBinary }
        kessetsu = if ($Arm -eq 'kessetsu') { [ordered]@{ version = (& $kessBinary --version | Out-String).Trim(); sha256 = Get-Sha256 $kessBinary } } else { $null }
        ngspice = [ordered]@{ version = '46'; sha256 = Get-Sha256 $ngspiceBinary }
        evaluator = [ordered]@{ path = (Resolve-Path -LiteralPath $evaluator).Path; sha256 = Get-Sha256 $evaluator }
        circuit_model = if ($modelPath) { [ordered]@{ name = [System.IO.Path]::GetFileNameWithoutExtension($modelPath); sha256 = Get-Sha256 $modelPath } } else { $null }
        trace_sha256 = Get-Sha256 $tracePath
        candidate_sha256 = if (Test-Path -LiteralPath $candidatePath -PathType Leaf) { Get-Sha256 $candidatePath } else { $null }
    }
    $metadata | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $evidenceRoot 'attempt.json') -Encoding UTF8

    Write-Host "Evidence: $evidenceRoot"
    Write-Host "Codex exit: $exitCode; evaluator exit: $evaluationExitCode; observed tool calls: $toolCallCount"
    if ($timedOut) { exit 124 }
    if ($exitCode -ne 0) { exit $exitCode }
    if ($evaluationExitCode -ne 0) { exit 10 }
} finally {
    if ($Task -eq 'U6') {
        if ($originalU6ModelEnv) { $env:KESSETSU_U6_MODEL = $originalU6ModelEnv }
        else { Remove-Item Env:KESSETSU_U6_MODEL -ErrorAction SilentlyContinue }
    }
    if (Test-Path -LiteralPath $tempRoot) {
        $resolvedTemp = (Resolve-Path -LiteralPath $tempRoot).Path
        $systemTemp = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\')
        if (-not $resolvedTemp.StartsWith($systemTemp + '\kessetsu-agent-eval-', [StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to remove unexpected temporary path: $resolvedTemp"
        }
        Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
    }
}
