# Kessetsu Development Roadmap

> This document is the **single source of truth** for Kessetsu development status, the active milestone, acceptance gates, and task order.
>
> Use `docs/architecture.md` for architectural rules and `docs/cli_reference.md` for the public CLI contract. If these documents conflict, this roadmap governs development status and the conflict must be resolved in the active milestone.
>
> Last comprehensive repository audit: **2026-08-14**
>
> Active milestone: **Phase 4 — Professional Schematics, Web Hub, and Release**
>
> Previous milestone: **Phase 3 — Simulation and Assertion Runtime (complete)**

This English edition was consolidated on 2026-08-20. Repetitive evidence from completed historical phases was compressed into dated closure records, while current scope, acceptance criteria, diagnostic contracts, run identifiers, and all open release gates remain authoritative. Line-level historical detail remains available in Git history.

---

## 1. Product Identity

**Kessetsu** is an agent-driven circuit-engineering platform. It turns textual circuit definitions into typed Circuit IR, deterministic SPICE, simulation and measurement results, structured verification feedback, professional schematics, and editable EDA artifacts. Humans and AI agents can iteratively develop a circuit from measurable electrical requirements.

```text
Compile, simulate and test circuits like software.
```

### Product North Star

Kessetsu is not limited to education, a particular user level, or one circuit family. A human or AI agent should be able to translate design requirements into measurable constraints and assertions, revise topology, values, and models, and receive reliable structured feedback at every step.

```text
Electrical requirements
    → circuit/topology candidate
    → compile + semantic validation + ERC
    → simulate + measure + assert
    → structured engineering feedback
    → topology/value/model revision
    → verified design that satisfies the requirements
    → high-quality schematic and EDA exports
```

Kessetsu does not need to embed an AI model. Its first responsibility is to serve any capable external AI agent through the versioned CLI JSON contract as a design oracle, simulation engine, and verification tool. A separate Agent API service is not required; future SDK or MCP adapters should remain thin consumers of the same Core and CLI contracts.

The first strong vertical is analog and mixed-signal, SPICE-based design. The long-term architecture is not constrained to classroom examples or one industry. Unsupported physical domains and simulator limitations must be explicit. An unverified design must never be presented as verified.

### Development and Publication Model

Kessetsu is developed agentically with LLM agents. Phases are evidence gates and dependency order, not calendar estimates or early-MVP release slices. Conventional development-time assumptions are not a reason to narrow the product vision.

The repository remains private during integrated development. The CLI/agent surface, simulation and assertion runtime, Web Hub, professional schematics, and target EDA exports must satisfy their acceptance gates together before public release. There is no external-user deadline driving premature publication.

### Strategic Reality Check — 2026-09-10

**Direction accepted by the owner on 2026-09-10:** Continue development around a provider-independent circuit design, verification, and artifact workflow. Validate its incremental value before publication. Stronger agents can use existing simulators and EDA tools directly; a custom language, JSON output, or AI-generated schematic is not by itself a defensible advantage. The developer owns routine planning and implementation; comparative product value and market demand remain unproven.

The existing Rust Core, shared CLI/Web semantics, executable requirements, and readable exports are useful assets. Their combined convenience and correctness must be demonstrated against a capable agent using ordinary tools, not against an unaided language model. Keep the no-account browser workspace and locally usable CLI central. Do not introduce a PCB engine or embedded chat solely because competing products offer them.

**Evidence limitations found in this review:**

- `scripts/replay-agent-eval.ps1` derives a failing candidate from the finished amplifier by changing its load from 8 ohms to 16 ohms, then restores it. The JSON record documents this derivation and only a broad model family. This is useful regression evidence, not an independent, unseen requirements-to-design evaluation.
- Three canonical simulation benchmarks and thirteen schematic fixtures establish bounded coverage, not general circuit-design or arbitrary-layout quality.
- The model boundary accepts allowlisted discrete-device parameters and one op-amp template. It does not import arbitrary manufacturer IC/subcircuit models. Passing generic-model assertions does not establish that a purchasable part or physical board meets the same requirements.
- In `core/src/models.rs`, the op-amp template declares `vcc`/`vee` but does not use them internally; it implements a controlled source and an RC pole. Supply-current accounting, rail saturation, and realistic output-current limits are not represented by that template. The amplifier's reported efficiency and clipping must be interpreted within this restricted model, not as full hardware validation.
- Assertions currently live beside the candidate source. A design agent could weaken them or alter a fixed load/supply. Product evaluations must keep the requirement oracle outside the agent's editable candidate.
- August verification records remain historical evidence. This review is not a new comprehensive implementation audit or a new competitive benchmark.

