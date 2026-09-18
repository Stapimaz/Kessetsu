# Simulation and Assertion Reference

Kessetsu compiles each source to canonical SPICE and runs the same typed analysis contract in native CLI and Web Hub. Native execution discovers a version-probed Ngspice executable; the browser uses the pinned Ngspice-derived worker runtime. Simulator console text is not the public API: datasets, measurements and assertions are versioned Rust Core objects.

## Analyses

| Statement | Result | Typical use |
|---|---|---|
| `simulate op` | scalar operating point | bias voltages/currents |
| `simulate tran <step> <stop>` | time series | waveform, RMS, THD, clipping, dissipation |
| `simulate tran <step> <stop> uic` | initialized time series | skips DC operating point; applies model capacitor initial conditions |
| `simulate ac dec|lin|oct <points> <start> <stop>` | complex frequency series | gain, phase and low-pass cutoff |
| `simulate dc <source> <start> <stop> <step>` | swept real series | transfer curve |

Each argument must be positive and dimensionally valid; DC sweep supports independent voltage/current sources. Multiple analysis kinds may coexist in one source. Operating-point, transient, and AC analyses are unique, while DC sweeps are unique per source; ambiguous repetitions fail semantic validation with `KES-C009`. The browser runs valid analyses independently in a cancellable Worker and Core combines them into `kessetsu.simulation.v1`.

## Assertions

The form is `assert metric(arguments) comparator threshold`. Comparators are `<`, `>`, `==`, `<=` and `>=`. Result states are:

- `PASS`: valid measurement satisfies the comparison.
- `FAIL`: valid measurement violates the comparison.
- `ERROR`: the metric could not be evaluated from the available typed data.
- `SKIPPED`: simulation did not complete, so the assertion was not evaluated.

Unsupported metric names and invalid argument shapes fail semantic validation with `KES-C006` before a simulator starts. Strict inequalities are not loosened. Equality and inclusive comparisons use the versioned absolute/relative tolerance reported with `kessetsu.assertion.v1`.

For supervised agents and CI, the same assertions may live in an evaluator-owned `.kessreq` file and be supplied with `kess test design.kess --requirements limits.kessreq`. This mode forbids inline assertions in the design, records the exact requirement-file SHA-256, and optionally verifies `--requirements-sha256`. See the [CLI reference](cli.md) for ownership and file behavior. Hash pinning detects changed limits; it is not a substitute for filesystem permissions controlled by the supervising process.

## Measurements

Primitive reductions include `value`, `min`, `max`, absolute `peak`, `average`/`avg` and `rms`. Derived metrics include gain, low-pass bandwidth/cutoff, frequency, phase, output power, efficiency, THD, clipping and device dissipation. Exact formulas, windows, required datasets, current polarity and power sign are normative in [engineering measurements](measurements.md).

## CLI examples

```powershell
kess simulate circuit.kess --format json
kess test circuit.kess --format json
Get-Content circuit.kess | kess test - --format json
```

Exit `3` denotes simulator/runtime failure; exit `4` denotes assertion `FAIL`, `ERROR` or `SKIPPED`. `kess test` also returns exit `4` and `KES-T000` without launching the simulator when no assertions are defined; use `kess simulate` when verification is not intended. JSON stdout remains one parseable object. Use `--include datasets,raw-log` only for debugging large/raw data.

## Reproducibility boundary

Kessetsu fixes source, Core schema, generated netlist, model/package hashes, simulator identity and evaluation formulas. Floating-point samples may differ slightly across simulator builds and platforms, so parity gates compare engineering decisions with declared tolerances instead of treating recorded decimals as universal golden bytes.
