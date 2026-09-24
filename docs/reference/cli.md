# Kessetsu CLI Reference

Development source also provides `kess study create/plan/run/export` and
`kess fit evaluate`; see the [study guide](../guides/parameter-studies.md) for
specifications, checkpoints, exit codes and reports, and the
[fitting guide](../guides/model-fitting.md) for calibration/validation semantics.
Published 1.2.0 does not include these commands yet.

The Kessetsu CLI sends `.kess` source through the shared Rust compilation pipeline and provides ERC, SPICE generation, Ngspice execution, and assertion evaluation commands. Human output is intended for people; versioned JSON output is intended for automation and AI agents.

Kessetsu 1.2.0 uses compile contract v5 and supports typed parameters, reusable blocks
and local external devices. Earlier CLI versions require an update for these additions.

## Usage

```bash
kess [--format human|json] [--schema-version kessetsu.cli.v1] \
  [--include ast,ir,graph,spice,datasets,simulation,models,raw-log,effective-source] \
  [--param NAME=VALUE] <COMMAND> [OPTIONS] [FILE]
```

`--format` is a true global option and can appear before or after the subcommand:

```bash
kess --format json check examples/rc_low_pass.kess
kess check examples/rc_low_pass.kess --format json
```

`--schema-version` and `--include` are also global options. `--include` accepts a comma-separated list or repeated uses. An unknown schema version is rejected with `KES-F002` and exit code `2` before the source is read or any output is created.

## Root parameter inputs

Kessetsu 1.2.0 accepts repeated `--param NAME=VALUE` on `check`, `compile`, `simulate`,
`test`, `render` and `export`. This is not available in published 1.1.0.

```bash
kess test filter.kess --param frequency=2kHz --param amplitude=2V --format json
kess compile - --param frequency=2kHz --format json --include effective-source,ir < filter.kess
```

Names refer only to declared root `param` quantities, are case-sensitive and cannot be
repeated. Values are numeric literals with optional compatible units; whole-literal
shorthand uses the declaration's unit. Keep formulas in source, not in `--param`.
Dependent defaults are recalculated. Invalid names, units, expressions or duplicate inputs
fail before output writes or simulator launch. Module values use existing `use` overrides;
CLI inputs cannot address a flattened instance path. `tool` calculations reject `--param`.

The source file is never edited. `--include effective-source` returns `debug.effective_source`:
ordinary editable `.kess` with only supplied root defaults replaced, preserving dependent
formulas, module defaults and unrelated comments/formatting. Without overrides it returns
the original source. Save or share this effective source to reproduce the used settings;
the original source alone still contains its original defaults. Materialized external-model
references remain source-relative: retain their resource directory when saving elsewhere.
`--include ir` records original defaults, supplied inputs and resolved quantities in
`debug.ir.parameter_manifest`. Human output lists effective root inputs on stderr; default
JSON remains compact. Independent `.kessreq` bytes and limits are never changed.

## Circuit tools

The tools calculate nominal values and generate ordinary editable `.kess` circuits. These
commands require **CLI 1.1.0 or newer**. Older installations can use the install guide to update.

```bash
kess tool divider --vin 12V --target 3V --lower 10k --load 10k --values exact --output divider.kess
kess tool rc-lowpass --cutoff 1kHz --resistance 1k --values e12 --output filter.kess
kess simulate divider.kess
kess export filter.kess --target kicad
```

`--values exact|e12|e24` selects nearest nominal component values (default E24), independently
for each designed component. An omitted `--load` means an open load. A load value is never rounded.
The RC calculation assumes an ideal source and high-impedance output.

No file is written unless `--output path.kess` is supplied. Existing outputs require `--force`.
Invalid units, nonpositive values, an impossible divider target and unrepresentable calculations
produce exit code `2`. `--format json` uses the existing CLI envelope with an additional
`calculation` field (`kessetsu.tool.v1`): normalized inputs, ideal/selected components, results
with units, equations, assumptions and editable source. Calculations are analytical, not
simulation PASS claims. Use normal `simulate`, `test`, `render` and `export` commands on the
generated source to continue. Source builds expose command details through `kess tool --help`.