**Market evidence checked on 2026-09-10:** [GPT-6 Astra](https://developers.openai.com/api/docs/models/gpt-6-astra) and [Claude Fable 5.1](https://www.anthropic.com/claude/fable) document stronger agent capabilities, but these pages do not establish general PCB correctness. [Flux](https://www.flux.ai/p/blog/simulate-circuits-with-a-prompt) already describes AI-driven SPICE simulation and specification-based iteration. [tscircuit](https://docs.tscircuit.com/) documents code-based schematic, PCB, simulation, and manufacturing workflows. [Quilter](https://docs.quilter.ai/using-quilter/introduction) automates placement/routing and validation from a schematic and starter board. These are documented capabilities, not comparative hands-on results. AI plus simulation is already a competitive category.

**Decision rule:** Continue the integrated product if new tasks show a repeatable benefit in correctness, user effort, or usable artifacts. If most useful tasks are blocked by model/interchange limits, prioritize those limits over UI expansion. If the complete workflow adds no repeatable benefit but rendering or verification does, evaluate packaging that subsystem as an interoperable tool. Reconsider the standalone product if neither provides value. No outcome should be assumed before the comparison.

### Product Surfaces

| Surface | Primary audience | Responsibility |
|---|---|---|
| Kessetsu Core | Entire system | Parser, AST, typed IR, graph, ERC, simulation, measurements, schematic layout, exporters |
| Kessetsu CLI, human + JSON | AI agents, automation, developers | Compile, measure, verify, revise, and export through stable contracts |
| Kessetsu Web Hub | Anyone wanting a no-install experience | Edit, verify, simulate, inspect, export, and share using the same Core |

### First-Class Outputs

- Structured engineering diagnostics and measurements
- Assertion results with `PASS`, `FAIL`, `ERROR`, and `SKIPPED`
- Deterministic SPICE and typed simulation datasets
- Readable, connectivity-verified schematics
- SVG, PNG, and PDF visual exports
- Schematic IR JSON and canonical SPICE
- Editable KiCad and LTspice schematics
- Identical Core semantics across CLI and Web

### Identity and Release Decisions

- [x] Set the product and repository name to **Kessetsu**, the canonical CLI command to `kess`, and the source extension to `.kess`.
- [x] Complete the unpublished-project identity migration without legacy aliases. _Migration record: `docs/kessetsu_migration.md`; repository: `Stapimaz/Kessetsu`._
- [x] Keep the embedded Web AI/chat experience outside the Phase 4 critical path; record it as a provider-independent Phase 5+ direction.
- [x] Keep schematics and EDA exports in the shared Core rather than making them Web-only features.
- [x] Define the first release as one integrated product rather than publishing disconnected subsystems early.

---

## 2. Status and Evidence Model

- `[x]`: Implemented, acceptance criteria verified, and relevant quality gates passed.
- `[ ]`: Not implemented or not yet proven.
- `PROTOTYPE`: Working code exists, but its API, tests, or failure handling are incomplete.
- `DEFERRED`: Intentionally moved to a later milestone and not a blocker for the current phase.

A section is complete only when all required subtasks and acceptance criteria are proven. Code presence alone is not evidence. Completion notes should cite tests, fixtures, commands, diagnostics, schemas, or dated decision records whenever possible.

### Non-Negotiable Architectural Rules

1. Circuit IR is the only backend source. ERC, SPICE, layout, exporters, and structured output never bypass it for the AST.
2. Parser, IR, graph, ERC, SPICE, layout, and frontend consumers share the canonical component catalog.
3. Schematic checks are called ERC, not DRC.
4. The same canonical circuit must produce deterministic node names and byte-stable SPICE.
5. User-named nets override automatic names; ambiguity produces a diagnostic.
6. Invalid values fail closed and never become silent zeroes, empty models, or malformed SPICE.
7. New syntax must not break valid `examples/*.kess` without an explicit migration decision.
8. Warnings and errors are distinct; only errors block downstream output.
9. Generated build and test artifacts do not enter source control.
10. A milestone closes through evidence: format, lint, tests, builds, runtime smoke, and relevant product review.

---

## 3. Completed Foundations — Phases 0–2.5

### Phase 0 — Historical MVP

The initial parser, AST, module flattening, component vocabulary, basic graph/ERC, SPICE generation, CLI shell, Ngspice sidecar, and Web prototype were created. Their prototype behavior was not accepted as a final contract; Phase 2.5 characterized and replaced unsafe paths.

### Phases 1–2 — Characterization and Typed Core

The intended golden/regression and typed-IR work was completed as part of Phase 2.5. Historical phase numbering is retained for traceability, but Phase 2.5 is the evidence-bearing closure.

### Phase 2.5 — Stabilization and Hardening

**Goal:** Prove that the Phase 0–2 feature set is reproducible from a clean clone, deterministic, fail-closed, regression-tested, and consistent across CLI, WASM, and Web.

#### 2.5.0 — Roadmap and Status Contract

- [x] Remove the incorrect early Phase 3 completion status and define evidence-based milestone states.
- [x] Audit the repository and align `architecture.md`, `cli_reference.md`, and this roadmap with real behavior.
- [x] Separate later datasheet-limit and confidence work from the simulation-runtime milestone.

#### 2.5.1 — Repository and Build Hygiene

- [x] Add the root `.gitignore` and remove generated `core/target`, debug output, WASM bootstrap files, and `examples/*.spice` from tracking.
- [x] Define `examples/` as user-facing `.kess` source and `core/tests/fixtures/golden/` as the tracked regression oracle.
- [x] Make `npm run build:wasm` the single documented WASM package command and make the production Web build regenerate WASM rather than relying on stale local output.
- [x] Reduce the Windows Ngspice sidecar to the documented minimum analog runtime and retain its license/version notice in `core/tools/ngspice/README.md`.
- [x] Add canonical root verification through `scripts/verify.ps1`.
- [x] Pin the supported toolchain: Rust `1.97.1`, Node `24.15.0`, CI wasm-pack `0.13.1`.
- [x] Add Rust format, Clippy, tests, release, WASM, Web lint/build, audit, and worktree-cleanliness gates to CI.
- [x] Remove unused dependency chains, pin patched transitive packages, and reach zero known npm production vulnerabilities without `audit fix --force`.

**Remote evidence:** CI run `31282475618` passed on `db24ee3` on 2026-08-09.

#### 2.5.2 — Characterization and Regression Infrastructure

- [x] Create isolated Rust integration-test and fixture structure under `core/tests/`.
- [x] Cover valid, invalid, and golden source corpora, including every repository example.
- [x] Characterize empty/comment-only input, UTF-8 BOM rejection, Unicode comments versus ASCII identifiers, CRLF/tabs, malformed syntax, and legacy syntax rejection.
- [x] Cover module/use/port flattening and deterministic parser failures.
- [x] Cover SI quantities, source waveforms, model resolution, assertions, physical units, scientific notation, and fail-closed invalid values.
- [x] Cover graph order independence, user-named nets, ground policy, namespace conflicts, invalid pins, all `KES-E001..009` diagnostics, and deterministic diagnostic ordering.
- [x] Add byte-stable SPICE goldens for minimal, demo, Wheatstone, feature, current-source, waveform, named-net, model, assertion, and disconnected-pin behavior.
- [x] Add isolated CLI contract tests for human/JSON output, I/O, parse, semantic/ERC, simulator, assertion, overwrite, and exit-code behavior.

#### 2.5.3 — Typed IR and Semantic Validation

- [x] Replace permissive SI parsing with typed quantities that validate numbers, prefixes, and physical dimensions separately.
- [x] Support decimal, negative, and scientific notation while rejecting unsupported trailing text.
- [x] Remove fail-open `unwrap_or(0.0)` conversions and make missing passive values errors.
- [x] Represent voltage and current sources with distinct typed parameters and typed DC/waveform variants.
- [x] Validate waveform arity and units.
- [x] Resolve built-in models with typed kind/provenance; reject unknown models and polarity/kind mismatches through `KES-C003` and `KES-C004`.
- [x] Prevent components without a usable model from producing invalid SPICE.
- [x] Validate assertion signals, comparators, thresholds, and units.
- [x] Serialize typed IR and semantic diagnostics for CLI and WASM.

#### 2.5.4 — Deterministic Graph and ERC

- [x] Replace the `9999` disconnected-pin sentinel with `Option<NetId>`.
- [x] Centralize component pins, SPICE order, prefixes, geometry, and signal/through metadata in `core/src/component.rs`.
- [x] Add invalid-pin, component/net collision, user-name conflict, duplicate-net, and ground-ambiguity diagnostics.
- [x] Give explicit GND priority and retain deterministic lexicographic legacy fallback with an ambiguity error.
- [x] Canonically order traversal, net IDs, diagnostics, component emission, and model injection.
- [x] Use one platform-independent SPICE-number formatter.
- [x] Prove byte-for-byte determinism through repeated compilation stress tests.

#### 2.5.5 — Single Compile Pipeline and CLI Contract

- [x] Define the side-effect-free `compile_source(source, options) -> CompileReport` entry point.
- [x] Carry schema version, optional AST, typed IR, diagnostics, graph summary, SPICE, layout, and optional exporter output in the report.
- [x] Centralize parser → flatten → IR → graph → ERC → backend order.
- [x] Reduce CLI and WASM to adapters over the same report.
- [x] Prevent downstream output after errors while allowing warnings with successful output.
- [x] Make `--format` genuinely global and keep JSON stdout to one clean object.
- [x] Fix exit semantics: success `0`, semantic/ERC `1`, parse/I/O `2`, simulation `3`, assertion `4`.
- [x] Define safe output and overwrite behavior, including source-overwrite prohibition.
- [x] Make unsupported frontend operations fail closed rather than return misleading success.

#### 2.5.6 — Web/WASM Synchronization

- [x] Move Web default source to the canonical grammar and shared repository fixture.
- [x] Align source terminology, connection syntax, component variants, symbol mapping, layout, and KiCad output with the shared catalog.
- [x] Replace Web `any` boundaries with explicit compile/report types.
- [x] Remove unused Vite template assets and clear React lint warnings.
- [x] Add shared default-circuit compile smoke coverage.

#### 2.5.7 — Documentation and Final Gate

- [x] Replace template documentation with project-specific README, architecture, CLI, Web Hub, and Ngspice runtime documentation.
- [x] Move agent instructions to the root `AGENTS.md` and align them with the canonical quality gate.
- [x] Pass format, Clippy, all Rust/CLI tests, release, WASM, Web lint/build, npm audit, example matrix, invalid corpus, determinism stress, and clean-worktree checks.
- [x] Pass the same canonical `scripts/verify.ps1` flow remotely.

**Phase 2.5 closure — 2026-08-09:** Local and remote canonical verification passed. The Core became typed, deterministic, fail-closed, and shared across CLI/WASM/Web. Phase 3 prerequisites were satisfied.

---

## 4. Phase 3 — Simulation and Assertion Runtime (Complete)

### Goal

Replace prototype subprocess and `.meas` behavior with versioned simulation, dataset, measurement, assertion, model, and agent contracts backed by real Ngspice fixtures.

### 3.1 — Simulation Domain and Runner

- [x] Define backend-neutral `SimulationRequest`, `SimulationResult`, and `SimulationRunner` contracts.
- [x] Represent OP, transient, AC, and DC sweep as typed, unit-checked analyses.
- [x] Separate measurements, warnings, errors, raw logs, and process status.
- [x] Use unique temporary directories with explicit cleanup and failure-retention policy.
- [x] Share one runner between `simulate` and `test`.
- [x] Implement executable discovery, version probing, timeout, cancellation, and parallel-run isolation.

**Evidence:** `kessetsu.simulation.v1`; malformed analysis fails with `KES-C009`; fake-process and bundled Ngspice tests cover success, launch failure, simulator failure, timeout, cancellation, artifact retention, and concurrency.

### 3.2 — Structured Ngspice Results

- [x] Replace stdout-table scraping with fixture-driven `.meas` and deterministic `wrdata` parsers.
- [x] Normalize OP into a sorted scalar map, transient/DC into real series, and AC into frequency plus complex series.
- [x] Classify warning, convergence, fatal, and result-parse failures as `KES-S003..006`.
- [x] Cover exponent, decimal-comma, LF/CRLF, malformed, duplicate, and non-finite output.
- [x] Pass real bundled-Ngspice OP, transient, and AC fixtures locally and in Linux CI.

### 3.3 — Assertion Runtime

- [x] Assign deterministic source-order `KES-Txxx` identities.
- [x] Distinguish `PASS`, `FAIL`, `ERROR`, and `SKIPPED`.
- [x] Replace missing-measurement `NaN` with explanatory errors.
- [x] Define absolute/relative tolerance, absolute-peak, OP, current-sign, and engineering-unit semantics.
- [x] Produce a versioned `kessetsu.assertion.v1` report with summary counts.

### 3.4 — Simulation CLI Contract

- [x] Return structured analysis from `kess simulate` and structured assertions from `kess test`.
- [x] Render human and JSON output from the same domain result.
- [x] Keep default `kessetsu.cli.v1` JSON compact and make AST/IR/graph/SPICE/datasets/models/raw-log opt-in.
- [x] Publish active domain versions and reject unknown schema versions with `KES-F002`.
- [x] Preserve simulation exit `3`, assertion exit `4`, and clean JSON stdout.

### 3.5 — Agent-Ready CLI Contract

- [x] Provide versioned JSON for `check`, `compile`, `simulate`, and `test` without a separate daemon.
- [x] Support file-free source through stdin and avoid filesystem output unless explicitly requested.
- [x] Keep repeated side-effect-free requests byte-stable.
- [x] Verify an external-agent compile → simulate → measure → revise loop without parsing human text.

**Evidence:** A 10 Ω candidate measures 200 mA and fails; the agent consumes structured fields, revises to 100 Ω, measures 20 mA, and passes.

### 3.6 — Models and Subcircuits

- [x] Add typed, allowlisted user declarations for diode/BJT/MOSFET models and a fixed-template op-amp subcircuit.
- [x] Validate subcircuit pin order through the shared component catalog.
- [x] Provide verified op-amp, PMOS, and power-transistor paths.
- [x] Fail closed on kind, polarity, pin, capability, metadata, and case-insensitive namespace conflicts through `KES-C010..013`.
- [x] Define `kessetsu.models.v1` provenance and deterministic `kessetsu.lock.v1` resolution.
- [x] Prevent raw SPICE/control injection across the typed boundary.

### 3.7 — Engineering Measurements and Product Benchmarks

- [x] Define typed node voltage, branch/device current, terminal-pair voltage, and power primitives.
- [x] Define reductions, time windows, frequency/phase semantics, gain, bandwidth/cutoff, output RMS power, efficiency, THD, clipping, and dissipation.
- [x] Make derived measurements assertion-safe and fail closed on missing or incompatible data.
- [x] Document formulas, units, signs, windows, and analysis requirements in `docs/engineering_measurements.md`.
- [x] Verify canonical RC-filter cutoff and AC response.
- [x] Verify gain-stage bias, gain, bandwidth, and clipping behavior.
- [x] Verify a four-stage power amplifier at 8 Ω for output power, gain, THD, clipping, efficiency, device stress, and dissipation.
- [x] Record a reproducible external-agent revision from a failing candidate to a passing design.

**Phase 3 closure — 2026-08-12:** All Phase 3 tasks and acceptance gates passed. The canonical benchmarks use real Ngspice. The power amplifier reaches approximately 1.95 W into 8 Ω, gain 56.5, and THD 2.23% while passing its defined constraints. Compile schema advanced to `kessetsu.compile.v2` during this work and later to `v3` for canonical schematics.

### Deferred from Phase 3

- `DEFERRED` Datasheet voltage/current/thermal rules until a Component Knowledge Base exists.
- `DEFERRED` HIGH/MEDIUM/LOW confidence until a measurable Verification Report exists.
- `DEFERRED` Layout round-trip connectivity to Phase 4 schematic/export work.

---

## 5. Phase 4 — Professional Schematics, Web Hub, and Release

### Goal

Turn the proven Core/CLI into an integrated first public product with a no-account, no-install Web Hub, professional schematics, browser simulation, visual and editable exports, sharing, cross-platform CLI packages, and publication-quality documentation.

Phase 4 is organized as vertical product slices rather than isolated subsystem work. The canonical path is editor → compile/ERC → schematic → simulation → plots/assertions → export, first for RC and then for gain-stage and power-amplifier benchmarks.

### Scope Boundary

**Required:** Professional and verifiable schematics, browser simulation, interactive results, benchmark parity, visual/machine/editable exports, sharing, cross-platform CLI packages, public documentation, production deployment, and release.

**Deferred:** Embedded natural-language AI chat, managed AI providers or user API keys, accounts/cloud projects, team features, automatic optimization/design-space exploration, and a broad manufacturer-model registry.

### 4.0 — Product Contracts, Characterization, and Decisions

- [x] Add a real Chromium smoke test that initializes WASM and compiles canonical source.
- [x] Make Web discover the Core compile schema and fail closed on unknown versions.
- [x] Characterize legacy schematic behavior on minimal, RC, Wheatstone, gain-stage, high-fan-out, and power-amplifier fixtures.
- [x] Record independent reuse/refactor/rewrite decisions for the Web shell, SVG renderer, Schematic IR, and layout engine. _ADR 0001._
- [x] Freeze the first-release supported-domain matrix. _`docs/supported_domain.md`._
- [x] Select RC as the first vertical and the 8 Ω power amplifier as the principal product/evaluation scenario.
- [x] Measure browser-simulation options and select a dedicated Worker around exact `eecircuit-engine` 1.7.0. _ADR 0002._
- [x] Record Circuit IR → Schematic IR → exporter ownership in the architecture constitution.

### 4.1 — Canonical and Verifiable Schematic IR

- [x] Define collection-order-independent, serializable `kessetsu.schematic.v1`.
- [x] Carry component instance, symbol/variant, value/model text, orientation, and canonical pin anchors explicitly.
- [x] Represent wire segments, typed endpoints, named nets, junctions, and disconnected geometric crossings separately.
- [x] Model ground, supplies, and restricted high-fan-out labels as semantic elements.
- [x] Reconstruct canonical graph connectivity from Schematic IR and fail closed with `KES-L001` on mismatch.
- [x] Prove byte stability under repeated builds and declaration/connection reordering.
- [x] Share symbol and pin definitions through the Core catalog; Web only displays Core SVG.
- [x] Replace the legacy fixed-row chain heuristic with deterministic constrained placement and orthogonal cost-based routing.
- [x] Add versioned quality metrics for collisions, crossings, bends, labels, routing, text, flow, compactness, and connectivity.
- [x] Lock deterministic SVG goldens and real-browser corpus rendering.

### 4.1R — Professional Schematic Readability Remediation (Complete)

The first technically passing candidate was reopened after real PNG review exposed poor visual composition. The detailed method is in `docs/schematic_quality_plan.md`; dated evidence is in `docs/evals/`.

- [x] SQ-1 — Build a reproducible thirteen-circuit Core/CLI/Web capture and scorecard harness.
- [x] SQ-2 — Expand hard and soft quality metrics without allowing labels to game connectivity/readability scores.
- [x] SQ-3 — Add typed pin-flow and net-role metadata to the shared catalog.
- [x] SQ-4 — Implement topology-aware stage, bridge, differential, clamp, and transistor placement.
- [x] SQ-5 — Use explicit local signal/feedback wires, trunk/branch routing, and labels only for genuine globals.
- [x] SQ-6 — Refine engineering values, text hierarchy, margins, semantic rail/GND glyphs, viewport fitting, and an optional Web-only dotted grid.
- [x] SQ-7 — Make the power-amplifier path readable as input/buffer → gain/error → driver → complementary output → load.
- [x] SQ-8 — Verify one shared layout across SVG/PNG/PDF/JSON/KiCad/LTspice and Web.
- [x] SQ-9 — Lock accepted scorecards and deterministic hashes; pass full local and remote verification. _Accepted commit `5308f3b`; remote CI run `31850078955`._
- [x] SQ-10 — Obtain explicit owner review of RC, gain-stage, and power-amplifier. _Accepted on 2026-08-20; later defects remain valid regression work._
  - [x] Refine distant component fields through deterministic eighth-grid placement without changing routing.
  - [x] Link Web hover/click highlighting across each symbol and its reference/value/model fields.
  - [x] Replace splayed BJT arrows with polarity-correct filled markers centered on emitter branches.
  - [x] Clear right-side text from rotated symbol overhang and correct schematic/plot pointer-coordinate mapping.

**Accepted visual golden:** `docs/evals/schematic-quality-candidate-2026-08-14.md`, accepted on 2026-08-20. Acceptance freezes a reproducible release baseline; it does not prohibit future improvements or fixes.

### 4.2 — Browser Simulation Runtime

- [x] Implement the browser adapter without breaking the shared simulation-domain boundary.
- [x] Run simulation in a dedicated module Worker, never on the UI thread.
- [x] Normalize OP, transient, AC, and DC sweep results into native `kessetsu.simulation.v1` semantics.
- [x] Evaluate measurements and assertions through the same WASM Core functions used by the native product.
- [x] Define timeout, cancellation, progress, crash/restart, and stale-result suppression behavior.
- [x] Keep raw logs and large datasets opt-in.
- [x] Expose simulator version/provenance and bounded native/browser numeric tolerances.
- [x] Pass real RC native/browser compile, SPICE, dataset, measurement, and assertion parity.
- [x] Verify runtime licenses, integrity hashes, notices, and cache/update policy.

### 4.3 — Zero-Friction Web Hub Vertical

- [x] Split editor, diagnostics, schematic, results, exports, and workspace state into clear responsibilities.
- [x] Start with a working RC example and one obvious `Run` action.
- [x] Combine editor, diagnostics, schematic, and results in a responsive CodePen-like workspace.
- [x] Add Monaco syntax highlighting, completion, and hover help.
- [x] Show parser/semantic/ERC source spans inline and navigate from diagnostics to source.
- [x] Separate debounced live compilation from explicit simulation.
- [x] Render OP values, transient, Bode magnitude/phase, and DC sweep from typed datasets.
- [x] Add signal selection, engineering units, cursor values, zoom/pan, and clear lifecycle states.
- [x] Associate assertion results and threshold overlays with the relevant signal.
- [x] Verify keyboard accessibility, responsive mobile behavior, and light/dark contrast.
- [x] Pass source edit → diagnostic → fix → simulation → assertion → schematic E2E.

### 4.4 — Real-Circuit Parity and Measurement Hardening

- [x] Add gain-stage and power-amplifier examples to the Web selector.
- [x] Build native/Web parity matrices for compile, SPICE, model manifest/lock, measurements, and assertions.
- [x] Prove byte-identical model directives and canonical netlists across native and browser builds.
- [x] Complete the open-licensed `kessetsu_analog@1.0.0` starter model package.
- [x] Show resolved model provenance, license, version, and simulator capability in Web.
- [x] Run the model-injection security corpus in Web.
- [x] Add explicit steady-state windows and time-weighted RMS/average behavior.
- [x] Define THD with explicit fundamental, measurement window, Hann policy, resampling, harmonic range, and fail-closed sample requirements.
- [x] Make bandwidth fail closed for non-low-pass responses.
- [x] Add typed terminal-pair stress measurements such as `V(QN.c,QN.e)`.
- [x] Show and pass all 12 power-amplifier requirements in real Chromium.

### 4.5 — Professional Render and EDA Export

- [x] Define `kessetsu.export.v1` so every exporter consumes verified typed artifacts rather than rebuilding semantics.
- [x] Record the first-release format matrix and explicit exclusions in `docs/export_formats.md`.
- [x] Generate SVG, PNG, and single-page vector PDF only from canonical Schematic IR.
- [x] Expose canonical SPICE and Schematic IR JSON as machine-readable artifacts.
- [x] Generate editable KiCad and LTspice schematics.
- [x] Evaluate Qucs-S, CircuitJS, EasyEDA/EDIF, and other targets without claiming unsupported formats.
- [x] Share symbol geometry and font measurement across native and Web exports.
- [x] Define deterministic SVG, bounded PNG scale/background, and PDF page/orientation/margin policy.
- [x] Implement safe `kess render` and `kess export --target` commands.
- [x] Report artifact schema, path, hash, byte length, connectivity, warnings, and known losses in JSON.
- [x] Verify KiCad and LTspice component/pin/wire/junction/label/value/model connectivity on RC, gain, and power fixtures.
- [x] Open and netlist all three fixtures in installed KiCad 10 and LTspice 24.
- [x] Offer every supported Core export in one understandable Web artifact area.
- [x] Fail closed on unsupported or connectivity-unsafe export behavior.

**Status:** Shared Core ownership, determinism, EDA connectivity/openability, and owner-reviewed visual readability are verified. Lossy conversion is never silent.

### 4.6 — Sharing, Product Narrative, and Independent Agent Evidence

- [x] Define bounded, versioned, client-side compressed `kessetsu.share.v1` URL fragments.
- [x] Fail closed on malformed, unknown-version, or decompression-bomb payloads.
- [x] Round-trip UTF-8 source, compile schema, and exact package/version manifest.
- [x] Present RC, gain-stage, and power-amplifier examples with clear purposes.
- [x] Complete language, simulation/assertion, measurement, supported-domain, tutorial, cookbook, and troubleshooting documentation.
- [x] Explain measurable differences from raw SPICE, traditional simulators, and code-based circuit tools using current primary sources. _`docs/why_kessetsu.md`._
- [x] Record a versioned, reproducible external-LLM power-amplifier evaluation. _16 Ω candidate: `KES-T003`, 997.7 mW → 8 Ω revision: 12/12 PASS._
- [x] Store model/version, prompt, tool calls, iterations, final source, and assertion provenance without making a nondeterministic live call a CI gate.
- [x] Open the final source from a shared URL and verify schematic, simulation, 12/12 assertions, export, and re-sharing in Chromium.
- [x] State clearly that the first release uses CLI/tool contracts for AI agents and the Web Hub for humans; embedded AI chat is deferred.
- [x] Add a simple responsive product landing page that leads to the Web Hub and demonstrates requirement → CLI failure → revision → pass.
- [x] Audit documentation by audience and lifecycle, create `docs/README.md`, and define English as the primary language.
- [x] Translate the authoritative `docs/architecture.md`, `docs/cli_reference.md`, and `docs/ROADMAP.md` to English without changing their contracts. _Completed section by section on 2026-08-20; the roadmap's historical evidence was consolidated while all active scope and release gates remained explicit._

### 4.6V — Unseen Design and Incremental-Value Gate

- [x] Review current market documentation and repository evidence; record the 2026-09-10 recommendation and its limitations above.
- [x] Freeze six unseen task specifications before candidate generation: passive filter, active filter, transistor bias/amplifier, load driver, fixed-load power amplifier, and a manufacturer-model-dependent design. _`docs/evals/unseen-design-v1.md`, 2026-09-10: immutable conditions, measured targets, repeated-run budgets, record requirements, and independent checking protocol. No candidate trials have been run._
- [ ] Compare the same identified model and agent harness with Kessetsu versus direct Ngspice plus scripts and the relevant EDA workflow. Give both equal specifications, tool access, model data, and run budgets; allow baseline automation. Start with one accessible model, repeat each task at least three times, then check whether conclusions transfer to another available model. Record unavailable access rather than inventing runs.
- [ ] Keep loads, supplies, topology constraints, and acceptance measurements in an evaluator-owned specification. Candidate revisions cannot lower thresholds, replace required parts/models, or remove tests. Use independent formulas/reference calculations where applicable so both workflows are not graded only by Kessetsu's own measurement implementation.
  - [x] Implement U1 evaluation for Kessetsu and direct SPICE candidates. _`node scripts/evals/passive-filter.mjs <kessetsu|direct> <candidate-file>` validates the restricted passive topology, fixed load/source and resistance range, builds its own testbench, measures raw Ngspice AC/OP data, and checks analytic loaded-RC results. Five tests cover valid/invalid numbers, changed requirements, malformed data, both frontends, real simulation, and wrong cutoff. Development fixtures are not agent trial results._
  - [ ] Implement and verify independent evaluators for U2–U6, including model adequacy and fixed topology/testbench integrity; retain acquisition/unsupported outcomes for U6.
- [ ] Save prompts, exact available model identifiers/settings, sources, tool traces, failures, human interventions, elapsed time, usage/cost when available, and final artifacts. Report successful designs, false passes, unsupported cases, and artifact usability separately. Existing fixture replay remains a regression test.
- [ ] Review fresh schematic PNGs and open editable exports from successful unseen tasks; a corpus hash alone cannot establish readable new layouts.
- [ ] Audit verification claims and show which requirements, model assumptions, operating conditions, and untested physical effects each PASS covers. Check the flagship amplifier's generic driver and power-accounting limitations before using it as evidence of real-part performance.
- [ ] Record per-task comparative results and a continue / interoperability-first / reconsider decision with the owner. Require no false acceptance of immutable requirements in the evaluated set and a repeatable practical benefit on at least two task families; do not turn a small sample into a universal success-rate claim. Convert demonstrated blockers into bounded Phase 4 tasks before closing this gate.

This gate evaluates existing Phase 4 product claims; it does not begin Phase 5. Manufacturer-model support, import, or a separate test-spec interface is added only through an explicit scoped task if the evidence requires it. No paid model runs or third-party project uploads are implied by recording this plan.

**Access constraint, owner-confirmed 2026-09-10:** Only the current Codex session is available. Separate model/API trials are unavailable. Continue evaluator implementation and independent UI/maintenance work; current-session development or self-review must not be reported as fresh-context independent comparative evidence. Do not purchase access or launch paid runs without an agreed budget.

### 4.6R — Owner Web Hub and Editor Review

- [x] Capture the owner's 2026-09-10 feedback and define the workspace revision below.
- [x] Reduce repeated landing-page calls to action while preserving the accepted visual direction: one primary hero entry, a quiet navigation entry, and no repeated closing sales block.
- [x] Consolidate example selection, Check/Run/Cancel, Share, and one Export entry in the top workspace toolbar. Keep local view controls beside their panel; expose model/netlist/legal details without a permanent bottom export strip.
- [x] Make source, schematic, and results independently collapsible/restorable with draggable desktop splitters, keyboard resizing, minimum sizes, reset layout, and persistence. Keep source and results alive when hidden; provide a usable narrow-screen arrangement.
- [x] Replace assertion pills with a readable requirements table containing status, requirement, measured value, and limit; retain signal selection, failure visibility, and all assertion states.
- [x] Verify resizing, collapse/restore, export selection, simulation, keyboard interaction, and mobile layout with browser tests and actual screenshots. _2026-09-10: 15 Chromium E2E passed; desktop/mobile/export/requirements/light-theme screenshots inspected. Existing schematic connectivity and pointer regressions passed._
- [ ] Obtain owner feedback on the revised workspace and update public screenshots once its visual direction is accepted.

### Release Execution Order and Commercial Direction

1. Restore the 4.7 local verification prerequisites and patch the reported development dependency advisory.
2. Freeze 4.6V task specifications and implement evaluator-owned requirements and reproducible records before generating candidate designs.
3. Run accessible comparisons and document model-access gaps. Independent 4.6R interface work can proceed while external evaluation access is pending; this does not close 4.6V.
4. Resolve demonstrated model/verification/interchange blockers, finish owner-facing review, and repeat affected evidence.
5. Complete production deployment/rollback proof and the explicitly authorized integrated public release.

The local CLI and no-account local-browser workspace remain free core surfaces. Commercial hypotheses are optional managed compute/automation, team workflows, and commercial licensing; pricing and implementation follow evidence of demand. Before release, define a privacy-respecting way to collect voluntary feedback and distinguish repeat usage from willingness to pay. Paid infrastructure, accounts, or artificial export restrictions are not first-release prerequisites.

### 4.7 — Cross-Platform Packaging and Public Release Gate

- [x] Restore and rerun the current full verification gate before release. _2026-09-10: repaired recovery from incomplete `cargo-audit 0.22.2` extraction and upgraded Vitest to 4.1.11. The full canonical `scripts/verify.ps1` passed, including Rust/WASM, Web lint/10 unit tests/build, 15 Chromium E2E, dependency/license/artifact audits, replay, five independent U1 evaluator tests, packaging, and EDA smoke. Npm audit reports zero vulnerabilities. Accepted RustSec maintenance warnings and documented KiCad ERC/export warnings remain; this is not a claim of warning-free EDA interoperability. Rerun against the final release candidate._
- [x] Define CLI artifacts for Windows x86-64, Linux x86-64, macOS Intel, and macOS Apple Silicon.
- [x] Include simulator discovery/provenance, license notices, checksums, release manifests, and version probes.
- [x] Automate clean-environment install → compile → real simulation → assertion smoke tests on all four targets. _GitHub run `31730843184`: all packaging and 12/12 simulation jobs passed._
- [ ] Verify a real production Web Hub deployment, including cache headers, WASM/Worker MIME types, CSP, telemetry boundary, and rollback. _Workflow, `/Kessetsu/` content-hash audit, CSP, zero-telemetry policy, and tag/ref rollback are prepared; actual deployment remains blocked while the repository is private._
- [x] Include real-browser E2E, schematic visual/connectivity, benchmark parity, host release smoke, and EDA application smoke in canonical verification.
- [x] Make security, dependency/license, and generated-artifact audits release gates. _Zero known vulnerabilities; two bounded-input transitive unmaintained notices remain visible in `docs/security_audit.md`._
- [x] Add verified install paths, screenshots, Web Hub links, and support boundaries to the root README.
- [ ] Create the final release tag/changelog/migration record and make the repository/Web Hub public only after every acceptance criterion passes. _`CHANGELOG.md`, migration notes, release workflow, AGPL-3.0-only text, commercial-license notice, Corresponding Source link, and contributor boundary are prepared. Do not create the tag or change visibility yet._

### Phase 4 and First Public Release Acceptance

- [x] A user can compile, simulate, measure, inspect, and export a canonical circuit in Web without an account or local installation.
- [x] Web and CLI use the same Core semantics and versioned compile/schematic/simulation/measurement/assertion contracts.
- [x] The complex power-amplifier schematic passes connectivity and professional readability, collision, flow, label, and compactness gates.
- [x] SVG, PNG, PDF, Schematic IR JSON, SPICE, KiCad, and LTspice artifacts are available under their declared fidelity contracts.
- [x] Every Web export is available from CLI through the same capability contract.
- [x] OP, transient, AC, and DC sweep results are interactive and assertions are connected to relevant signals/thresholds.
- [x] User-defined and packaged models are reproducible, provenance-aware, and injection-safe in native and Web builds.
- [x] RC, gain-stage, and power-amplifier benchmarks make the same engineering decisions in CLI and Web.
- [x] A recorded amplifier correction replays structured CLI feedback without parsing human output. _This proves regression behavior; independent unseen design ability is evaluated separately in 4.6V._
- [ ] Pass the 4.6V unseen-design and incremental-value gate and resolve the blockers selected from its evidence.
- [x] Shared URLs round-trip source and exact package/version requirements without schema loss.
- [x] CLI installation and first simulation pass clean-machine smoke tests on every supported platform.
- [x] A new user can define, measure, assert, and export a circuit using public documentation only.
- [x] Canonical verification, browser E2E, schematic connectivity/visual corpus, benchmark parity, and release artifacts pass. _Accepted commit `5308f3b`; local full gate and remote CI `31850078955`._
- [ ] Publish the repository and Web Hub only after the incremental-value gate, owner review, production-deployment proof, and release-transaction gates are complete.

---

## 6. Phase 5+ — Long-Term Vision

After Phase 4 closes, order these options using 4.6V evidence. The default preference is useful model coverage and interchange, stronger verification across operating conditions, and agent onboarding, ahead of accounts, more export formats, or a new PCB engine. This is a prioritized direction, not authorization to bypass Phase 4 or implement the entire list.

- Provider-independent natural-language design/chat inside the Web Hub
- Bring-your-own-provider/API-key and optional managed AI service
- Accounts, cloud project storage, teams, and sharing permissions
- Component Knowledge Base and datasheet-derived rules
- Broad manufacturer SPICE-model registry with automated provenance updates
- Requirements/constraint schema and agent-driven design-loop tooling
- Parametric sweep, optimization, and design-space exploration
- Stability, RF, noise, and Monte Carlo measurements
- Datasheet-based voltage/current/thermal verification
- A measurable Verification Report and, only then, calibrated confidence
- Broader reusable subcircuit and component libraries
- VS Code extension
- PCB export, footprint mapping, and BOM
- Additional EDA schematic/netlist adapters
- Advanced layered/Sugiyama and hypergraph layout
- Multiple grounds such as AGND, DGND, and chassis
- Alternative simulator adapters such as Xyce and LTspice
- Cloud simulation and team/CI dashboards

Phase 5 must not begin until Phase 4 and the first-public-release acceptance gates are complete.

---

## 7. Target Pipeline

```text
Kessetsu source
    │
    ▼
Parser ─────────────► KES-Pxxx diagnostics
    │
    ▼
AST + source spans
    │
    ▼
Semantic analysis / typed Circuit IR ─► KES-Cxxx diagnostics
    │
    ├────────► Graph + structural ERC ─► KES-Exxx diagnostics
    ├────────► Deterministic SPICE
    ├────────► Schematic IR
    └────────► Structured JSON/WASM
                    │
                    ▼
              Simulation backend
                    │
                    ├────► KES-Sxxx diagnostics
                    └────► KES-Txxx assertion results
```

---

## 8. Working and Update Protocol

For every development session:

1. Read the active milestone and first open task in `docs/ROADMAP.md`.
2. Confirm the relevant architectural rule in `docs/architecture.md`.
3. Add characterization or regression coverage before changing behavior.
4. Implement the bounded change.
5. Run the relevant quality gates.
6. Mark only proven checkboxes as `[x]`.
7. Record material decisions or scope changes in the roadmap.
8. Confirm that verification did not leave unexpected worktree artifacts.

### Current Next Task

**4.6V — Extend independent requirement evaluators from U1 to U2–U6.**

The owner accepted this direction on 2026-09-10 and authorized developer-led execution. Six task specifications and the U1 evaluator now exist; the initial 4.6R interface revision is implemented and awaiting owner feedback. Continue the remaining evaluator/model-adequacy work with the current session while independent model access remains unavailable. Historical gates remain recorded; they do not substitute for new design evidence. Public release still requires the integrated Phase 4 gates and explicit owner authorization.
