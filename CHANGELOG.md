# Changelog

All notable changes are documented here. Kessetsu follows [Semantic Versioning](https://semver.org/) for product releases and versions its machine contracts independently.

## [Unreleased]

Changes merged after the first public release will be recorded here.

## [1.0.0] — 2026-09-14

First public-release candidate.

### Product

- One Rust Core shared by native CLI and WebAssembly.
- Zero-account Web Hub with live compile/ERC, canonical schematic, real browser Ngspice simulation, plots, assertions, seven export formats and versioned share URLs.
- Local Web document workflow with `.kess` open/save, explicit naming, deterministic export filenames and versioned browser draft recovery.
- Selected lowercase Kessetsu wordmark, signal-line identity and matching favicon with a self-hosted, license-bundled Inter glyph subset.
- Agent-oriented CLI with stdin, `kessetsu.cli.v1` JSON, stable diagnostics/exit codes and safe artifact writes.
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
