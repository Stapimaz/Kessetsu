# Tutorial: From Source to Verified Circuit

This tutorial builds a first-order RC low-pass, verifies its cutoff and exports the result. You can paste the same source into Web Hub or save it as `rc.kess` for the CLI.

```kessetsu
net GND
net IN
net OUT
source VIN ac(1V)
resistor R1 1k
capacitor C1 159.154943nF
connect VIN.minus to GND
connect VIN.plus to IN
connect R1.p1 to IN
connect R1.p2 to OUT
connect C1.p1 to OUT
connect C1.p2 to GND
simulate ac dec 40 100Hz 100kHz
assert gain(V(OUT),V(IN)) > 0.99
assert cutoff(V(OUT),V(IN)) > 990Hz
assert cutoff(V(OUT),V(IN)) < 1010Hz
```

First, validate syntax, semantics, ERC and schematic connectivity without simulation:

```powershell
kess check rc.kess
```

Then run the engineering assertions:

```powershell
kess test rc.kess
```

An agent should request structured output instead of parsing human prose:

```powershell
Get-Content rc.kess | kess test - --format json
```

Export visual, machine and editable artifacts from the same verified circuit:

```powershell
kess render rc.kess -o rc.svg
kess render rc.kess -o rc.png --scale 3
kess export rc.kess --target schematic-json -o rc.kessetsu.json
kess export rc.kess --target kicad -o rc.kicad_sch
kess export rc.kess --target ltspice -o rc.asc
```

Existing targets are not overwritten unless `--force` is explicit. Open the source in Web Hub to inspect the schematic and Bode result. **File → Save source** downloads a portable `.kess` file, while the versioned local browser draft protects unsaved work across reloads. Press **Share** to create a URL fragment containing a compressed copy of the source and exact model-package versions. No project upload or account is required.

Next try the canonical [gain stage](../core/tests/fixtures/benchmarks/gain_stage.kess) and [8 Ω power amplifier](../core/tests/fixtures/benchmarks/power_amplifier.kess). The supported physical boundary is documented in [supported domain](supported_domain.md).
