# Parameter Studies

A study repeats circuit simulation over declared component values, supplies, loads,
temperatures or tolerances, keeping every outcome together. It does not silently change
your open source or relax its requirements.

## First study in Web

1. Choose **File → Examples → Loaded Filter**.
2. Open **Analyze → Test parameter variations…**. It takes a source snapshot.
3. Select `resistance` and enter `820Ohm, 1kOhm, 1.2kOhm` as an explicit list.
4. Add `load`, with `2kOhm, 10kOhm`. These lists produce six combinations.
5. Run. Three cases pass and three fail: heavy loading reduces gain.
6. Inspect assertion values and select cases for an overlay. Download full Results JSON,
   summary CSV, full numeric CSV, SVG or a printable HTML report.

Alternatively open the [filter study](../../examples/studies/loaded-filter.kessstudy.json).
The [driver study](../../examples/studies/transistor-driver.kessstudy.json) has twelve
supply/load/temperature combinations with a generic `2N3904`. It demonstrates model-dependent
behavior, not manufacturer ratings or physical hardware validation.

**Apply parameters** replaces root parameter declarations after confirmation. Temperature,
independent requirements and results stay in the study, not ordinary editor simulation.
The source snapshot remains independent until you choose **Use current circuit**.

## CLI workflow

```sh
kess study create examples/loaded_filter.kess --output filter.kessstudy.json --sweep resistance=820Ohm,1kOhm,1.2kOhm --sweep load=2kOhm,10kOhm
kess study plan filter.kessstudy.json --format json
kess study run filter.kessstudy.json --output filter-results.json --format json
kess study export filter-results.json --output filter.csv --target csv
kess study export filter-results.json --output samples.csv --target data-csv
kess study export filter-results.json --output overlay.svg --target svg --signal "V(OUT)"
kess study export filter-results.json --output report.html --target html
kess study package filter.kessstudy.json --results filter-results.json --output filter-package --signal "V(OUT)"
```

The run exits `1` because some constraints fail, but writes the full result file.
Study exit codes: `0` completed/feasible, `1` constraint failure, `2` invalid configuration,
I/O or identity, `3` execution/measurement errors, `130` cancellation. Without constraints
the status is `completed`, never `passed`. Progress goes to stderr; JSON stdout is one
compact `kessetsu.cli.v1` object. Source/spec/model destinations cannot be overwritten.
Studies do not write adjacent SPICE or lock files.

Ctrl+C terminates the active native solver and preserves partial checkpoints. Continue:

```sh
kess study run filter.kessstudy.json --output filter-results.json --resume
```

Complete cases, including failed/error cases, are reused; cancelled/pending cases run again.
Choose a new output or `--force` for a fresh run to retry completed failures.
Existing output requires `--force`, except the explicitly selected resume checkpoint.

## Specification and candidate grids

The editable JSON contract is `kessetsu.experiment.v1`; `.kessstudy.json` is a convention.
It embeds normal `.kess` source and optionally literal evaluator-owned requirements.
Model files remain separate. Only declared root parameters can be varied.

Use the complete downloadable examples as templates. Optional fields include:

```json
"axes": [
  {"parameter": "resistance", "values": {"kind": "linear", "start": "1kOhm", "stop": "3kOhm", "points": 5}},
  {"parameter": "load", "values": {"kind": "log", "start": "1kOhm", "stop": "100kOhm", "points": 3}}
],
"temperatures_c": [27],
"timeout_ms": 30000,
"measurements": [{"name": "corner", "expression": "upper_cutoff(V(OUT),V(IN))", "unit": "Hertz"}],
"objective": {"measurement": "corner", "direction": "minimize"}
```

This fragment belongs inside a complete specification. Linear/log endpoints are inclusive;
log endpoints must be positive. Lists preserve order. Every combination of axes, revisions,
tolerance samples and temperatures counts against the evaluation budget. Case quantities
retain full round-trip precision, not display-rounded values.

Optional `revisions` is a list of `{ "name": "Revision B", "source": null,
"parameters": { "resistance": "2kOhm" } }` objects. Omitted source uses main source;
replacement source embeds another complete circuit. Axes apply to every revision.
Web **Advanced JSON** supports combined studies/revisions. Compare exported Results JSON
files to overlay up to six cases from each, without merging or rewriting their evidence.
Normal overlays allow twelve selected cases; in CLI repeat `--case CASE_ID` for selection.

## Fixed constraints and finite search

Inline assertions are design-owned constraints. Optional `requirements` embeds exact
literal `.kessreq` bytes and cannot mix with inline assertions. Independent limits cannot
resolve design parameters. Changed requirements invalidate resume.