## Research data

Development builds can preview instrument-style CSV, preserve an explicitly mapped and
unit-normalized dataset, and compare two imported scalar datasets without launching a
simulator:

```bash
kess data preview capture.csv --delimiter semicolon --decimal comma --skip-records 2 --format json
kess data import capture.csv --mapping capture.kessimport.json --output capture.kessdata.json
kess simulate circuit.kess --format json --include simulation > simulation.json
kess data from-simulation simulation.json --mapping circuit.kesssim.json \
  --output simulation.kessdata.json
kess data compare capture.kessdata.json --reference reference.kessdata.json \
  --mapping comparison.kesscompare.json --output residuals.json --format json
```

Preview never guesses a final mapping. Import records the original CSV bytes, SHA-256,
column/unit mapping, normalized SI values, metadata and skipped-row reasons in
`kessetsu.research-data.v1`. Compare uses an explicit signal mapping, interpolation and
coverage policy and writes the point-by-point residual evidence plus bias, MAE, RMSE and
maximum absolute error. Default JSON stdout stays compact; `--include datasets` includes
the complete imported arrays or comparison points.

`data from-simulation` accepts either raw `kessetsu.simulation.v1` JSON or a
`kessetsu.cli.v1` envelope produced with `--include simulation`. It applies the same
Core-owned projection as the Web research-data workflow and records the solver, result hash,
analysis index and vector mapping. Operating-point results have no comparison axis and are
rejected. The explicit simulation include contains the complete typed result, including raw
simulator logs and artifact paths; review it before sharing. Use the smaller `datasets`
include when that provenance is not needed.

Existing outputs require `--force`, and an output may never alias a CSV, mapping, dataset
or reference input. Data commands reject `--param` and return exit code `2` for invalid
data, mappings, identity checks or file operations. They compare real scalar signals.
Finite fitting is a separate command over completed study results; direct arbitrary
study-result comparison and Web import are not implied.
See the [research-data guide](../guides/research-data.md) for mapping schemas, locale and
unit rules, missing-data behavior, comparison semantics, provenance and limits.

## Finite parameter fitting

Development builds can score candidates from a completed parameter study against explicit
calibration and holdout-validation datasets:

```bash
kess fit evaluate study-results.json --spec model.kessfit.json \
  --data calibration=capture.kessdata.json \
  --data validation=holdout.kessdata.json \
  --output fit-result.json --format json
```

The specification binds 1-8 fitted parameters to existing study axes and 2-32 observations
to exact revision, temperature and optional non-fit conditions. At least one calibration and
one validation observation are required. Selection uses only calibration scores; validation
is reported separately and cannot change the selected candidate. Failed observations, full
residual comparisons, masks, boundary hits and near-equivalent candidates remain in the
`kessetsu.fit-result.v1` artifact.

Default stdout is a compact candidate summary. `--include datasets` also places complete
residual evidence in stdout; the output artifact is always complete. Existing output requires
`--force`, but cannot alias any input. The command rejects global `--param` and returns exit
code `2` with `KES-R002` on invalid input or file operations. See the
[fitting guide](../guides/model-fitting.md) for the specification, dimensionless score and
interpretation limits.

## Stdin and File-Free Agent Use

When the file path is `-`, Kessetsu reads source from stdin:

```bash
kess check - --format json < circuit.kess
kess compile - --format json --include spice < circuit.kess
kess simulate - --format json < circuit.kess
kess test - --format json < circuit.kess
```

With stdin, `compile`, `simulate`, and `test` do not write a SPICE file into the working directory unless an explicit `--output` is provided. Use `--include spice` when a JSON `compile` call needs the generated netlist. Human-mode `compile -` prints the netlist to stdout. To retain an artifact, provide an output such as `--output result.spice`; the normal safe-overwrite contract still applies.

Side-effect-free stdin plus JSON is idempotent for agent retries: the same source, options, and deterministic simulator result produce a byte-for-byte identical envelope. A caller that requests file output must use `--force` when intentionally replacing an existing artifact.

