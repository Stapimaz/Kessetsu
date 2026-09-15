# Kessetsu Development Roadmap

> This document is the **single source of truth** for Kessetsu development status, the active milestone, acceptance gates, and task order.
>
> Use `docs/architecture.md` for architectural rules and `docs/reference/cli.md` for the public CLI contract. If these documents conflict, this roadmap governs development status and the conflict must be resolved in the active milestone.
>
> Last comprehensive repository audit: **2026-09-15**
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
- At the time of the review, assertions lived beside the candidate source, so a design agent could weaken them or alter a fixed load/supply. The later `kessetsu.requirements.v1`/`.kessreq` contract now separates evaluator-owned assertions; fixed topology, load, supply, and model conditions still require an evaluator-owned harness where applicable.
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
- [x] Complete the unpublished-project identity migration without legacy aliases. _Completed before public release; repository: `Stapimaz/Kessetsu`. The one-time migration plan is retained in Git history rather than the active documentation tree._
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
- [x] Audit the repository and align `architecture.md`, `reference/cli.md`, and this roadmap with real behavior.
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
- [x] Document formulas, units, signs, windows, and analysis requirements in `docs/reference/measurements.md`.
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
- [x] Freeze the first-release supported-domain matrix. _`docs/reference/supported-domain.md`._
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

The first technically passing candidate was reopened after real PNG review exposed poor visual composition. The detailed method and dated evidence are in `docs/evals/`.

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
- [x] Record the first-release format matrix and explicit exclusions in `docs/reference/exports.md`.
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
- [x] Explain measurable differences from raw SPICE, traditional simulators, and code-based circuit tools using current primary sources. _`docs/guides/why-kessetsu.md`._
- [x] Record a versioned, reproducible external-LLM power-amplifier evaluation. _16 Ω candidate: `KES-T003`, 997.7 mW → 8 Ω revision: 12/12 PASS._
- [x] Store model/version, prompt, tool calls, iterations, final source, and assertion provenance without making a nondeterministic live call a CI gate.
- [x] Open the final source from a shared URL and verify schematic, simulation, 12/12 assertions, export, and re-sharing in Chromium.
- [x] State clearly that the first release uses CLI/tool contracts for AI agents and the Web Hub for humans; embedded AI chat is deferred.
- [x] Add a simple responsive product landing page that leads to the Web Hub and demonstrates requirement → CLI failure → revision → pass.
- [x] Audit documentation by audience and lifecycle, create `docs/README.md`, and define English as the primary language.
- [x] Translate the authoritative `docs/architecture.md`, `docs/reference/cli.md`, and `docs/ROADMAP.md` to English without changing their contracts. _Completed section by section on 2026-08-20; the roadmap's historical evidence was consolidated while all active scope and release gates remained explicit._

### 4.6V — Unseen Design and Incremental-Value Gate

