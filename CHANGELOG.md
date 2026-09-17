# Changelog

All notable changes are documented here. Kessetsu follows [Semantic Versioning](https://semver.org/) for product releases and versions its machine contracts independently.

## [Unreleased]

### Parameterized excitation and analyses

- Accept `{expression}` in all numeric slots of unquoted `ac`, `sine`, `sine_ac`,
  `pulse` and `pwl` voltage/current sources, with lexical module parameter binding.
- Accept expressions for transient step/stop, AC points/start/stop and DC start/stop/step.
  Keep scale keywords/source names literal and require positive integral AC point counts.
- Resolve typed values directly into IR; retain waveform-field and optional sorted
  analysis-field provenance without changing literal backend output.

These changes are in source only, not in published 1.1.0 or the live Web Hub.

### CLI output protection

- Check netlist and model-lock destinations before either write. Reuse byte-identical
  locks without rewriting; require `--force` for different existing lock contents.
- Reject source/lock and netlist/lock destination aliases even with `--force`.

This repair is in source only, not in published 1.1.0 CLI downloads.

### Explicit AC measurements

- Added `gain_at(out,in,frequency)`, `lower_cutoff(out,in[,reference_frequency])`
  and `upper_cutoff(out,in[,reference_frequency])` to the shared CLI/Web evaluator.
- Interpolate magnitude along log frequency without extrapolation; require explicit
  references for ambiguous disjoint bands and report missing edges as errors.
- Measurement contract v2 adds these metrics without changing legacy metrics or the
  CLI/assertion/requirements envelopes. Editor completion and hover describe the new calls.

These changes are in source only, not in published 1.1.0 downloads or the live Web Hub.

### Schematic geometry and editable exports

- Verify actual conductive paths, pin contacts, isolated crossings and matching semantic
  labels, rather than trusting wire net tags. Invalid drawing geometry blocks export.
- Preserve pin escape space, separate crowded components and keep reference/value blocks
  close to their symbols. Negative supply markers and their captions point outward.
- Map LTspice symbols to native rotations/mirrors and reroute from their actual pins;
  preserve orthogonal paths and validate the resulting geometry before export.
- Place KiCad reference/value properties from the shared schematic annotations and draw
  conductive joins explicitly. Added an original dense-bias regression circuit.

These repairs are in source only, not in published 1.1.0 downloads or the live Web Hub.

### Builtin diode portability

- Corrected `1N4148`, `1N4007` and the default diode path to use portable Ngspice diode
  directives without descriptive or non-enforced rating fields. Electrical coefficients
  are unchanged; repaired builtin models have provenance version `1.0.1` and new content
  hashes, retaining their legacy/unverified attribution. Exact model packages are unchanged.
- Added real native/browser OP regression coverage for both named models and the default.

This repair is not yet included in published downloads or the live Web Hub.

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

These changes are in source only, not in published 1.1.0 downloads or the live Web Hub.
Waveform/analysis/assertion expressions and CLI parameter overrides are not yet supported.
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
