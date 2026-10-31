# Tutorial: From Source to Verified Circuit

This tutorial walks you through building, simulating, and verifying your first circuit in Kessetsu. You can run everything directly in your browser with zero setup, or locally from your terminal.

- **Goal:** Build and verify a 1 kHz first-order RC low-pass filter (`R1 = 1 kΩ`, `C1 = 159.154943 nF`).
- **What Kessetsu checks:** Passband gain (`> 0.99`), −3 dB cutoff frequency (`990 Hz` to `1010 Hz`), and phase at 1 kHz (`-46°` to `-44°`).
- **Time required:** 2 minutes in your browser (no installation needed), or 5 minutes with the local CLI.

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

### Circuit breakdown

- `net ...` — names the electrical nodes (`GND`, `IN`, `OUT`).
- `source / resistor / capacitor ...` — declares the parts and their values.
- `connect ...` — wires component pins to nets (verified before simulation by Electrical Rules Check).
- `simulate ...` & `assert ...` — runs an Ngspice AC frequency sweep and grades each requirement as `PASS` or `FAIL`.

## Run it in Web Hub

1. **Open the workspace:** Open [Web Hub](https://kessetsu.com/#editor) and choose **File → Examples → RC Low-pass** (or paste the circuit above).
2. **Simulate the circuit:** Click **Simulate circuit** on the schematic bar (or switch to **Simulation** / **Split** view and press **Run**).
3. **Inspect the plots and requirements:** Select the **ac** analysis tab to inspect the magnitude and phase Bode plots and verify that all **5 / 5** engineering requirements pass.
4. **See a requirement fail:** Change `resistor R1 1k` to `resistor R1 2k` and run again—the schematic updates automatically, and the cutoff assertions report `FAIL` with the new ~500 Hz cutoff. Change `R1` back to `1k`.
5. **Export your schematic:** Click **Export** in the top bar to download SVG, PNG, PDF, KiCad (`.kicad_sch`), LTspice (`.asc`), SPICE, or BOM CSV.

### Saving and sharing in the browser

Chromium browsers with the File System Access API provide **File → Save** / **Save As...** and reuse the selected `.kess` file. Firefox and other browsers without that API use **Save in browser** for a local checkpoint and expose **Download .kess...** separately for a portable copy. `Ctrl+S` always invokes the browser-appropriate Save action; `Ctrl+Shift+S` invokes Save As or Download. A successful save clears the unsaved marker, while cancellation or failure does not. The versioned local browser draft also protects unsaved work across reloads, but browser-local data is tied to that browser and device. Press **Share** to create a URL fragment containing a compressed copy of the source and exact model-package versions. No project upload or account is required.

## Run it with the CLI

Install [Kessetsu CLI and its simulator prerequisites](https://kessetsu.com/install/) first.
`kess check` needs only the CLI; `kess test` also needs Ngspice. Browser users do not need
either local installation.

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

Existing targets are not overwritten unless `--force` is explicit. Open the source in Web Hub to inspect the schematic and simulation.

Next try the runnable [gain stage](../../examples/gain_stage.kess) and [8 Ω power amplifier](../../examples/power_amplifier.kess). The supported physical boundary is documented in [supported domain](../reference/supported-domain.md).