- [x] Review current market documentation and repository evidence; record the 2026-09-10 recommendation and its limitations above.
- [x] Freeze six unseen task specifications before candidate generation: passive filter, active filter, transistor bias/amplifier, load driver, fixed-load power amplifier, and a manufacturer-model-dependent design. _`docs/evals/unseen-design-v1.md`, 2026-09-10: immutable conditions, measured targets, repeated-run budgets, record requirements, and independent checking protocol. No candidate trials had been run when the specification was frozen._
- [x] Compare the same identified model and agent harness with Kessetsu versus direct Ngspice plus scripts and the relevant EDA workflow. Give both equal specifications, tool access, model data, and run budgets; allow baseline automation. Start with one accessible model, repeat each task at least three times, then check whether conclusions transfer to another available model. Record unavailable access rather than inventing runs. _Completed 2026-09-13 for the only accessible model, `gpt-5.6-sol`/medium. Both arms passed 3/3 in U1–U5; Kessetsu was faster in U1/U3/U5 and direct in U2/U4. U6 exposed a real capability difference: direct passed 3/3 with the exact TI OPA197 model while Kessetsu safely reported 3/3 unsupported because it cannot import arbitrary external subcircuits. Kessetsu delivered 14/15 complete automatic visual/EDA sets across supported tasks; direct delivered no editable EDA schematic. A distinct second model was unavailable, so transferability remains untested rather than inferred. Evidence: the six per-task reports and `docs/evals/unseen-design-summary-2026-09-13.md`._
- [x] Keep loads, supplies, topology constraints, and acceptance measurements in an evaluator-owned specification. Candidate revisions cannot lower thresholds, replace required parts/models, or remove tests. Use independent formulas/reference calculations where applicable so both workflows are not graded only by Kessetsu's own measurement implementation. _Completed 2026-09-11: the frozen `unseen-design-v1.md` contract is enforced by bounded U1–U6 parsers and evaluator-owned testbenches; immutable-condition tampering, malformed data, and known false-pass paths are covered. Independent analytic/complex-response/KCL/Fourier/time-integration checks are used where applicable. This establishes evaluator integrity, not comparative agent value._
  - [x] Implement U1 evaluation for Kessetsu and direct SPICE candidates. _`node scripts/evals/passive-filter.mjs <kessetsu|direct> <candidate-file>` validates the restricted passive topology, fixed load/source and resistance range, builds its own testbench, measures raw Ngspice AC/OP data, and checks analytic loaded-RC results. Six tests cover valid/invalid numbers, changed requirements, inert self-check output directives, malformed data, both frontends, real simulation, and wrong cutoff. Development fixtures are not agent trial results._
  - [x] Implement and verify the bounded U2 active-filter evaluator. _2026-09-10: `node scripts/evals/active-filter.mjs <kessetsu|direct> <candidate-file>`. Supports an input RC followed by a non-inverting resistive-feedback stage using the exact built-in generic op-amp model. Checks fixed stimulus, rails, load, topology, and model; replaces candidate analyses with evaluator-owned OP/AC/transient runs. Compares the full complex AC response to an independent finite-gain/pole/loading formula and checks transient peak against AC gain. Relative cutoff is referenced to the required 100 Hz gain. Reports generic-model limitations and no hardware validation. Five developer tests and full canonical verification passed (including 15 browser E2E); the initial npm install file lock was resolved by stopping the developer-owned preview. Existing RustSec/EDA warnings remain unchanged._
  - [x] Harden shared evaluator input/evidence boundaries before comparative use. _Completed 2026-09-11: U1 now rejects content after `.end`; U1/U2 retain testbench, process status/logs, and any partial datasets on simulator or parse failure. A shared evaluator runtime provides the same failure-evidence behavior for U3/U4. Full canonical verification passed._
  - [x] Extend U2 evaluator coverage before comparative scoring. _Completed 2026-09-11: accepts one- or two-section input RC ladders and structurally restricted declared linear op-amp templates, includes interstage loading and actual declared gain/pole in its independent complex-response formula, and rejects disguised/altered models. Other active-filter topologies remain explicitly unsupported. Full canonical verification passed._
  - [x] Implement and verify bounded U3 common-emitter evaluation. _Completed 2026-09-11 for divider bias, unbypassed emitter degeneration, and input/output coupling using the exact frozen 2N3904 model. Evaluator-owned OP/AC/transient runs check collector bias/current, inverted 1 kHz gain, independently integrated transient fundamental, DC KCL, and AC/transient agreement. Direct/Core frontend, tampering, data-integrity, and real-Ngspice tests passed in canonical verification._
  - [x] Implement and verify bounded U4 low-side load-driver evaluation. _Completed 2026-09-11 for direct or bounded resistor gate drive using the exact frozen IRF540 model. Evaluator-owned transient analysis time-weights the final four periods while excluding 50 us around transitions, cross-checks drain/load KCL, and reports that switching loss, thermal behavior, and manufacturer fidelity are not covered. Direct/Core frontend, tampering, data-integrity, and real-Ngspice tests passed in canonical verification._
  - [x] Implement and verify bounded U5 fixed-load power-amplifier evaluation. _Completed 2026-09-11 for the buffer → voltage-gain → error-driver → complementary-output topology using exact generic op-amp/power-BJT models. The evaluator fixes 4 ohm/±9 V/100 mV at 1 kHz, independently integrates 10–30 ms RMS power and harmonic amplitudes, and measures positive device dissipation. It explicitly reports generic-driver supply power unavailable and does not claim total efficiency or hardware fidelity. Direct/Core, tampering, data-integrity, failure-evidence, and real-Ngspice tests passed in canonical verification._
  - [x] Implement and verify U6 manufacturer-model acquisition/integrity/capability evaluation. _Completed 2026-09-11: exact local-file hash and five-pin OPA197 identity, fixed topology/sources/load, independent AC/transient measurements, no generic fallback, and acquisition/capability error records. TI `SBOMA34D` Rev. D / Final 1.3 was acquired from the official product page for a local probe; hashes and use boundary are recorded in `docs/evals/opa197-model-probe-2026-09-11.md`, while copyrighted model text is neither committed nor emitted in evidence. Synthetic contract tests and the optional official local probe passed._
  - [x] Resolve the U6 model/interchange blocker without rewriting or redistributing the manufacturer model. _Completed 2026-09-11: the initial failure was traced to embedding a PSpice library in the main deck and to an over-minimized Ngspice sidecar. The evaluator now uses an isolated `.include` support file with `ngbehavior=ps`; the Windows runtime adds only the official Ngspice 46 `analog.cm` and `xtradev.cm` modules required by the translated transfer/switch elements. The exact TI model hash now passes OP, AC, and transient evaluation, remains user-supplied, and is absent from evidence except for provenance/hash. Runtime archive/module hashes and the diagnosis are recorded in the runtime README and model-probe document._