Measurements are observations, not constraints. Name a shared metric and its enum unit:
`Volt`, `Ampere`, `Hertz`, `Second`, `Watt`, `Joule`, `Ratio`, `Percent`, `Degree`,
`Ohm`, `Farad` or `Henry`. Study observations require literal numeric arguments.
Missing/malformed observations are errors, not feasible candidates.

An objective minimizes/maximizes one observation among feasible evaluated cases. The whole
declared grid is the finite budget; no adaptive/unbounded search occurs. With no feasible
case there is no best candidate. Without constraints, completed candidates are comparable
but not certified designs. No global optimum is claimed.

## Tolerances and temperature

Choose **Nominal + tolerance corners** or **Nominal + Monte Carlo samples**.
Uniform `relative: 0.05` means ±5% half-width; Gaussian means 5% standard deviation,
not a maximum bound. Gaussian samples are unbounded: invalid components remain error
cases rather than being silently clipped.

```json
"tolerances": {
  "mode": "monte_carlo", "seed": 42, "samples": 12,
  "parameters": [
    {"parameter": "resistance", "relative": 0.05, "distribution": "uniform", "group": "batch"},
    {"parameter": "load", "relative": 0.1, "distribution": "uniform", "group": "batch"}
  ]
}
```

Nominal precedes samples. Same-group parameters share one normalized signed deviation:
perfect positive correlation, not an arbitrary correlation matrix. Group distributions
must match. Ungrouped parameters draw independently. Corners enumerate all ±uniform
combinations plus nominal; grouped/Gaussian corners are rejected. Axes and tolerances
cannot target the same parameter. Seeds fix draw order and repeat generation in the same
runtime. Gaussian transcendental rounding can differ across platforms; compare quantities
and solver results with numerical tolerances, not bit-identical assumptions.

Temperature is °C, default 27. The accepted −200…300 range is a software bound, not model
validity. Ngspice `.temp` affects only temperature-dependent model equations; ideal passives
and generic ideal op-amps gain no thermal coefficients. See the
[Ngspice manual](https://ngspice.sourceforge.io/docs/ngspice-manual.pdf).
Local libraries with conflicting control/temperature overrides are rejected.
Finite samples/corners do not establish production yield or hardware safety.

## Cancellation, identity and limits

Native: up to 256 cases. Web: up to 32. One active solver, 1 ms–120 s per-case timeout.
Core caps retained numeric values at two million per case; native checkpoint JSON is capped
at 64 MiB, Web retained JSON at 32 MiB. Narrow large analyses or split studies. Native capacity
failure preserves the previous durable checkpoint, not a false completion.

Closing the Web dialog stops the study; reopening preserves partial results in the same
session. Download Results JSON before refreshing/leaving: studies are in memory, not drafts
or a cloud account. For larger work, download the spec and run CLI. Cross-adapter report
comparison works, but resume deliberately requires the original adapter/runtime identity.

Identity binds exact source, fixed requirements, models/manifests, conditions, seed, analyses,
timeouts, Core/measurement contracts and runtime fingerprint. Changed inputs/model bytes/
runtime build refuse resume. Checksums detect accidental corruption, not signed independent
attestations; they cannot protect against an owner deliberately rewriting evidence.

Native model paths resolve relative to the spec directory. Creating a spec does not copy
resources; keep exact referenced paths/bytes when moving it. In Web open its source in the
editor and bind matching files through Circuit details. Bodies are never uploaded or embedded
in spec/results exports. Without those files a model-dependent study is not self-contained.

For publication or transfer, [create a portable research package](portable-research-packages.md).
It copies permitted dependencies, records prohibited dependencies precisely, and places the
complete result, numeric tables, plot, report, hashes and rerun instructions in one new folder.

## Reading and exporting evidence

All cases remain in the denominator: `passed`, `failed`, `error`, `cancelled`, `pending`,
or `completed` without constraints. Inspect these counts and individual values, not just a
rounded success percentage. Full Results JSON (`kessetsu.experiment-results.v1`) retains
conditions, provenance, datasets and resume records. Summary CSV has case/measurement rows;
full numeric CSV has case/analysis/axis/signal/real/imaginary values, including complex AC.

SVG is a white-paper voltage/current overlay with labeled axes and conditions. AC uses
magnitude on log frequency; full complex data remains in JSON/CSV. Only drawing points are
decimated, deterministically preserving extrema. Printable HTML can be printed to PDF using
your browser. Keep full numerical evidence and matching local models alongside published plots.
