# Changelog

All notable changes are documented here. Kessetsu follows Semantic Versioning for release artifacts and versions its machine contracts independently.

## 0.1.0 — 2026-08-13

First public-release candidate.

### Product

- One Rust Core shared by native CLI and WebAssembly.
- Zero-account Web Hub with live compile/ERC, canonical schematic, real browser Ngspice simulation, plots, assertions, seven export formats and versioned share URLs.
- Agent-oriented CLI with stdin, `kessetsu.cli.v1` JSON, stable diagnostics/exit codes and safe artifact writes.
- OP, transient, AC and DC sweep datasets; engineering metrics for gain, cutoff/bandwidth, phase/frequency, output power, efficiency, THD, clipping and dissipation.
- Typed builtin/user/package models with exact versions, hashes, licenses and lockfile provenance.
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

This is the first versioned release, so there is no earlier public schema to migrate. Prototype users should note that `render` is now functional, editable export uses `kess export --target ...`, compile reports are `kessetsu.compile.v3`, and browser share links require `kessetsu.share.v1`; unknown older payloads fail closed.
