# Simulation and Assertion Reference

Kessetsu compiles each source to canonical SPICE and runs the same typed analysis contract in native CLI and Web Hub. Native execution discovers a version-probed Ngspice executable; the browser uses the pinned Ngspice-derived worker runtime. Simulator console text is not the public API: datasets, measurements and assertions are versioned Rust Core objects.

## Analyses

| Statement | Result | Typical use |
|---|---|---|
| `simulate op` | scalar operating point | bias voltages/currents |
| `simulate tran <step> <stop>` | time series | waveform, RMS, THD, clipping, dissipation |
| `simulate ac dec|lin|oct <points> <start> <stop>` | complex frequency series | gain, phase and low-pass cutoff |
| `simulate dc <source> <start> <stop> <step>` | swept real series | transfer curve |

Each argument must be positive and dimensionally valid; DC sweep supports independent voltage/current sources. Multiple analyses may coexist in one source. The browser runs them independently in a cancellable Worker and Core combines them into `kessetsu.simulation.v1`.

## Assertions

The form is `assert metric(arguments) comparator threshold`. Comparators are `<`, `>`, `==`, `<=` and `>=`. Result states are:

- `PASS`: valid measurement satisfies the comparison.
- `FAIL`: valid measurement violates the comparison.
- `ERROR`: the metric could not be evaluated from the available typed data.
- `SKIPPED`: simulation did not complete, so the assertion was not evaluated.

Strict inequalities are not loosened. Equality and inclusive comparisons use the versioned absolute/relative tolerance reported with `kessetsu.assertion.v1`.

## Measurements

Primitive reductions include `value`, `min`, `max`, absolute `peak`, `average`/`avg` and `rms`. Derived metrics include gain, low-pass bandwidth/cutoff, frequency, phase, output power, efficiency, THD, clipping and device dissipation. Exact formulas, windows, required datasets, current polarity and power sign are normative in [engineering measurements](engineering_measurements.md).

## CLI examples

```powershell
kess simulate circuit.kess --format json
kess test circuit.kess --format json
Get-Content circuit.kess | kess test - --format json
```

Exit `3` denotes simulator/runtime failure; exit `4` denotes assertion `FAIL`, `ERROR` or `SKIPPED`. JSON stdout remains one parseable object. Use `--include datasets,raw-log` only for debugging large/raw data.

## Reproducibility boundary

Kessetsu fixes source, Core schema, generated netlist, model/package hashes, simulator identity and evaluation formulas. Floating-point samples may differ slightly across simulator builds and platforms, so parity gates compare engineering decisions with declared tolerances instead of treating recorded decimals as universal golden bytes.