- [x] Save prompts, exact available model identifiers/settings, sources, tool traces, failures, human interventions, elapsed time, usage/cost when available, and final artifacts. Report successful designs, false passes, unsupported cases, and artifact usability separately. Existing fixture replay remains a regression test. _Completed for U1–U6 under the gitignored `.artifacts/agent-comparison-v1/` evidence tree. Six committed reports record scope, hashes, usage, corrections/exclusions, unsupported cases, and artifact usability; cost remains explicitly unavailable._
- [x] Review fresh schematic PNGs and open editable exports from successful unseen tasks; a corpus hash alone cannot establish readable new layouts. _Completed 2026-09-13 for every successful generated Kessetsu artifact: 14 fresh PNGs across U1–U5 passed visual review, and every corresponding KiCad/LTspice export opened/netlisted with all references retained and zero ERC errors beyond the characterized per-symbol `lib_symbol_issues` warning. U6 produced no Kessetsu candidate/artifact because exact manufacturer models are unsupported, and direct U6 was text-only; no nonexistent visual claim is made._
- [x] Audit verification claims and show which requirements, model assumptions, operating conditions, and untested physical effects each PASS covers. Check the flagship amplifier's generic driver and power-accounting limitations before using it as evidence of real-part performance. _Completed 2026-09-11: `docs/evals/verification-claim-audit-v1.md` defines PASS/FAIL/ERROR language and per-task publication boundaries. Every U1–U6 PASS/FAIL payload now emits versioned `verification_scope` metadata with task/check identity, operating conditions, model assumptions, excluded effects, and `hardware_validated: false`. U4 excludes switching intervals; U5 explicitly leaves driver power/total efficiency unavailable; U6 remains macromodel rather than hardware evidence. Dedicated contract tests and all evaluator suites passed._
- [x] Record per-task comparative results and a continue / interoperability-first / reconsider decision with the owner. Require no false acceptance of immutable requirements in the evaluated set and a repeatable practical benefit on at least two task families; do not turn a small sample into a universal success-rate claim. Convert demonstrated blockers into bounded Phase 4 tasks before closing this gate. _Completed 2026-09-13: the owner had directed continued development toward a complete private release candidate; the six-task evidence supports an interoperability-first continuation, not electrical-success or universal-speed superiority. Automatic artifact value repeated across five supported families, no immutable-requirement false acceptance was found, and U6 selected the external-model blocker below. See `docs/evals/unseen-design-summary-2026-09-13.md`._

**Evidence-selected blocker — required before 4.6V acceptance:**

- [x] Specify a typed external-subcircuit reference contract for Core/CLI: exact content hash, provenance/version/license, declared pin order/component kind, redistribution flag, simulator compatibility mode, deterministic lock data, and fail-closed diagnostics. Raw manufacturer model bodies must not become Kessetsu source or silently degrade to a generic template. _Accepted 2026-09-14 in `docs/decisions/0003-external-subcircuit-references.md`: source-relative native resources and explicitly bound stdin/browser bytes are hash- and structure-verified before IR; closed compatibility/redistribution enums prevent argument injection and silent copying; model bodies and absolute local paths are excluded from serialized contracts._
- [x] Implement the bounded contract through Circuit IR, native compile/simulate, model lock/provenance reporting, schematic metadata, and KiCad/LTspice export without bypassing the shared IR. The source design references a user-owned local model; Kessetsu neither embeds nor redistributes it. _Completed 2026-09-14: `external_subcircuit opamp` resolves through resource-bound AST → typed Circuit IR; native CLI canonicalizes source-relative files, simulation stages exact bytes in an isolated directory, and deterministic model v2/lock v2 plus schematic v2/KiCad/LTspice metadata retain the hash, entry, pins, compatibility, provenance, and redistribution policy without serializing the body. SPICE/LTspice exports retain an explicit sidecar dependency instead of copying it._
- [x] Define and test the Web boundary explicitly: safe user-selected model ingestion when the browser runtime supports the required simulator capability, otherwise a precise unsupported result with no server upload or generic fallback. _Completed 2026-09-14 for the present capability boundary: native file-based commands bind resources; stdin and WASM have no resource map and fail with `KES-C015`. Chromium E2E verifies that the Web editor reports the missing user-owned bytes, disables Run, emits no SPICE, performs no upload, and never substitutes the generic op-amp. Browser file ingestion remains a future capability rather than an implied feature._
- [x] Run tampering, missing-file, changed-hash, pin-order, unsupported-runtime, injection, and exact OPA197 integration tests; rerun full canonical verification. _Completed 2026-09-14: synthetic tests cover traversal/absolute/path injection, metadata control characters, missing bytes, hash changes, malformed or duplicate subcircuits, pin count/order, namespace collision, unsupported compatibility, source-directory escape, output-sidecar policy, and exact temporary staging. The optional exact-hash TI OPA197 path passed native OP/AC/transient through bundled Ngspice PSpice mode. Full `scripts/verify.ps1` passed with that local model enabled, including 16 Chromium E2E, all evaluator contracts, packaging, clean-artifact simulation, and installed KiCad/LTspice smoke._
- [x] Run a versioned U6 follow-up with three fresh Kessetsu attempts. Preserve the original unsupported result; close 4.6V only if the exact model passes and usable artifacts retain truthful provenance. _Completed 2026-09-14: three isolated `gpt-5.6-sol`/medium attempts passed the evaluator-owned exact TI OPA197 checks 3/3 and produced complete SVG/PNG/Schematic JSON/SPICE/KiCad/LTspice sets. Original PNGs passed visual review and KiCad files opened/netlisted with zero ERC errors beyond the known embedded-symbol warnings. Real LTspice review exposed that initial `.asc` files used the typed alias rather than the external `.SUBCKT` entry; the originals were retained, Core was fixed and regression-tested, and deterministic re-exports opened with the exact entry, relative sidecar, and all references. Candidate sources and electrical records were unchanged; model text was never committed or serialized. Evidence: `docs/evals/unseen-design-u6-followup-2026-09-14.md`._