## Commands

### `check`

Runs parsing, semantic validation, and ERC without producing a file.

```bash
kess check examples/rc_low_pass.kess
```

### `compile`

Generates a SPICE netlist after all checks pass.

```bash
kess compile examples/rc_low_pass.kess
kess compile examples/rc_low_pass.kess --output build/rc-low-pass.spice
```

The default destination is the source path with a `.spice` extension. Existing files are never overwritten silently; intentional replacement requires `--force`:

```bash
kess compile examples/rc_low_pass.kess --force
```

The `--output` path is resolved relative to the working directory. If the destination is the source file itself, the operation is rejected even with `--force`. The CLI does not create missing parent directories automatically.

### `simulate`

Compiles the source, writes the SPICE file under the same output policy, and runs Ngspice in batch mode through the shared simulation runner. The runner probes the executable version, creates a unique temporary directory for every run, and applies a default 30-second timeout. A failed simulator process status or fatal/error output returns exit code `3`. Human and JSON renderers use the same typed `SimulationResult`; raw simulator logs appear only through explicit `--include raw-log`.

The language supports `op`, `tran`, `ac`, and `dc` sweeps of independent voltage/current sources. Analysis arguments and physical units are validated during semantic conversion. Operating-point, transient, and AC analyses are unique; DC sweeps are unique per source. Unsupported, malformed, or ambiguous repeated analyses produce source-located `KES-C009`, and the simulator does not start.

```bash
kess simulate examples/rc_low_pass.kess --force
```

### `test`

Evaluates source assertions after compilation and simulation. A simulation failure returns exit code `3`; an unsuccessful assertion result returns exit code `4`. A source with no assertions fails before simulator launch with `KES-T000` and exit code `4`; use `simulate` when no verification contract is intended.

```bash
kess test examples/rc_low_pass.kess --force
```

For a supervised agent or CI loop, keep requirements outside the editable design and pass an assertion-only `.kessreq` file:

```bash
kess test design.kess --requirements limits.kessreq --format json --force
kess test design.kess --requirements limits.kessreq \
  --requirements-sha256 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef \
  --format json --force
```

External requirements use `kessetsu.requirements.v1`, allow only comments and `assert` statements, and are compiled by the same Core assertion semantics. The design must contain no inline assertions when `--requirements` is present; mixed ownership fails with `KES-R003`. JSON records the exact-byte SHA-256, count, and whether the caller pinned an expected digest. A mismatched or malformed digest fails with `KES-R004` before simulation. Keep the file or expected digest outside the design agent's write authority when tamper resistance matters.

Each assertion receives a deterministic `KES-Txxx` code in source order. Results are separated into `PASS`, `FAIL`, `ERROR`, and `SKIPPED`: an unmet threshold is `FAIL`; a missing measurement is `ERROR`; and an incomplete simulation produces `SKIPPED`. Unsupported metric names and invalid argument shapes are semantic `KES-C006` errors and never reach the simulator. If any non-passing assertion state exists, the command returns exit code `4`.

Supported primitive metrics are `value`, `min`, `max`, `peak`, `average`/`avg`, and `rms`. `peak` means `max(abs(x))`, not the signed maximum. In OP analysis, `value`, `min`, `max`, and `average` return the signed scalar value, while `peak` and `rms` return its magnitude. Equality and inclusive comparators use defined absolute/relative tolerances for small numeric differences; strict `<` and `>` boundaries are not relaxed.

Positive current enters the component's canonical positive/reference pin. This is the `plus` pin for a voltage source, so the measured current of a source delivering power is often negative. Human output displays engineering prefixes together with physical units. JSON contains the same typed assertion report and numeric summary.

When required, set `KESSETSU_NGSPICE` to an explicit simulator executable path. If that path cannot be launched, simulation fails closed with exit code `3`.

### `render`

Produces SVG, PNG, or a single-page vector PDF from canonical Schematic IR. The output extension selects the format. PNG scale is limited to `0.25..8`, and background can be `white|transparent`.

