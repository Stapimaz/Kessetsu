# Changelog

All notable changes are documented here. Kessetsu follows [Semantic Versioning](https://semver.org/) for product releases and versions its machine contracts independently.

## [Unreleased]

### Typed external model parameters

- External subcircuits may expose a bounded, unit-typed allowlist of existing `.SUBCKT`
  parameters; instances can bind literals or root parameter expressions without editing model text.
- Resolved per-instance values and provenance remain in Circuit IR and survive canonical SPICE,
  LTspice, KiCad and Schematic JSON export. Unknown names, unit mismatches, duplicates and
  parameters absent from the exact hash-bound library fail closed.
- Compile reports advance to `kessetsu.compile.v6`, canonical schematics to
  `kessetsu.schematic.v3`, and model manifests/locks to v3; v4/v5 source shares remain
  explicitly supported.

### Research-data foundation

- Local `kess data preview/import/compare` and shared Core/WASM contracts for explicitly
  mapped CSV, engineering units, calibration, raw-source identity and missing-row accounting.
- Scalar reference comparison with explicit windows/shifts, interpolation and coverage;
  full residuals and sample-weighted bias, MAE, RMSE and maximum error.
- Local Web research-data workspace for CSV preview, explicit mapping, two-dataset overlay,
  residual metrics and evidence download without changing the open circuit or uploading data.
- Typed projection of the current successful transient, AC-magnitude or DC simulation into
  the same research-data comparison flow, retaining solver/result/vector provenance.
- Native `--include simulation` evidence and `kess data from-simulation` close Web/CLI
  projection parity without moving interpolation or residual semantics into adapters.
- Optional local Python package for safe CLI invocation, exact contract validation, typed
  simulation/study/research tables and pandas conversion; includes a headlessly verified
  Jupyter workflow from circuit simulation through residual evidence and plots.
- Synthetic tutorial files and detailed [research-data guide](docs/guides/research-data.md).
  Direct multi-case study comparison and fitting are not yet delivered.

### Local parameter studies

- Shared versioned study specifications and deterministic list/linear/log grids, revisions,
  temperatures, nominal/corner and seeded uniform/Gaussian tolerance studies.
- Native `kess study create/plan/run/export`, durable per-case checkpoints and cancellation/resume
  guarded by source, requirements, models and solver/Core build identity.
- Web Analyze → Parameter study: configure, run, stop/resume, inspect failures, compare reports,
  apply parameters and download full JSON/numeric CSV, summary CSV, SVG or HTML reports.
- Finite-grid objectives retain all outcomes and choose only feasible evaluated candidates;
  no global-optimum, hardware-validity or production-yield claims.
- Explicit-window rise/fall/settling time, directional overshoot and signed integrated energy;
  measurement contract advances to `kessetsu.measurement.v3`. Legacy metrics retain semantics.
- Explicit budgets and detailed [study guide](docs/guides/parameter-studies.md).

## [1.2.0] — 2026-09-18

### Website and documentation

- Simplified landing copy and route-specific loading messages.
- Refreshed landing schematic from the shared Core instead of a hand-drawn mockup.
- On-site changelog, documentation navigation and section index, plus a complete Web editor guide.
- Removed internal roadmap, historical evaluation reports and operational notes from current
  public source and new release packages; technical test specifications remain test inputs.
- Clear Windows installation/session guidance with a directly usable launcher path.

### Compatibility

Existing literal circuits continue to work. Upgrade CLI and Web usage to 1.2.0 for parameters,
new AC metrics and browser local models. Compile reports now use `kessetsu.compile.v5` and
measurements `kessetsu.measurement.v2`; CLI, assertions and requirements envelopes are unchanged.
Existing v4 source-share links are recompiled; unknown future schemas are rejected.
Independent `.kessreq` limits remain literal. Local model files must be supplied separately;
the official PSpice OPA197 model remains native-only. Model identity is not hardware validation.

### Local external devices and model files

- Catalog-backed comparator and two-terminal external subcircuits, instantiated with
  `device`; shared typed pins, ERC, schematic and all seven export formats.
- Explicit local model selection in Web Circuit details. Exact file/hash/entry validation,
  portable Ngspice library checks and native-only compatibility diagnostics before execution.
  Files stay in memory, never in drafts, share URLs, manifests or ordinary model-body exports.
- Generic comparator and published threshold-memristor examples, with exact files/defaults,
  attribution/license and electrical comparison coverage. Existing official OPA197 stays
  user-acquired/native-only, not redistributed or substituted.
- Optional transient `uic` for model initial conditions; ordinary analyses retain their
  previous serialization and behavior. External header defaults/continuations are supported.
- Fix repeated primitive assertion limits producing duplicate Ngspice measurement records;
  each assertion retains its own limit and evaluation.


### Reusable circuit examples

- Add complete RC filter and non-inverting amplifier examples with independently configured
  instances, fixed simulation assertions and access through File → Examples in Web.
- Preserve structured module-instance paths on virtual IR interfaces, including literal blocks
  without parameter provenance; electrical IDs and flattened backend topology remain unchanged.
- Document ports versus parameters, defaults, loading, model assumptions and portable
  source-embedded block identity in the cookbook. No registry or external block imports.


### Parameter diagnostics and compilation limits

- Preserve actual declaration/module-input positions through nested elaboration without
  changing default provenance or guessing parameter locations from similarly named comments.
- Bound source size, parsed/expanded statements and expression work; share node accounting
  across numeric roles and reject unsupported nested waveform calls without parser recursion.
- Preserve supported literal/quoted waveforms, existing source examples and backend semantics.


### Reproducible parameter inputs

- Add shared versioned root numeric inputs and repeatable CLI `--param NAME=VALUE` for
  circuit checking, compilation, simulation, tests and exports, without editing source files.
- Recalculate dependent defaults, reject unknown/duplicate/invalid inputs before writes
  or simulation and retain original defaults/effective override provenance in typed IR.
- Return portable effective `.kess` through `--include effective-source`; expose the same
  pure input contract in WASM without adding persistent editor override state.
- Preserve line endings/comments during materialization and allow trailing `//` comments
  without consuming a statement's required line ending.


### Parameterized inline measurements

- Accept `{expression}` in design-owned assertion thresholds and supported numeric
  metric arguments: time windows, target/reference frequency, THD fundamental and clipping rails.
- Resolve typed IR quantities before simulation; preserve literal signal/device/policy names,
  existing measurement semantics and sorted assertion-field provenance.
- Keep independent `.kessreq` numeric fields literal-only, with explicit expression rejection.


### Parameterized excitation and analyses

- Accept `{expression}` in all numeric slots of unquoted `ac`, `sine`, `sine_ac`,
  `pulse` and `pwl` voltage/current sources, with lexical module parameter binding.
- Accept expressions for transient step/stop, AC points/start/stop and DC start/stop/step.
  Keep scale keywords/source names literal and require positive integral AC point counts.
- Resolve typed values directly into IR; retain waveform-field and optional sorted
  analysis-field provenance without changing literal backend output.


### CLI output protection

- Check netlist and model-lock destinations before either write. Reuse byte-identical
  locks without rewriting; require `--force` for different existing lock contents.
- Reject source/lock and netlist/lock destination aliases even with `--force`.


### Explicit AC measurements

- Added `gain_at(out,in,frequency)`, `lower_cutoff(out,in[,reference_frequency])`
  and `upper_cutoff(out,in[,reference_frequency])` to the shared CLI/Web evaluator.
- Interpolate magnitude along log frequency without extrapolation; require explicit
  references for ambiguous disjoint bands and report missing edges as errors.
- Measurement contract v2 adds these metrics without changing legacy metrics or the
  CLI/assertion/requirements envelopes. Editor completion and hover describe the new calls.


### Schematic geometry and editable exports

- Verify actual conductive paths, pin contacts, isolated crossings and matching semantic
  labels, rather than trusting wire net tags. Invalid drawing geometry blocks export.
- Preserve pin escape space, separate crowded components and keep reference/value blocks
  close to their symbols. Negative supply markers and their captions point outward.
- Map LTspice symbols to native rotations/mirrors and reroute from their actual pins;
  preserve orthogonal paths and validate the resulting geometry before export.
- Place KiCad reference/value properties from the shared schematic annotations and draw
  conductive joins explicitly. Added an original dense-bias regression circuit.


### Builtin diode portability

- Corrected `1N4148`, `1N4007` and the default diode path to use portable Ngspice diode
  directives without descriptive or non-enforced rating fields. Electrical coefficients
  are unchanged; repaired builtin models have provenance version `1.0.1` and new content
  hashes, retaining their legacy/unverified attribution. Exact model packages are unchanged.
- Added real native/browser OP regression coverage for both named models and the default.



### Parameterized circuit foundation

- Independent module parameter scopes and named `use` overrides, with recalculated
  defaults, typed caller binding and structured instance-path/override provenance.
- Declared module ports in IR/ERC, scoped local nets and flattened-path collision errors.
  Virtual module ports are excluded from physical schematic-pin expectations, without
  weakening physical connection checks.

- Top-level typed `param` declarations and bounded, unit-checked arithmetic in passive and DC-source values.
- Parameter/dependency and resolved-field provenance in `kessetsu.parameters.v1`; all electrical and drawing backends still consume resolved Circuit IR.
- Source completion/highlighting without new editor panels; compile contract v5 with explicit v4 source-share compatibility and unchanged exact-package checks.
- Prefix/exponent overflow/underflow validation and bounded module recursion/expansion diagnostics.

Parameterized module analyses/assertions must be placed at the circuit root until their
context/target handling is implemented.

## [1.1.0] — 2026-09-16

### Circuit tools

- Shared nominal divider/RC calculations with typed SI inputs, exact/E12/E24 values, achieved results and editable circuit templates.
- `kess tool divider` and `kess tool rc-lowpass` with the existing JSON envelope and explicit `.kess` output.
- Local browser tools at `/tools/`, with generated-circuit download, editor continuation and recovery of the previous browser circuit.
- Discoverable toolkit links on desktop/mobile landing pages; simulation without assertions shows completion instead of a misleading 0/0 requirements summary.

### Web and discovery

- Build-time HTML for the existing landing and installation pages, with distinct route titles and descriptions.
- Public guides and references at `/docs/`, generated from the reviewed Markdown sources without a JavaScript requirement; sitemap covers all public content pages.
- GitHub project description, topics and canonical product homepage updated. No analytics or circuit uploads added.

## [1.0.1] — 2026-09-16

### Distribution

- Guided CLI installation page with OS selection, copyable commands, first-circuit download, simulator prerequisites, updates, and manual fallback.
- Per-user Windows and POSIX bootstrappers that select official stable releases, verify archive/executable integrity and version, configure PATH, preserve previous bundles, and refuse unmanaged paths. The CLI binary remains the independently released stable version.

### Fixed

- LTspice export emits dotted analysis directives and correct DC-source references. The first analysis is active; additional analyses remain selectable comments with an explicit warning. Target-application smoke compares pin connectivity and component data rather than reference presence alone.
- Windows ZIP packaging uses portable slash-separated entries on both Windows PowerShell 5 and PowerShell 7, preserving strict installer path validation.

### Documentation

- Concise public roadmap and documentation index, with account-specific execution prose removed from published reports while preserving evaluation outcomes and limitations.
- Reviewed public-document manifest and focused publication checks for future release packaging.

## [1.0.0] — 2026-09-16

First public release.

### Product

- One Rust Core shared by native CLI and WebAssembly.
- Zero-account Web Hub with live compile/ERC, canonical schematic, real browser Ngspice simulation, plots, assertions, seven export formats and versioned share URLs.
- Local Web document workflow with `.kess` open, native in-place Save and Save As on Chromium, browser-local Save plus explicit `.kess` download on Firefox and other fallback browsers, explicit naming, deterministic export filenames and versioned draft recovery.
- Project-level actions stay in the global toolbar, while the Simulation panel owns Run/Cancel, execution state, plots and requirement results.
- Selected lowercase Kessetsu wordmark, signal-line identity and matching favicon with a self-hosted, license-bundled Inter glyph subset.
- Agent-oriented CLI with stdin, `kessetsu.cli.v1` JSON, stable diagnostics/exit codes and safe artifact writes.
- Evaluator-owned `kessetsu.requirements.v1` files with exact-byte SHA-256 provenance and optional pinning for supervised agent/CI loops.
- Fail-closed test semantics: zero assertions cannot pass, unknown metrics fail before simulation, and ambiguous repeated analyses are rejected without breaking distinct DC-source sweeps.
- Complete typed PWL source parsing with dimensional values and non-negative, strictly increasing time points.
- OP, transient, AC and DC sweep datasets; engineering metrics for gain, cutoff/bandwidth, phase/frequency, output power, efficiency, THD, clipping and dissipation.
- Typed builtin/user/package models with exact versions, hashes, licenses and lockfile provenance.
- Typed, hash-bound external op-amp subcircuit references for native CLI simulation, including safe source-relative resource binding, PSpice compatibility, deterministic lock/provenance metadata, and explicit no-embedding export behavior.
- Deterministic Schematic IR with connectivity and visual-quality gates.
- SVG, PNG, PDF, Schematic JSON, SPICE, KiCad and LTspice export.
- RC, gain-stage and four-stage 8 Ω / approximately 2 W power-amplifier benchmark parity across CLI and Web.

### Distribution

- Windows x86-64 bundle with Ngspice 46 sidecar.
- Linux x86-64, macOS x86-64 and macOS arm64 CLI bundles with verified system-Ngspice discovery.
- Absolute bundled/explicit/PATH simulator provenance and a portable Ngspice MOS1 `KESSETSU_PMOS_V1@1.0.1` directive verified across supported runners.
- Per-artifact SHA-256, release manifest and clean-machine simulation smoke workflow.
- AGPL-3.0-only public license with a separately negotiated commercial-license path; license/source notices are bundled in CLI and Web distributions.

### Migration

This is the first versioned release, so there is no earlier public schema to migrate. Prototype users should note that `render` is now functional, editable export uses `kess export --target ...`, compile reports are `kessetsu.compile.v4`, and browser share links require `kessetsu.share.v1`; unknown older payloads fail closed.