This gate evaluates existing Phase 4 product claims; it does not begin Phase 5. Manufacturer-model support, import, or a separate test-spec interface is added only through an explicit scoped task if the evidence requires it. No paid model runs or third-party project uploads are implied by recording this plan.

**Access update, owner-confirmed 2026-09-11:** Fresh, ephemeral Codex CLI sessions authenticated through the owner's existing ChatGPT plan are authorized for the comparative gate; direct API calls, API keys, and GPT-6 Astra are not authorized. Use `gpt-5.6-sol` at medium reasoning for the first-model runs and conserve allowance through task-level checkpoints. A distinct second model remains unavailable until separately authorized, and that gap must remain explicit rather than being replaced with invented transferability evidence.

### 4.6R — Owner Web Hub and Editor Review

- [x] Capture the owner's 2026-09-10 feedback and define the workspace revision below.
- [x] Reduce repeated landing-page calls to action while preserving the accepted visual direction: one primary hero entry, a quiet navigation entry, and no repeated closing sales block.
- [x] Consolidate example selection, Check/Run/Cancel, Share, and one Export entry in the top workspace toolbar. Keep local view controls beside their panel; expose model/netlist/legal details without a permanent bottom export strip.
- [x] Make source, schematic, and results independently collapsible/restorable with draggable desktop splitters, keyboard resizing, minimum sizes, reset layout, and persistence. Keep source and results alive when hidden; provide a usable narrow-screen arrangement.
- [x] Replace assertion pills with a readable requirements table containing status, requirement, measured value, and limit; retain signal selection, failure visibility, and all assertion states.
- [x] Verify resizing, collapse/restore, export selection, simulation, keyboard interaction, and mobile layout with browser tests and actual screenshots. _2026-09-10: 15 Chromium E2E passed; desktop/mobile/export/requirements/light-theme screenshots inspected. Existing schematic connectivity and pointer regressions passed._
- [x] Replace the prototype-style stacked toolbars and permanent panel switcher with one compact application menubar. Move categorized examples under `File`, keep the primary Check/Run/Export/Share actions visible, and move source/license access under `Help` without weakening the Corresponding Source boundary. _Completed 2026-09-14, then refined after owner review: the active circuit name occupies the center; `File`, `View`, and `Help` contain only real commands; Run/Export/Share remain grouped on the right; redundant manual Check became a live auto-check status._
- [x] Give Source, Schematic, and Results native panel title bars with minimize, maximize/restore, double-click, `Escape`, persisted sizing, and a restore dock that appears only for minimized panels. Preserve each panel's live state and all schematic/result interactions. _Completed 2026-09-14: all three live panels share compact window controls, every panel may minimize, maximize/restore is reversible by button, double-click, or `Escape`, and the v2 persisted layout migrates v1 preferences._
- [x] Make the Web Hub dark-only, translate every user-visible browser and Worker status to English, and apply a restrained landing-page cleanup that removes decorative gradient excess without changing the accepted content structure. _Completed 2026-09-14: the light-theme branch and control are removed; workspace/Worker status text is English; landing content and layout are unchanged while page glows, card gradients, rotation, and excessive shadows are reduced._
- [x] Add interaction and visual regression coverage for menus, categorized examples, minimize/maximize/restore, layout persistence, narrow screens, dark-only behavior, and the English-only runtime path; inspect fresh desktop/mobile screenshots before owner review. _Completed 2026-09-14: fresh landing, menubar, workspace, requirements, export, and mobile screenshots were inspected; all 16 Chromium E2E and the full canonical verification gate passed._
- [x] Correct the Web document language metadata to English so locale-sensitive capitalization cannot produce Turkish glyphs, and regress the landing eyebrow text in Chromium. _Completed 2026-09-14: the root document now declares `lang="en"`; Chromium verifies the rendered hero eyebrow is ASCII `EXECUTABLE CIRCUIT ENGINEERING` rather than locale-transformed Turkish `İ`._
- [x] Remove redundant manual Check and Simulation-menu actions from the auto-compiling workspace. Replace them with an explicit live auto-check status and nest categorized examples under `File → Examples`; do not add placeholder advanced settings before the underlying capabilities exist. _Completed 2026-09-14: compile state is explicit from Core loading through checking, valid, and invalid results; the noninteractive header status reports `Auto-checking…`, `Checked`, or the error count. Examples now use an accessible nested menu on desktop and mobile. All 16 Chromium E2E and the full canonical verification gate passed._
- [x] Remove the overloaded header overflow control. Present resolved models and generated SPICE in one focused `View → Circuit details…` dialog, and keep documentation, Corresponding Source, and license access under `Help`. _Completed 2026-09-14: the header ellipsis and nested disclosure stack are gone; model provenance and generated SPICE now share one scrollable inspection dialog, while legal navigation remains in the application Help menu._
- [x] Replace instant sharing with an explicit naming and copy flow. Preserve the circuit name in backward-compatible versioned links, explain the self-contained/no-upload boundary, and retain exact source/package round trips. _Completed 2026-09-14: Share opens a focused dialog, validates and embeds an editable circuit name, copies only on explicit action, provides a manual-copy fallback, and restores the name after reload. Existing unnamed v1 links remain valid. A first-party short-link service is deferred because the current no-upload static architecture has no durable ID-to-project store._
- [x] Obtain owner feedback on the revised workspace and update public screenshots once its visual direction is accepted. _Accepted through the owner’s final editor/landing review sequence ending 2026-09-14. The resulting dark menubar, native panel controls, requirements table, named sharing, and local document workflow are preserved in fresh RC and power-amplifier screenshots generated by the browser suite._
- [x] Replace the provisional uppercase/icon identity with an owner-selected production wordmark and favicon before publication. _Completed 2026-09-15: the selected lowercase wordmark uses a short signal line and terminal node with the requested tighter spacing; its final optical touch-up centers the node between the preceding `s` and following `e` and protects that relationship with browser geometry coverage. The landing header uses the owner-approved 22 px presentation while the accepted 15 px editor and 12 px product-preview treatments remain unchanged. A 2.2 KiB self-hosted Inter subset keeps the mark stable across platforms, its OFL license ships in Web and CLI notices, and the generic Vite favicon is gone. Fresh desktop/mobile landing renders and regenerated RC/power-amplifier documentation and social-image sources were inspected after final sizing. Branding E2E, performance, runtime-license, deployment, the full canonical gate, and remote Linux CI run `34904319476` passed on commit `cca5123`; the final alignment/sizing follow-ups and refreshed assets also passed the full canonical gate locally on 2026-09-15._
- [x] Normalize the global Run, Export, and Share controls and make the unsaved-document state truthful. _Completed 2026-09-15: the desktop actions received consistent geometry and the document lifecycle became capability-aware. Native file-handle browsers provide in-place Save and Save As; Firefox and other unsupported browsers provide a browser-local Save checkpoint plus an explicit portable `.kess` download. Only successful persistence clears the dirty marker, cancellation and errors preserve it, opened native files retain their handle, and circuit naming remains stable after editing the default example. Unit and Chromium lifecycle regressions cover handle reuse, repicking, cancellation, fallback persistence/downloads, dirty-state transitions, and control geometry._
- [x] Make simulation ownership explicit in the workspace. Remove Run/Cancel from the global project toolbar, rename the user-facing Results panel to Simulation, and place its compact Run/Cancel control beside simulation status in that panel header. Preserve the internal `results` layout key so existing saved layouts continue to restore. _Completed 2026-09-15: the global toolbar now contains only compile state, Export, and Share; Simulation owns execution, progress, stale-result state, plots, and requirements. Desktop and narrow-screen browser coverage protects the new labels and action locality; refreshed RC and power-amplifier screenshots were inspected._
- [x] Add a focused Firefox document-lifecycle release regression without duplicating the full browser suite. Verify that `Ctrl+S` creates a browser-local checkpoint instead of invoking page download, while `Ctrl+Shift+S` / `Download .kess...` remains the explicit portable path; keep native handle reuse under Chromium coverage. _Completed 2026-09-15: the canonical and remote gates install both runtimes, run the full product suite in Chromium, and run the capability-specific document path in Firefox. The final local gate passed 19 browser scenarios with the intentionally inapplicable Firefox native-handle case skipped._

