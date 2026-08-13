# Kessetsu Cookbook

## Bias and operating point

Add `simulate op`, then constrain a node with two explicit assertions:

```kessetsu
assert value(V(OUT)) > 4.9V
assert value(V(OUT)) < 5.1V
```

## One source for transient and AC

Use `sine_ac` when the same input drives time- and frequency-domain analyses:

```kessetsu
source VIN sine_ac(0V,100mV,1kHz,1V)
simulate tran 5us 10ms
simulate ac dec 30 10Hz 10MHz
```

## Ignore startup in steady-state measurements

Pass an inclusive time window to reductions and derived metrics:

```kessetsu
assert rms(V(OUT),2ms,10ms) > 3V
assert output_power(V(OUT),RL,2ms,10ms) > 1.9W
assert dissipation(Q1,2ms,10ms) < 2W
```

## Check transistor stress with explicit terminals

```kessetsu
assert peak(V(Q1.c,Q1.e),2ms,10ms) < 40V
assert peak(I(Q1)) < 1A
assert peak(P(Q1)) < 5W
```

## Agent retry loop without temporary files

Send the complete candidate on stdin, inspect only structured fields, revise and repeat:

```powershell
Get-Content candidate.kess | kess check - --format json
Get-Content candidate.kess | kess test - --format json
```

Read `diagnostics[]`, `assertions.assertions[]` and `assertions.summary`; do not scrape terminal sentences. The replayable [power-amplifier agent eval](evals/power-amplifier-agent-v1.json) shows a failing 16 Ω candidate revised to the required 8 Ω load.

## Choose an export

- SVG: semantic text and best browser/vector handoff.
- PNG: presentations and raster previews; set `--scale` and background explicitly.
- PDF: one-page vector document.
- Schematic JSON: lossless versioned Kessetsu interchange.
- SPICE: canonical simulation netlist.
- KiCad/LTspice: editable handoff with the loss/capability report documented in [export formats](export_formats.md).
