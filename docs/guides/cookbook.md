# Kessetsu Cookbook

## Start from a useful calculation

The toolkit in source builds / the next release offers a loaded voltage divider and an RC low-pass filter. Enter SI quantities,
choose exact/E12/E24 nominal values and inspect the achieved result, equations and assumptions.
**Open in editor** generates an editable circuit with OP or AC analysis; **Download .kess**
saves the source. Continue with simulation and the editor's existing exports. The previous
browser circuit is retained under **File → Restore previous circuit**.

For source builds / the next CLI release:

```bash
kess tool divider --vin 12V --target 3V --lower 10k --load 10k --values exact --output divider.kess
kess tool rc-lowpass --cutoff 1kHz --resistance 1k --values e12 --output filter.kess
```

The exact divider above selects R1=15 kΩ and R2=10 kΩ: the 10 kΩ load makes the effective
lower resistance 5 kΩ, producing 3 V. Without that load the output would be 4.8 V.
The E12 RC tool selects 150 nF with 1 kΩ, giving about 1061 Hz rather than exactly 1000 Hz.
Published CLI 1.0.1 does not yet include these commands. Component tolerances and physical
ratings remain separate from nominal calculations.

### Give an external agent a complete task

Copy this brief to an agent that can run your local terminal (source build / next CLI release):

> Use Kessetsu to design a nominal loaded divider. Vin is 12 V, RL is 10 kΩ and R2 is 10 kΩ.
> Require 2.97 V < V(OUT) < 3.03 V and absolute supply current below 1 mA.
> First inspect `kess --version`, `kess tool --help` and `kess tool divider --help`.
> Generate editable source, save the requirements separately and run `kess test` with JSON.
> Explain any failure, revise the candidate while preserving the requirements and retest.
> Export a PNG and KiCad schematic, and return the source, measured results and assumptions.

For a reproducible failure → revision example, save this unchanged `divider.kessreq`:

```kessetsu
assert value(V(OUT)) > 2.97V
assert value(V(OUT)) < 3.03V
assert peak(I(VIN)) < 1mA
```

```bash
# A deliberately wrong target fails the evaluator-owned voltage requirements.
kess tool divider --vin 12V --target 4V --lower 10k --load 10k --output divider.kess
kess test divider.kess --requirements divider.kessreq --format json --force
# Correct the candidate, keeping the requirement file unchanged.
kess tool divider --vin 12V --target 3V --lower 10k --load 10k --output divider.kess --force
kess test divider.kess --requirements divider.kessreq --format json --force
kess render divider.kess --output divider.png
kess export divider.kess --target kicad --output divider.kicad_sch
```

The first `test` exits 4 with a failed voltage assertion; the revised one passes. JSON
`diagnostics`, `assertions`, `measurements` and `artifacts` provide the feedback directly.
Use `--requirements-sha256` when the evaluator also needs hash pinning. Simulation success
is conditional on the nominal resistors and ideal source, not a hardware guarantee.

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

Read `diagnostics[]`, `assertions.assertions[]` and `assertions.summary`; do not scrape terminal sentences. The replayable [power-amplifier agent eval](../evals/power-amplifier-agent-v1.json) shows a failing 16 Ω candidate revised to the required 8 Ω load.

## Choose an export

- SVG: semantic text and best browser/vector handoff.
- PNG: presentations and raster previews; set `--scale` and background explicitly.
- PDF: one-page vector document.
- Schematic JSON: lossless versioned Kessetsu interchange.
- SPICE: canonical simulation netlist.
- KiCad/LTspice: editable handoff with the loss/capability report documented in [export formats](../reference/exports.md).

## Choose and verify a component model

Use the least complicated model that truthfully matches the job.

### 1. Built-in model

Known generic parts need no declaration:

```kessetsu
diode D1 1N4148
transistor Q1 npn 2N3904
mosfet M1 IRF540
opamp U1 KESSETSU_OPAMP_V1
```

Run `kess compile design.kess --format json --include models` to inspect exact provenance. Built-ins are useful for topology and first-pass verification; they are not manufacturer guarantees.

### 2. Exact Kessetsu package

Select the package by exact version and use the model it supplies:

```kessetsu
model_include kessetsu_analog 1.0.0
opamp U1 KESSETSU_PACKAGE_OPAMP
```

A file-based compile or simulation writes `kessetsu.lock` beside the generated SPICE artifact. Commit that lock with the circuit when reproducibility matters.

### 3. User-owned manufacturer op-amp model

Keep the downloaded model under the circuit directory; Kessetsu will not fetch or redistribute it:

```text
my-filter/
├── design.kess
└── models/
    └── VendorOp.lib
```

Calculate the exact SHA-256:

```powershell
(Get-FileHash .\models\VendorOp.lib -Algorithm SHA256).Hash.ToLowerInvariant()
```

```bash
sha256sum ./models/VendorOp.lib
```

Read the model file's `.SUBCKT` line and bind its real entry name and pin order explicitly:

```kessetsu
external_subcircuit opamp VendorOp (in_p,in_n,vcc,vee,out) file="models/VendorOp.lib" entry=VendorOp sha256=<64-hex-digest> version="vendor-version" license="vendor terms" source="https://vendor.example/model" simulator=ngspice_ps redistribution=prohibited
opamp U1 VendorOp
```

Then use a file-based native command so the relative resource can be validated and staged:

```powershell
kess compile .\design.kess --output .\design.spice --format json --include models
kess test .\design.kess --format json --include models
```

The digest, `.SUBCKT` entry, canonical five-pin mapping, license and compatibility mode must all match. The current Web Hub and stdin-only CLI cannot bind external file bytes and fail closed instead of substituting a generic model. See [language reference](../reference/language.md#typed-models-and-packages) for the complete contract.
