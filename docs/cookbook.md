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

## Choose and verify a component model

Use the least complicated model that truthfully matches the job.

### 1. Built-in model

Known generic parts need no declaration:

```kessetsu
diode D1 1N4148
transistor Q1 npn 2N3904
mosfet M1 nmos IRF540
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

The digest, `.SUBCKT` entry, canonical five-pin mapping, license and compatibility mode must all match. The current Web Hub and stdin-only CLI cannot bind external file bytes and fail closed instead of substituting a generic model. See [language reference](language_reference.md#typed-models-and-packages) for the complete contract.