### 4.6P — Pre-Public Language, Trust, and Newcomer Hardening

The owner approved this bounded gate on 2026-09-15 after the final editor review. It does not reopen the accepted visual direction or begin Phase 5. Its purpose is to ensure that `1.0.0` cannot report vacuous verification success, accept language constructs that only fail at runtime, or send a new user through a misleading first workflow.

- [x] Restore a green Linux candidate by making the accepted Export/Share control geometry independent of platform font metrics; retain exact browser coverage rather than weakening the visual contract. _Completed 2026-09-15: both controls use fixed 76 × 30 px geometry with bounded internal padding; the exact browser assertion remains intact. Full local verification passed and remote Linux CI run `34989738740` passed on commit `d4f7410`._
- [x] Make `kess test` fail clearly when the circuit defines zero assertions. Keep `simulate` as the successful command for simulation without requirements, preserve structured JSON behavior, and add Core plus CLI regressions. _Completed 2026-09-15: zero assertions now produce `KES-T000`, structured `test_failed`, and exit 4 before simulator discovery; an empty assertion report is never vacuously successful. Targeted Core/CLI contracts passed._
- [x] Reject unsupported assertion metrics during semantic compilation, with a source-located `KES-C006` diagnostic. Runtime dispatch remains defensive but must not be the first validation boundary. _Completed 2026-09-15: the semantic allowlist now matches runtime measurement dispatch, unknown metrics fail on their source token, and a mutated-IR regression preserves defensive runtime rejection._
- [x] Remove simulation-selection ambiguity before publication: keep OP, transient, and AC analyses unique; allow at most one DC sweep per source; and source-locate malformed or duplicate `simulate` diagnostics until named analysis selectors exist. _Completed 2026-09-15: ambiguous repeats fail with source-located `KES-C009`; the established valid contract for separate voltage- and current-source DC sweeps remains backward compatible. Targeted IR/compiler contracts passed._
- [x] Specify the first immutable-requirements workflow for agent use without bypassing Circuit IR. Decide whether it belongs in `1.0.0` only after a versioned contract, threat boundary, CLI UX, and backward-compatibility review; do not let an agent-controlled design silently redefine evaluator-owned requirements. _Completed 2026-09-15 in ADR 0004 and `kessetsu.requirements.v1`: `.kessreq` is an assertion-only Core contract, external and inline authorities cannot mix, exact bytes are SHA-256 recorded and optionally pinned, and the typed assertions are attached to Circuit IR before the ordinary simulation/evaluation path. Core and CLI regressions cover hashing, invalid content, zero requirements, hash mismatch, mixed authority, no-launch failures, and successful structured evaluation. Filesystem ownership remains a truthful caller boundary rather than a cryptographic claim._
- [x] Curate the shipped example set and newcomer path. Release examples must have intentional names and English commentary; intentionally invalid fixtures stay in the test corpus; README commands must distinguish `check`, `simulate`, and assertion-backed `test` truthfully. _Completed 2026-09-15: `examples/` now contains six intentionally named English circuit sources plus one evaluator-owned `.kessreq`; parser/feature/floating-pin cases moved under test fixtures. README, CLI docs, tutorial, package install text, and release packaging use the assertion-backed RC path. Every public `.kess` passes dynamic check/compile coverage, benchmark copies are parity-locked, and affected SPICE/schematic tests passed._
- [x] Complete practical editor language assistance for the public grammar: all declaration forms, analyses, supported measurement functions, context-appropriate snippets, and useful hover text. `New Circuit` must start from a valid, instructive minimal document rather than a nearly empty stub. _Completed 2026-09-15: Monaco has grammar-aligned snippets and hover help for all top-level declarations, model boundaries, analyses, source waveforms, and supported metrics, with analysis/assertion/waveform context routing. The previously unreachable typed PWL path is now parsed with dimensional, non-negative, strictly increasing points. New Circuit opens a valid source/load example and compiles to a two-component schematic; 19 Web unit tests, production build, and Chromium/Firefox lifecycle regression passed._
- [x] Run a clean newcomer journey across landing page, Web editor, packaged CLI, assertions, simulation, and one EDA export; then run the full canonical gate, require green remote Linux CI, and update only the evidence-backed release record. Publication remains owner-controlled. _Completed 2026-09-15: the canonical gate passed end to end; the browser matrix passed 18 Chromium scenarios plus the Firefox fallback document lifecycle, with the inapplicable Firefox native-handle case intentionally skipped. The packaged newcomer smoke passed `check`, 5/5 inline assertions, SVG, KiCad, and 5/5 SHA-256-pinned external requirements; the packaged power-amplifier path passed 12/12 assertions; and installed KiCad/LTspice smoke passed for RC, gain-stage, and power-amplifier designs. Exact candidate commit `6c971fc` then passed remote Linux CI run `35016922389`._

