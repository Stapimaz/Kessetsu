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
simulate ac dec 40 10Hz 100kHz
assert gain(V(OUT),V(IN)) > 0.99
assert cutoff(V(OUT),V(IN)) > 990Hz
assert cutoff(V(OUT),V(IN)) < 1010Hz
assert phase(V(OUT),V(IN),1kHz) > -46deg
assert phase(V(OUT),V(IN),1kHz) < -44deg
```

## Run it in Web Hub

Open the [Web Hub](https://kessetsu.com/#editor). A previous browser draft may reopen automatically.
Save any work you want to keep, then choose **File → Examples → RC Low-pass**, or replace the
Source text with the circuit above. Both use the same five checks. Wait for **Source valid**, then press
**Run** in the Simulation panel. Select the AC analysis and a voltage signal to inspect its Bode
plot. The assertion table should show five passing checks: one gain limit, two cutoff limits and
two phase limits. A green source check alone is not a simulation result.

Change `R1` from `1k` to `2k` and run again. The schematic updates automatically, while the new
simulation shows which cutoff assertions no longer pass. Restore `1k`, run once more, then use
**Export** to download SVG, KiCad, LTspice, SPICE or another available artifact from the same source.

## Run it with the CLI

Save the source as `rc.kess`. First validate syntax, semantics, ERC and schematic connectivity
without simulation:

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

Existing targets are not overwritten unless `--force` is explicit. Open the source in Web Hub to inspect the schematic and simulation. Chromium browsers with the File System Access API provide **File → Save** / **Save As...** and reuse the selected `.kess` file. Firefox and other browsers without that API use **Save in browser** for a local checkpoint and expose **Download .kess...** separately for a portable copy. `Ctrl+S` always invokes the browser-appropriate Save action; `Ctrl+Shift+S` invokes Save As or Download. A successful save clears the unsaved marker, while cancellation or failure does not. The versioned local browser draft also protects unsaved work across reloads, but browser-local data is tied to that browser and device. Press **Share** to create a URL fragment containing a compressed copy of the source and exact model-package versions. No project upload or account is required.

Next try the runnable [gain stage](../../examples/gain_stage.kess) and [8 Ω power amplifier](../../examples/power_amplifier.kess). The supported physical boundary is documented in [supported domain](../reference/supported-domain.md).
