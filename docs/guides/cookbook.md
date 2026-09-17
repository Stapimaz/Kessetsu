# Kessetsu Cookbook

## Start from a useful calculation

The Web toolkit offers a loaded voltage divider and an RC low-pass filter. Enter SI quantities,
choose exact/E12/E24 nominal values and inspect the achieved result, equations and assumptions.
**Open in editor** generates an editable circuit with OP or AC analysis; **Download .kess**
saves the source. Continue with simulation and the editor's existing exports. The previous
browser circuit is retained under **File → Restore previous circuit**.

With CLI 1.1.0 or newer:

```bash
kess tool divider --vin 12V --target 3V --lower 10k --load 10k --values exact --output divider.kess
kess tool rc-lowpass --cutoff 1kHz --resistance 1k --values e12 --output filter.kess
```

The exact divider above selects R1=15 kΩ and R2=10 kΩ: the 10 kΩ load makes the effective
lower resistance 5 kΩ, producing 3 V. Without that load the output would be 4.8 V.
The E12 RC tool selects 150 nF with 1 kΩ, giving about 1061 Hz rather than exactly 1000 Hz.
CLI 1.0.1 does not include these commands; update to 1.1.0 or newer. Component tolerances and physical
ratings remain separate from nominal calculations.

### Give an external agent a complete task

Copy this brief to an agent that can run your local terminal with CLI 1.1.0 or newer:

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

## Reuse a circuit block (unreleased)

This section requires a development source build. Published CLI 1.1.0 and the live Web
Hub do not yet support parameterized blocks. No migration is needed for literal circuits.

A **module** defines topology once. A **use** creates an instance with its own settings.
**Ports** connect that instance to the surrounding circuit; **parameters** change its
numeric values. There is no separate block editor, registry or new project format.

### One filter, two cutoff settings

The complete [reusable filter example](../../examples/reusable_filters.kess) includes its
source, connections, AC analysis and four fixed acceptance assertions. In development
Web builds, open **File → Examples → Reusable Filters**, then **Run simulation** in the
Simulation panel. With a source-built CLI, from the repository root:

```bash
kess test examples/reusable_filters.kess --format json
```

Its reusable definition is:

```kessetsu
module LowPass(input,output,gnd) {
  param resistance: Ohm = 1k
  param cutoff: Hz = 1k
  param capacitance: F = 1 / (2 * pi * resistance * cutoff)
  resistor R1 {resistance}
  capacitor C1 {capacitance}
  connect input to R1.p1
  connect R1.p2, C1.p1 to output
  connect C1.p2 to gnd
}
```

Create independent instances and connect their named ports, not their internal resistors:

```kessetsu
use LowPass SLOW(cutoff=500Hz)
use LowPass FAST(cutoff=2kHz)
connect VIN.plus, SLOW.input, FAST.input to IN
connect SLOW.output to SLOW_OUT
connect FAST.output to FAST_OUT
```

These excerpts are not standalone circuits: use the complete example for supplies/ground,
all connections and analysis. `use LowPass DEFAULT` uses the declared defaults.
Changing a cutoff recomputes its instance's capacitance; it does not modify another instance.
A supplied `capacitance=150nF` instead selects that nominal component explicitly: the actual
cutoff need not match the setting named `cutoff`. The formula assumes an ideal input source
and unloaded output. A load or cascaded stage changes the response; simulate the whole circuit.

### One amplifier, two gains

Open **File → Examples → Reusable Amplifiers**, or use the complete
[amplifier example](../../examples/reusable_amplifiers.kess):

```bash
kess test examples/reusable_amplifiers.kess --format json
kess test examples/reusable_amplifiers.kess --param second_gain=8 --format json --include ir --force
```

The example cascades two instances of one non-inverting topology with nominal stage gains
5 and 10, external ±12 V supplies and separate 10 kΩ loads. It measures each gain relative
to that stage's input; the overall nominal gain is 50, not 10. Its feedback resistor is calculated as
`(gain - 1) * reference`. The `second_gain=8` call deliberately fails the unchanged gain-10
requirement (exit 4); the first instance remains independently configured. `--force` permits
replacing the generated simulation artifact from the first call, not the original source.
On subsequent file-based runs, use it only when replacing that output is intended.
Editing a design setting must not silently rewrite its acceptance criteria.
The `KESSETSU_OPAMP_V1` model is
generic, not a manufacturer part. Finite bandwidth, output swing and loading still matter.
A requested gain of 1 makes this resistor topology invalid; use a directly wired follower
instead. The formula is not a guarantee for any requested gain or physical device.

### Save, identity and scope

- Keep the module definition and its `use` statements in the same `.kess` document. Saving,
  downloading or source-sharing that document preserves the definitions and overrides.
- Definition names are unique within the document. `FIRST` is an instance name, not a
  package version. Nested structured paths such as `STAGE.FILTER` are preserved in IR
  module-interface metadata; electrical IDs such as `STAGE_FILTER_R1` remain unchanged.
- Module bodies use their own parameters. Pass root/caller quantities explicitly, for
  example `use LowPass FAST(cutoff={fast_cutoff})`; globals are not implicitly captured.
- Use exact declared port names: a typo is an ERC error. Connect ground and power through
  explicit ports. Important external measurements should use named circuit nets.
- Put analyses and assertions at the circuit root. Parameterized module-local analysis
  and assertion contexts are unsupported; there are no automatic per-instance test runs.
- Blocks are source-embedded, not imported packages. `model_include` selects exact **model**
  packages; it does not install circuit modules. External module files, package version
  resolution and a block registry are not supported. Do not omit a definition from a share.

For terminal overrides, `--include effective-source` returns source with the accepted root
settings materialized; save that source if you want to reopen or share the exact effective
design. A command-line override alone does not edit the original file. For model dependencies,
keep the existing exact package/resource bindings. KiCad/LTspice exports flatten block topology
and do not preserve editable `.kess` module definitions; retain the source alongside exports.
See [parameters and limits](../reference/language.md#independent-module-parameters-unreleased)
and [root inputs](../reference/cli.md#root-parameter-inputs-unreleased).

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