### Release Execution Order and Commercial Direction

1. The owner changes repository visibility to public from the verified private candidate.
2. Verify the real production Web Hub, headers, MIME types, CSP, and rollback path.
3. Activate GitHub Pages/custom-domain DNS and HTTPS for `kessetsu.com`.
4. Run the four-platform release matrix, create the immutable `v1.0.0` tag and GitHub Release, then verify public assets and the live site once more.

The local CLI and no-account local-browser workspace remain free core surfaces. Commercial hypotheses are optional managed compute/automation, team workflows, and commercial licensing; pricing and implementation follow evidence of demand. Before release, define a privacy-respecting way to collect voluntary feedback and distinguish repeat usage from willingness to pay. Paid infrastructure, accounts, or artificial export restrictions are not first-release prerequisites.

### 4.7 — Cross-Platform Packaging and Public Release Gate

- [x] Complete final public-repository hygiene: organize active documentation by audience, remove completed one-time plans from the active tree, verify English/current claims and local links, audit tracked files and high-confidence credential patterns, and pass canonical local plus remote Linux verification. _Completed 2026-09-16: the docs root now contains only its index and the architecture/roadmap contracts; guides, references, maintainer operations, decisions, evidence, and assets have explicit homes. The completed identity-migration plan moved to Git history, all 45 Markdown files passed local-link audit, active text is English with no former-brand/path residue, and high-confidence credential scans found no matches. The full canonical gate passed, including packaged documentation/newcomer and installed EDA smoke; exact cleanup commit `8ba5c50` passed remote Linux CI run `35021980964`._
- [ ] Purge accidentally committed build outputs from Git history before changing visibility, then verify a fresh clone and force-push the rewritten private `main`. _The active tree is clean and a high-confidence history scan found no credential matches, but the private repository still contains 2,679 historical `core/target` blobs and is approximately 301 MiB remotely. Rewriting history changes commit identities and therefore requires explicit owner authorization before execution._
- [x] Stabilize the release CI before publication: install the lockfile-defined Playwright package before its matching browser runtime, eliminate the Linux fake-simulator `ETXTBSY` race, and require a green remote run from the final candidate commit. _Completed 2026-09-14 and reconfirmed 2026-09-15: CI installs the lockfile-defined Web dependencies before the matching Playwright browser; the simulator runner retries only Linux `ETXTBSY` launch races with bounded fail-fast behavior preserved elsewhere; remote Linux run `34871602877` first proved the repair, commit `2e2012d` passed run `34873765006`, and the branded final-candidate commit `cca5123` passed run `34904319476` including all 17 Chromium scenarios._
- [x] Complete the Web document lifecycle: new/open/save `.kess`, versioned local draft recovery, explicit document naming, and name-derived source/export filenames without server storage. _Completed 2026-09-14 and hardened 2026-09-15: `File` owns guarded New/Open/Save/Save As/Rename flows; native file-handle browsers write in place while Firefox and other unsupported browsers distinguish browser-local Save from an explicit portable `.kess` download. Only successful persistence clears the dirty marker. Local files and every Core export use deterministic name-derived filenames; stale share fragments are left when editing or replacing their source; and a bounded `kessetsu.web-draft.v1` record restores unsaved browser work without an account or upload. Contract unit and Chromium lifecycle coverage protect handle reuse, explicit Save As, cancellation, fallback persistence/downloads, and dirty-state transitions._
- [x] Establish one product-version source and prepare `1.0.0`: synchronize Cargo/npm/package/release metadata, verify SemVer consistency in the canonical gate, maintain an `[Unreleased]` changelog, expose version/What’s New in Web, and publish only the selected version’s release notes. _Completed 2026-09-14: root `VERSION` is authoritative and verified against Cargo plus both npm records; the CLI/Web/package report 1.0.0; changelog extraction is version-scoped; Web exposes version/What’s New; and the Windows v1.0.0 archive passed manifest/binary version agreement plus the 12/12 real-simulation smoke. No tag or GitHub release was created._
- [x] Close newcomer documentation and package gaps: platform install/PATH steps, a practical built-in/package/external-model recipe, truthful Web defaults, and self-contained or explicitly online-safe release documentation links. _Completed 2026-09-14: package `INSTALL.txt` now owns platform setup, simulator, first-command and PATH guidance; `.kess` examples and offline documentation ship in every archive; packaging fails on broken local Markdown links; and the cookbook documents built-in, exact-package, and hash-bound external-model workflows. The Web README now names its actual canonical default fixture._
- [x] Finish public-site readiness without publishing: visible first-release scope, support/source/changelog navigation, canonical/social/search metadata for `kessetsu.com`, a measured cold-load/simulation payload budget, and current product screenshots. _Completed 2026-09-14: landing claims now link to the bounded first-release domain; footer navigation exposes docs/scope/changelog/source/feedback; the root-domain CNAME, canonical/Open Graph/Twitter metadata, robots and sitemap are build-audited; current screenshots were regenerated and inspected; and enforced gzip ceilings passed at 71 KiB landing, 2,662 KiB editor activation, and 5,544 KiB first-simulation activation._
- [x] Restore and rerun the current full verification gate before release. _Final-candidate evidence updated 2026-09-15: commit `6c971fc` passed the full canonical `scripts/verify.ps1` locally and remote Linux CI run `35016922389`. Coverage included version/release-note contracts, Rust/WASM, Web lint/19 unit tests/build, payload budgets, runtime/deployment audits, the 20-case Chromium/Firefox browser matrix, dependency/license/artifact audits, replay and evaluator contracts, the v1.0.0 Windows package plus inline and hash-pinned newcomer smokes, clean real-Ngspice 12/12 power-amplifier simulation, and installed KiCad/LTspice smoke. The optional user-owned TI OPA197 executions were not bound in this shell and skipped; the earlier exact-hash run and recorded U6 follow-up remain preserved. Npm audit reports zero vulnerabilities. Two documented transitive RustSec maintenance warnings and the characterized embedded-symbol KiCad warnings remain; this is not a claim of warning-free EDA interoperability or hardware validation._
- [x] Define CLI artifacts for Windows x86-64, Linux x86-64, macOS Intel, and macOS Apple Silicon.
- [x] Include simulator discovery/provenance, license notices, checksums, release manifests, and version probes.
- [x] Automate clean-environment install → compile → real simulation → assertion smoke tests on all four targets. _GitHub run `31730843184`: all packaging and 12/12 simulation jobs passed._
- [ ] Verify a real production Web Hub deployment, including cache headers, WASM/Worker MIME types, CSP, telemetry boundary, and rollback. _Workflow, `/Kessetsu/` content-hash audit, CSP, zero-telemetry policy, and tag/ref rollback are prepared; actual deployment remains blocked while the repository is private._
- [x] Include real-browser E2E, schematic visual/connectivity, benchmark parity, host release smoke, and EDA application smoke in canonical verification.
- [x] Make KiCad application smoke fail on every ERC error and every warning except the characterized `lib_symbol_issues` limitation of the self-contained single-file export. _Completed 2026-09-11: all three canonical exports open and netlist successfully with zero ERC errors. The only accepted warnings are one missing external `Kessetsu` library-table entry per portable embedded symbol; unexpected types or summary/count drift now fail verification. A warning-free multi-file KiCad project bundle would be a separate export format, not a silent change to the current `.kicad_sch` contract._
- [x] Make security, dependency/license, and generated-artifact audits release gates. _Zero known vulnerabilities; two bounded-input transitive unmaintained notices remain visible in `docs/maintainers/security-audit.md`._
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
- [x] Pass the 4.6V unseen-design and incremental-value gate and resolve the blockers selected from its evidence. _Accepted 2026-09-14: U1–U5 established repeatable automatic artifact/interchange value without an electrical-success or universal-speed claim; the selected external-manufacturer-model blocker was implemented and the versioned U6 follow-up passed 3/3 with exact-model provenance and reviewed artifacts. Cross-model transferability and hardware validation remain explicitly unproven._
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
- Optional first-party short share links backed by privacy-defined project storage; do not route private circuit source through a third-party URL shortener
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

**4.7 — Finish repository hygiene before the owner-controlled public release transaction.**

The functional pre-public implementation gates and active-tree hygiene are closed, most recently on cleanup commit `8ba5c50` and remote Linux CI run `35021980964`. The sole pre-visibility task is an owner-authorized history rewrite that removes previously committed build outputs and is verified from a fresh clone. Only then should the owner change repository visibility and begin production Web Hub, rollback, DNS/Pages, tag, and GitHub Release verification. Phase 5 remains blocked until those Phase 4 release gates close.
