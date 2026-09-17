# Kessetsu Roadmap

This is the public product roadmap, not a release promise or an internal execution log.
Last updated: 2026-09-17.

## Released

- [x] **1.0.0:** shared Rust Core, native CLI and no-account browser workspace.
- [x] **1.0.1:** corrected LTspice analyses/DC-source references and portable Windows ZIP packaging.
- [x] **1.1.0:** shared CLI/Web [circuit tools](https://kessetsu.com/tools/): loaded voltage
  divider and RC low-pass calculations, Exact/E12/E24 component values and editable
  circuit source that continues into simulation and schematic export.
- [x] Typed circuit definitions, deterministic SPICE, semantic validation and ERC.
- [x] Ngspice simulation, engineering measurements and executable assertions.
- [x] Separate evaluator-owned `.kessreq` requirements with optional hash pinning.
- [x] Canonical schematics and SVG, PNG, PDF, JSON, SPICE, KiCad and LTspice exports.
- [x] Native typed references to hash-verified external op-amp subcircuits.
- [x] Windows x86-64, Linux x86-64 and macOS Intel/Apple Silicon CLI packages.
- [x] Browser editor, local saving/downloads, simulation plots and source-based sharing.
- [x] Guided [installation](https://kessetsu.com/install/) with verified per-user installers and update instructions.
- [x] Crawlable landing/install HTML and [public guides](https://kessetsu.com/docs/) generated from reviewed sources, with per-page metadata and sitemap coverage.
- [x] Separate public product documentation from internal planning and operational notes.

See the [changelog](../CHANGELOG.md) and [releases](https://github.com/Stapimaz/Kessetsu/releases)
for shipped changes. Website and installation tooling can evolve independently of CLI binaries;
published binary releases and tags remain immutable.

## In development

- [ ] CLI model-lock output protection: require explicit replacement of differing locks,
  reuse identical contents and reject source/netlist destination collisions. Implemented
  and regression-checked in source; not yet in published 1.1.0 downloads.

- [ ] Frequency-specific AC gain and independent lower/upper half-power edges, with
  explicit band references and fail-closed ambiguous-band handling. Shared CLI/Web
  implementation is regression-checked in source, including native/browser numerical
  parity; not yet in published 1.1.0 or the deployed Web Hub.

- [ ] Schematic repairs for dense circuits: coordinate-based connectivity verification,
  safer routing, closer component annotations and orientation-aware EDA export. Implemented
  and regression-checked in source; not yet in published 1.1.0 or the deployed Web Hub.
- [ ] Builtin diode portability repair, verified in native/browser source tests but not
  yet included in published downloads or the deployed Web Hub.
- [ ] Named quantities and reusable parameterized circuits. The first source-only slice
  supports typed top-level/module parameters, caller-scoped instance overrides and
  passive/DC-source/waveform expressions and numeric analysis settings; it is not in
  published 1.1.0 binaries or the deployed Web Hub. Assertion numeric expressions and
  root CLI overrides remain in development.

## Under consideration

These are candidate directions, not committed delivery dates. Priorities will follow concrete
user needs and reproducible engineering evidence.

- Broader useful model and reusable-subcircuit coverage, with explicit provenance.
- Better EDA interchange and component/footprint mapping.
- Verification across operating conditions, parameter sweeps and additional measurements.
- Easier integration with external AI agents and automation.
- Improved onboarding, accessibility and workflows based on newcomer feedback.

CLI and no-account browser use remain free core surfaces. Accounts, embedded AI and PCB design
are not current product capabilities.

## Evidence and boundaries

The bounded [six-task comparison](evals/unseen-design-summary-2026-09-13.md) demonstrated
repeatable automatic-artifact and interchange benefits, not universal electrical-success or
speed superiority. It used one model; cross-model and hardware validation remain untested.
The original external-model limitation and its versioned follow-up remain documented.

The [supported domain](reference/supported-domain.md), [measurement contract](reference/measurements.md)
and [export reference](reference/exports.md) define current support and limitations.
Architecture rules live in [architecture.md](architecture.md).
