# Changelog

All notable changes are documented here. Kessetsu follows [Semantic Versioning](https://semver.org/) for product releases and versions its machine contracts independently.

## [Unreleased]

### Distribution

- Guided CLI installation page with OS selection, copyable commands, first-circuit download, simulator prerequisites, updates, and manual fallback.
- Per-user Windows and POSIX bootstrappers that select official stable releases, verify archive/executable integrity and version, configure PATH, preserve previous bundles, and refuse unmanaged paths. The CLI binary remains the independently released stable version.

### Fixed

- LTspice export emits dotted analysis directives and correct DC-source references. The first analysis is active; additional analyses remain selectable comments with an explicit warning. Target-application smoke compares pin connectivity and component data rather than reference presence alone.

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
