# Tutorial: From Source to Verified Circuit

This tutorial builds a first-order RC low-pass, verifies its cutoff and exports the result. You can paste the same source into Web Hub or save it as `rc.nl` for the CLI.

```netlang
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
netlang check rc.nl
```

Then run the engineering assertions:

```powershell
netlang test rc.nl
```

An agent should request structured output instead of parsing human prose:

```powershell
Get-Content rc.nl | netlang test - --format json
```

Export visual, machine and editable artifacts from the same verified circuit:

```powershell
netlang render rc.nl -o rc.svg
netlang render rc.nl -o rc.png --scale 3
netlang export rc.nl --target schematic-json -o rc.netlang.json
netlang export rc.nl --target kicad -o rc.kicad_sch
netlang export rc.nl --target ltspice -o rc.asc
```

Existing targets are not overwritten unless `--force` is explicit. Open the source in Web Hub to inspect the schematic and Bode result, then press **Share**: the URL fragment contains a compressed, versioned copy of the source and exact model-package versions. No project upload or account is required.

Next try the canonical [gain stage](../core/tests/fixtures/benchmarks/gain_stage.nl) and [8 Ω power amplifier](../core/tests/fixtures/benchmarks/power_amplifier.nl). The supported physical boundary is documented in [supported domain](supported_domain.md).
