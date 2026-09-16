# Target-application verification only. Pin order comes from canonical Schematic IR,
# never a second component catalog or source-syntax parser.
function Get-EdaDeviceRecords {
    param([string]$Text)
    $records = @{}
    $inSubcircuit = $false
    foreach ($line in ($Text -split '\r?\n')) {
        $line = $line.Trim()
        if ($line -match '^\.subckt\b') { $inSubcircuit = $true; continue }
        if ($line -match '^\.ends\b') { $inSubcircuit = $false; continue }
        if ($line -match '^\.control\b') { break }
        if ($inSubcircuit -or $line -notmatch '^[RCLDQMVIX]\S*\s') { continue }
        $tokens = @($line -split '\s+')
        if ($records.ContainsKey($tokens[0])) { throw "Duplicate EDA device: $($tokens[0])" }
        $records[$tokens[0]] = $tokens
    }
    return $records
}

function Assert-EdaNetlists {
    param($Schematic, [xml]$KiCad, [string]$LtspiceText, [string]$SpiceText, [string]$LtspiceSchematicText)
    $canonical = Get-EdaDeviceRecords $SpiceText
    $ltDevices = Get-EdaDeviceRecords $LtspiceText
    $expectedRefs = @($Schematic.components.reference)
    $actualRefs = @($KiCad.export.components.comp | ForEach-Object { [string]$_.ref })
    if (@(Compare-Object $expectedRefs $actualRefs).Count -or $ltDevices.Count -ne $expectedRefs.Count) {
        throw 'EDA component set differs from canonical Schematic IR.'
    }
    $kiPins = @{}
    foreach ($net in $KiCad.export.nets.net) {
        foreach ($node in $net.node) {
            $key = "$($node.ref):$($node.pin)"
            if ($kiPins.ContainsKey($key)) { throw "Duplicate KiCad pin: $key" }
            $kiPins[$key] = [string]$net.code
        }
    }
    # Compare partitions, not generated net names. Detect both merged and split nets,
    # and anchor LTspice ground to node 0.
    $kiForward = @{}; $kiReverse = @{}
    $ltForward = @{'0' = '0'}; $ltReverse = @{'0' = '0'}
    $pinCount = 0
    foreach ($component in $Schematic.components) {
        $ref = $component.reference
        $kiComponent = @($KiCad.export.components.comp | Where-Object ref -eq $ref)[0]
        $expectedValue = if ($null -ne $component.value) { $component.value } else { $component.model }
        if ([string]$kiComponent.value -cne [string]$expectedValue) { throw "KiCad value/model changed: $ref" }
        if ($component.model) {
            $modelField = @($kiComponent.fields.field | Where-Object name -eq 'Kessetsu_Model')
            if ($modelField.Count -ne 1 -or $modelField[0].InnerText -cne $component.model) {
                throw "KiCad model identity changed: $ref"
            }
        }
        $kiLibrary = @($KiCad.export.libparts.libpart | Where-Object {
            $_.lib -eq $kiComponent.libsource.lib -and $_.part -eq $kiComponent.libsource.part
        })[0]
        $canonicalKey = @($canonical.Keys | Where-Object { $_.Substring(2) -ceq $ref })
        if ($canonicalKey.Count -ne 1) { throw "Missing canonical SPICE device: $ref" }
        $prefix = $canonicalKey[0].Substring(0, 1)
        $ltKey = if ($ref.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
            $ref
        } elseif ($prefix -eq 'V' -or $prefix -eq 'I') {
            "${prefix}_$ref"
        } else { "$prefix$([char]0xA7)$ref" }
        if (-not $ltDevices.ContainsKey($ltKey)) { throw "LTspice lost component: $ref" }
        $ltRecord = $ltDevices[$ltKey]
        $pins = @($component.pins)
        $tailStart = 1 + $pins.Count
        # LTspice's BJT symbol emits an optional substrate node, absent from
        # the three-pin catalog; it must remain grounded.
        if ($component.symbol -eq 'bjt' -and $ltRecord.Count -eq $pins.Count + 3) {
            if ($ltRecord[$tailStart] -ne '0') { throw "LTspice BJT substrate changed: $ref" }
            $tailStart++
        }
        $expectedRecord = $canonical[$canonicalKey[0]]
        $expectedTail = ($expectedRecord[(1 + $pins.Count)..($expectedRecord.Count - 1)] -join ' ')
        $actualTail = ($ltRecord[$tailStart..($ltRecord.Count - 1)] -join ' ')
        if ($actualTail -ine $expectedTail) { throw "LTspice value/model/stimulus changed: $ref" }
        for ($index = 0; $index -lt $pins.Count; $index++) {
            $pin = $pins[$index]
            $number = [string]($index + 1)
            $libraryPin = @($kiLibrary.pins.pin | Where-Object num -eq $number)
            if ($libraryPin.Count -ne 1 -or $libraryPin[0].name -cne $pin.name) {
                throw "KiCad pin identity changed: $ref.$($pin.name)"
            }
            $key = "${ref}:$number"
            if (-not $kiPins.ContainsKey($key)) { throw "KiCad lost pin: $key" }
            $expectedNet = [string]$pin.net
            $kiNet = $kiPins[$key]; $ltNet = $ltRecord[1 + $index]
            foreach ($mapping in @(
                @{Label='KiCad'; Forward=$kiForward; Reverse=$kiReverse; Actual=$kiNet},
                @{Label='LTspice'; Forward=$ltForward; Reverse=$ltReverse; Actual=$ltNet}
            )) {
                if (($mapping.Forward.ContainsKey($expectedNet) -and $mapping.Forward[$expectedNet] -ne $mapping.Actual) -or
                    ($mapping.Reverse.ContainsKey($mapping.Actual) -and $mapping.Reverse[$mapping.Actual] -ne $expectedNet)) {
                    throw "$($mapping.Label) connectivity changed: $ref.$($pin.name)"
                }
                $mapping.Forward[$expectedNet] = $mapping.Actual
                $mapping.Reverse[$mapping.Actual] = $expectedNet
            }
            $pinCount++
        }
    }
    if ($kiPins.Count -ne $pinCount) { throw 'KiCad netlist has extra pins.' }
    $analyses = @([regex]::Matches($SpiceText, '(?m)^(?:ac|tran|op|dc)\b[^\r\n]*'))
    for ($index = 0; $index -lt $analyses.Count; $index++) {
        $command = $analyses[$index].Value
        if ($command -match '^dc ([IV])_(\S+) ') {
            $prefix = $Matches[1]; $ref = $Matches[2]
            $ltSource = if ($ref.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
                $ref
            } else { "${prefix}_$ref" }
            $command = $command -replace '^dc [IV]_\S+ ', "dc $ltSource "
        }
        $marker = if ($index -eq 0) { '!' } else { ';' }
        if ($LtspiceSchematicText -notmatch ("(?m)^TEXT [^\r\n]* " + [regex]::Escape("$marker.$command") + '\s*$')) {
            throw "LTspice schematic analysis changed: $command"
        }
        if ($index -eq 0 -and $LtspiceText -notmatch ("(?m)^\." + [regex]::Escape($command) + '\s*$')) {
            throw "LTspice active analysis changed: $command"
        }
    }
    if ($LtspiceText -notmatch '(?m)^\.end\s*$') { throw 'LTspice netlist is incomplete.' }
    Write-Host "EDA semantic comparison passed: $($expectedRefs.Count) components, $pinCount pins, values/models and analyses."
}