```bash
kess render circuit.kess --output circuit.svg
kess render circuit.kess --output circuit.png --scale 3 --background transparent
kess render circuit.kess --output circuit.pdf
```

### `export`

Produces machine-readable or editable artifacts through the shared versioned exporter contract:

```bash
kess export circuit.kess --target schematic-json --output circuit.kessetsu.json
kess export circuit.kess --target spice --output circuit.spice
kess export circuit.kess --target kicad --output circuit.kicad_sch
kess export circuit.kess --target ltspice --output circuit.asc
```

`render` and `export` stop without creating output and emit a `KES-X...` diagnostic when canonical connectivity is not verified or when the target cannot safely represent a required feature. Replacing an existing file requires `--force`. With `--format json`, artifacts report schema/version, MIME type, SHA-256, byte length, connectivity, capability, warnings, and known losses. See the [export matrix](exports.md) for format boundaries.

## JSON Contract

JSON stdout is exactly one JSON object for every invocation. Progress and simulator logs are never written to stdout. The default agent envelope is `kessetsu.cli.v1`. Current development compile reports use `kessetsu.compile.v6`, canonical schematics use `kessetsu.schematic.v3`, model manifests/locks use `kessetsu.models.v3`/`kessetsu.lock.v3`, simulation results use `kessetsu.simulation.v1`, engineering measurements use `kessetsu.measurement.v3`, assertion reports use `kessetsu.assertion.v1`, external requirement sets use `kessetsu.requirements.v1`, and finite fitting uses `kessetsu.fit.v1` plus `kessetsu.fit-result.v1`. Published 1.2.0 uses compile v5/schematic v2/model v2/lock v2/measurement v2; 1.1.0 uses compile v4/measurement v1. Active subcontracts appear in `domain_versions`. Resolved parameter/field provenance (`kessetsu.parameters.v1`) is opt-in through `--include ir`, not additional default JSON bulk.

See the [engineering-measurement contract](measurements.md) for assertion primitives, derived-metric formulas, analysis requirements, and sign conventions.

Default output is intentionally compact. In addition to command/schema metadata, it contains only `status`, diagnostics, summary, measurements, assertions, and artifact references. Canonical AST/IR/graph/SPICE, analysis datasets, the complete typed simulation, model manifest/lock content, and raw logs appear under `debug` only when selected through the corresponding `--include` option.

Successful `check` summary:

```json
{
  "schema_version": "kessetsu.cli.v1",
  "command": "check",
  "status": "success",
  "domain_versions": {
    "compile": "kessetsu.compile.v6",
    "simulation": null,
    "measurement": null,
    "assertion": null,
    "requirements": null,
    "export": null
  },
  "diagnostics": [],
  "summary": {
    "errors": 0,
    "warnings": 0,
    "info": 0,
    "analyses": 0,
    "measurements": 0,
    "assertions": null
  },
  "measurements": {},
  "assertions": null,
  "requirements": null,
  "artifacts": []
}
```

Examples that request debug fields:

```bash
kess compile circuit.kess --format json --include ast,ir,graph,spice
kess simulate circuit.kess --format json --include datasets,raw-log --force
kess simulate circuit.kess --format json --include simulation --force
kess compile circuit.kess --format json --include models --force
```

The first command adds `debug.ast`, `debug.ir`, `debug.graph`, and `debug.spice_netlist`; the second adds `debug.datasets` and `debug.raw_log`; the third adds the complete `debug.simulation`; and the fourth adds `debug.models.manifest` and `debug.models.lock`. Unselected large fields are omitted entirely rather than serialized as `null`.

The assertion field of a `test` result is independently versioned and summarized:

```json
{
  "schema_version": "kessetsu.assertion.v1",
  "assertions": [
    {
      "code": "KES-T001",
      "status": "PASS",
      "metric": "peak",
      "signal": "I(V1)",
      "actual": 0.005,
      "threshold": 0.1,
      "unit": "A"
    }
  ],
  "summary": {
    "total": 1,
    "passed": 1,
    "failed": 0,
    "errors": 0,
    "skipped": 0
  }
}
```

Diagnostic fields are shared across every stage:

```json
{
  "code": "KES-E003",
  "severity": "error",
  "stage": "erc",
  "message": "Floating Pin: R1.p1 is not connected to anything.",
  "component": "R1",
  "pin": "p1"
}
```

A successful `compile` reports the written SPICE file as a `spice_netlist` entry in `artifacts`. If a model or subcircuit is used, the deterministic `kessetsu.lock` beside it is also reported as a `model_lock` artifact. Netlist text appears only with `--include spice`; model provenance and lock content appear only with `--include models`. On I/O or runtime failure, `status` is never `success`.

Kessetsu 1.2.0 checks both SPICE and model-lock destinations before writing
either. An identical existing lock is reused without rewriting; replacing different lock
contents requires `--force`. Even with `--force`, the lock destination cannot be the source
or the SPICE destination. Published 1.1.0 does not yet include this lockfile protection.

## Model and Subcircuit Use

User-defined device models and op-amp subcircuits are typed declarations, not raw SPICE:

```kessetsu
model diode SafeD version=1.0.0 license=MIT Is=2e-9 Rs=0.5
model bjt SafeN npn version=1.0.0 license=MIT Is=1e-12 Bf=100
model mosfet SafeP pmos version=1.0.0 license=MIT Vto=-2 Kp=4
subcircuit opamp SafeOp (in_p,in_n,vcc,vee,out) version=1.0.0 license=MIT gain=100k bandwidth=2MHz
external_subcircuit opamp OPA197 (in_p,in_n,vcc,vee,out) file="models/OPAx197.LIB" entry=OPAx197 sha256=<64-hex-digest> version="Final 1.3" license="vendor terms" source="vendor URL" simulator=ngspice_ps redistribution=prohibited
```

Exact packaged-model selection:

```kessetsu
model_include kessetsu_analog 1.0.0
opamp U1 KESSETSU_PACKAGE_OPAMP
```

`version` and `license` are required on user declarations; `source` is optional for generated typed models. Allowed parameters are restricted by kind. External declarations require every shown field, resolve `file` relative to a file-based `.kess` source, and verify its exact bytes and catalog-terminal `.SUBCKT` entry before IR. `ngspice_ps` is a closed compatibility value, not arbitrary simulator arguments. Native stdin has no external-byte binding and fails with `KES-C015`; no generic fallback occurs. Web accepts explicit local file selection for the portable profile.  `comparator`/`two_terminal` declarations instantiate with `device`; see the [model catalog](model-catalog.md). SPICE/LTspice output using the relative dependency must stay beside the source. Unknown parameters, incorrect polarity/kind, invalid pin order, duplicate names, resource/hash failures, or raw-directive payloads produce structured `KES-C010..017` diagnostics. Built-in generic verification paths remain available but are never substituted for a requested external model.

## Exit Codes

- `0`: Success.
- `1`: Flattening, semantic-validation, or ERC failure.
- `2`: Parse, I/O, safe-output-policy, or export/render frontend failure.
- `3`: Simulator launch, process, or runtime failure.
- `4`: One or more assertions did not pass.

Human and JSON formats use the same code path and exit semantics.

## Agent Loop

No separate daemon or dedicated Agent API is required. An agent can implement the following loop by reading JSON fields only:

1. Run `check - --format json` to obtain syntax, semantic, and ERC diagnostics.
2. Run `compile - --format json --include spice` when inspection of the canonical netlist is needed.
3. Run `simulate - --format json` to read typed measurements and the analysis summary.
4. Run `test - --requirements frozen.kessreq --format json` when the supervising process, rather than the design source, owns acceptance criteria. Optionally pin `--requirements-sha256`; record `requirements.sha256` from the response.
5. Read `assertions[].status`, `actual`, `threshold`, and summary fields to determine the remaining target difference.
6. Revise only the design source and repeat the same stdin call.

The repository contract suite verifies this flow with a cross-platform fixture: it reads the measured 200 mA result from a failing 10 Ω candidate, revises the resistor to 100 Ω, and passes the assertion at 20 mA. The test never parses human terminal text.
