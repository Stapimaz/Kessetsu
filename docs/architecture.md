# Kessetsu Architecture Constitution

This document defines the core algorithms, compilation pipeline, and language-design standards of Kessetsu. **Every developer and AI agent working on the project must follow these rules before changing code.**

> **For development planning and task tracking:** see `docs/ROADMAP.md`.

## 1. Core System Components

Kessetsu consists of two primary parts:

1. **`kessetsu-core` (Rust):** Contains the language parser, AST construction, Circuit IR conversion, ERC (Electrical Rules Check) engine, SPICE netlist generator, and automatic schematic-layout engine.
   - It builds as a library (`lib`), a CLI (`bin/`), and WebAssembly for the Web (`wasm.rs`).
2. **`webapp` (React/TypeScript):** The UI where users write source and inspect results such as schematics and plots. It runs `kessetsu-core` in the browser through WASM.

### Compilation Pipeline

```text
Kessetsu Source (.kess)
    │
    ▼
  Parser (kessetsu.pest → pest → AST)
    │
    ▼
  Module Flattening → Canonical AST
    │
    ▼
  Circuit IR (typed conversion + semantic validation)
    │
    ▼
  Canonical Graph → ERC
    │
    ├──► SPICE Netlist
    ├──► Schematic IR → Render / EDA Export
    └──► CompileReport (CLI/WASM/API)
```

**Critical rule:** Every backend—SPICE, layout, ERC, and JSON—must consume Circuit IR. No backend may generate output directly from the AST.

### Single Compile Contract

The canonical Core entry points are `compile_source(source, options) -> CompileReport` and its resource-aware form `compile_source_with_resources(source, options, resources) -> CompileReport`. Neither writes files, launches processes, or prints logs; frontends bind resource bytes and own those side effects. Reports are versioned as `kessetsu.compile.v4` and, depending on options, can carry the flattened AST, typed IR, deterministic graph summary, SPICE, canonical `kessetsu.schematic.v2`, SVG, temporary legacy layout, and KiCad output. External resource bytes never appear in the report.

Parse, flattening, semantic, and ERC failures are normalized into the shared `Diagnostic` model. If any error-severity diagnostic exists, no backend output is produced. Warnings and informational diagnostics may accompany successful output. The CLI and WASM layers must remain adapters around this entry point and must not construct parallel compilation pipelines.

The CLI renders the canonical report inside the `kessetsu.cli.v1` agent envelope without changing Core semantics. Default JSON contains only compact status, diagnostic, summary, measurement, assertion, and artifact fields. AST, IR, graph, SPICE, datasets, and raw simulator logs are serialized only through explicit `--include` options. Compile, simulation, measurement, and assertion contract versions are declared in `domain_versions`. An unknown CLI schema request is rejected with `KES-F002` before compilation or file output begins. JSON stdout remains exactly one object. File output is a frontend responsibility: an existing destination is not overwritten without `--force`, and generated output may never overwrite the source `.kess` file.

When the CLI source path is `-`, source is read from stdin. Stdin-based `compile`, `simulate`, and `test` commands do not write a SPICE file unless an explicit `--output` is supplied. The netlist remains in memory for simulation, and JSON callers can request its text through `--include spice`. This removes the need for temporary source files and makes side-effect-free stdin requests byte-stable for the same input and options. Existing safe-overwrite behavior for file-based commands remains unchanged.

Simulator discovery checks packaged executable locations and system fallbacks. Automation and packaging environments may specify an executable through `KESSETSU_NGSPICE`. This override does not change the compilation pipeline, and launch/process failures remain CLI exit code `3`.

## 2. Language Syntax and Rules

### Supported Components

| Keyword | Type | Pins | SPICE Prefix |
|---|---|---|---|
| `resistor` | Passive | p1, p2 | R_ |
| `capacitor` | Passive | p1, p2 | C_ |
| `inductor` | Passive | p1, p2 | L_ |
| `diode` | Semiconductor | p1, p2 | D_ |
| `transistor` | BJT (NPN/PNP) | c, b, e | Q_ |
| `mosfet` | MOSFET (NMOS/PMOS) | d, g, s | M_ |
| `opamp` | Operational amplifier | in_p, in_n, vcc, vee, out | X_ |
| `source` | Voltage source | plus, minus | V_ |
| `current_source` | Current source | plus, minus | I_ |

### Fundamental Rules

- `source` is the canonical component type for DC and waveform voltage sources; generated SPICE uses the `V_` prefix.
- **Values:** Component values are parsed into typed quantities. SI prefixes are supported:
  - `resistor R1 10k` → 10000 Ω
  - `capacitor C1 100uF` → 100 µF
  - `source Vin 5V` → 5 V DC
  - `source Vin sine(0V, 1V, 1kHz)` → transient sine source
  - `source Vin ac(1V)` → small-signal AC source
  - `source Vin sine_ac(0V, 1V, 1kHz, 1V)` → shared transient and AC source
  - Backward compatibility: `source Vin "SINE(0 1V 1kHz)"` is also accepted as a string.
- **Simulation commands:** Analysis commands are not carried as raw SPICE strings. They become typed `Analysis` variants during semantic conversion. Unsupported commands, invalid arity, incompatible units, or an invalid sweep direction fail closed with `KES-C009`.
  - `simulate op` — DC operating point
  - `simulate tran 10us 1ms` — transient analysis
  - `simulate ac dec 10 1Hz 1MHz` — AC analysis
  - `simulate dc V1 0V 5V 100mV` — independent voltage/current-source sweep
- **Assertions:**
  - `assert max(V(out)) < 3.3V`
  - `assert peak(I(D1)) < 100mA`
  - `assert output_power(V(out),RL) > 2W`
  - The canonical definitions of primitive and derived metrics, analysis requirements, and sign semantics are in `docs/engineering_measurements.md`.

## 3. Circuit IR (Intermediate Representation)

A typed intermediate layer exists between the AST and SPICE/layout/ERC. Its purposes are:

1. **Type safety:** Typed parameters such as resistance, capacitance, and waveform replace `value: String`.
2. **Static validation:** Electrical constraints can be checked before SPICE runs.
3. **Backend independence:** Syntax can evolve without forcing backend semantics to change.
4. **Agent access:** AI agents may inspect typed IR and diagnostics from compile reports; raw IR is not accepted as backend input.

**Rule:** No backend may bypass IR and operate directly on the AST.

## 4. Node-Naming Algorithm (`graph.rs`)

Component pin names, canonical backend order, SPICE prefixes, and layout pin coordinates are defined once in `core/src/component.rs`. Graph, ERC, SPICE, and layout must not define independent pin lists. A physically disconnected pin is represented by `Option<NetId>::None`, never by a magic integer. `NetId(0)` is the typed ground identity.

SPICE nodes are not assigned arbitrary integers. Kessetsu uses the following readability and determinism rules:

1. Everything connected to an explicit `net GND` is always node `"0"`, and this explicit reference takes precedence over the legacy fallback. If no explicit GND exists, the lexicographically first voltage-source `minus` net is used for backward compatibility. When multiple independent candidates exist, Kessetsu still selects deterministically but also emits `KES-E008`; ambiguity is never silently accepted.
2. **User-named nets are first-class identities.** If the user declares `net output`, that net appears as `output` in SPICE.
   - A component and a net may not share the same exact identifier (`KES-E006`).
   - A physical net may not have multiple user names (`KES-E007`).
3. For unnamed nets, the connected pin names are sorted alphabetically and the first pin name is selected.
4. Dots in the pin name become underscores and the result receives an `N_` prefix.
   - *Example:* If a node connects `R1.p2`, `C1.p1`, and `Q1.b`, the first alphabetically is `C1.p1`, so the node becomes **`N_C1_p1`**. SPICE refers to its voltage as `v(N_C1_p1)`.

**Determinism rule:** The same canonical graph must always produce the same node names. This algorithm must not be broken. User-named nets take precedence; automatic naming is only a fallback for unnamed nets.

**Note:** The algorithm is deterministic but not edit-stable. Adding a component can change an automatically generated name. Important measurement points should use user-named nets.

## 5. SPICE Engine and Standard Models

Ngspice integration uses a repository/release sidecar on Windows and the `KESSETSU_NGSPICE` override in automation and other packaging environments.

- Known parts such as `2N3904`, `1N4148`, and `IRF540` cause the SPICE generator to append the appropriate built-in `.model` definition automatically. Users do not need to write `.model` directives.
- **Model provenance:** IR records whether each model is built in or user-defined and records its type.
- **Model quality:** Built-in models are generic. Manufacturer-specific models and additional quality levels belong to later releases.
- **Canonical numbers:** Generated SPICE uses one formatter. Mid-range values are trimmed decimal values, very small and large values use normalized lowercase exponents, and neither `-0` nor binary floating-point artifacts may leak into output.

### Supported Models

| Model | Type | Source |
|---|---|---|
| 2N3904 | BJT NPN | Built in |
| 2N3906 | BJT PNP | Built in |
| 2N2222 | BJT NPN | Built in |
| KESSETSU_POWER_NPN_V1 | Generic power BJT NPN | Verified built in |
| KESSETSU_POWER_PNP_V1 | Generic power BJT PNP | Verified built in |
| 1N4148 | Diode | Built in |
| 1N4007 | Diode | Built in |
| IRF540 | MOSFET NMOS | Built in |
| KESSETSU_PMOS_V1 | Generic MOSFET PMOS | Verified built in |
| KESSETSU_OPAMP_V1 | Generic op-amp subcircuit | Verified built in |

Default built-ins are `2N3904`/`2N3906` for BJTs, `IRF540` for MOSFETs, `1N4148` for diodes, and `KESSETSU_OPAMP_V1` for op-amps. No fundamental component type defined by the language is therefore entirely unusable. `KESSETSU_*` models are Kessetsu's own open-licensed generic verification models; they are not claimed to match a specific manufacturer's data sheet. `KESSETSU_PMOS_V1` has provenance version `1.0.1` and uses portable Ngspice `MOS1` parameters.

### Typed User-Model and Subcircuit Boundary

Kessetsu does not accept raw `.include`, `.model`, `.subckt`, or control directives. Users provide typed declarations only. Parameter allowlists, numeric values, model kind/polarity, and metadata are validated during semantic conversion before Core produces a canonical directive:

```kessetsu
model diode SafeD version=1.0.0 license=MIT Is=2e-9 Rs=0.5
model mosfet SafeP pmos version=1.0.0 license=MIT Vto=-2 Kp=4
subcircuit opamp SafeOp (in_p,in_n,vcc,vee,out) version=1.0.0 license=MIT gain=100k bandwidth=2MHz
```

- Device model kinds are `diode`, `bjt`, and `mosfet`. The safe subcircuit template is currently limited to `opamp`.
- BJTs require `npn|pnp`; MOSFETs require `nmos|pmos`. Component/model kind or polarity mismatch produces `KES-C004`.
- User models and subcircuits carry required `version` and `license` metadata; `source` is optional. Electrical parameters come from a fixed allowlist for each kind. Unknown, duplicate, or non-finite parameters produce `KES-C010`.
- Op-amp pin order must match the shared component catalog byte for byte: `in_p,in_n,vcc,vee,out`. A mismatch is `KES-C012`. Backends do not own separate pin lists.
- Built-in, packaged, user-model, and subcircuit names share one case-insensitive namespace. Collisions produce `KES-C013`.
- Simulator capability is recorded as `ngspice-35+`. Unsupported package or simulator versions fail closed with `KES-C011`.

Exact packages are selected with syntax such as `model_include kessetsu_analog 1.0.0`; floating versions and ranges are not supported. Resolved models and packages appear in a `kessetsu.models.v2` manifest containing source, license, version, simulator capability, and a `sha256:` content hash. When file-based compile/simulate/test uses a model, a deterministic `kessetsu.lock` (`kessetsu.lock.v2`) is written beside the SPICE artifact and reported as a `model_lock` CLI artifact. Stdin-only calls do not write files; callers can request manifest and lock content through `--include models`.

External op-amp subcircuits use the typed `external_subcircuit` contract from [ADR 0003](decisions/0003-external-subcircuit-references.md). The declaration carries a source-relative resource reference, exact SHA-256, `.SUBCKT` entry, canonical pin mapping, provenance, closed simulator-compatibility mode, and explicit redistribution policy. The native CLI resolves only files contained under the `.kess` source directory; Core validates bytes before producing IR. Simulation stages an exact temporary copy and maps `ngspice_ps` only to the bounded Ngspice compatibility option. The Web runtime currently reports unresolved external resources as unsupported; it neither uploads them nor substitutes a generic model.

Attempts to hide `.control`, `.include`, shell syntax, or line breaks inside quoted parameters cannot cross typed numeric and metadata validation. When an error exists, the SPICE backend does not run. Regression tests protect this injection boundary.

## 6. ERC (Electrical Rules Check) Engine (`erc.rs`)

> **Note:** Older versions used the term DRC. The correct term for schematic-level checks is **ERC (Electrical Rules Check)**. DRC refers to physical PCB design checks.

### Structural Checks Without Simulation

| Code | Severity | Description |
|---|---|---|
| KES-E001 | Error | Duplicate component declaration |
| KES-E002 | Error | Undefined component reference |
| KES-E003 | Error | Floating mandatory pin |
| KES-E004 | Error | Direct short circuit (`source plus == minus`) |
| KES-E005 | Error | Pin reference not present on the component type |
| KES-E006 | Error | Component/net namespace collision |
| KES-E007 | Error | Multiple user names for one physical net |
| KES-E008 | Error | Multiple independent ground candidates |
| KES-E009 | Error | Duplicate net declaration |

### Runtime Diagnostics

| Code | Severity | Description |
|---|---|---|
| KES-S001 | Error | Simulator executable could not be launched |
| KES-S002 | Error | Simulator process/output failure |
| KES-S003 | Warning | Simulator warning or runtime cleanup warning |
| KES-S004 | Error | Convergence, singular-matrix, or timestep failure |
| KES-S005 | Error | Fatal or aborted simulator output |
| KES-S006 | Error | Measurement or analysis-dataset parse failure |

Assertion results are separate from simulation diagnostics and use the versioned `kessetsu.assertion.v1` report. Every assertion receives a deterministic `KES-T001`, `KES-T002`, and so on in source order, with one of `PASS`, `FAIL`, `ERROR`, or `SKIPPED`. Missing measurements never become `NaN`; they produce an explanatory `ERROR`. Unsupported metric names and invalid argument shapes fail semantic compilation before simulation. If simulation does not finish successfully, Kessetsu does not fabricate assertion results and reports them as `SKIPPED`. An empty assertion report is never successful: `kess test` emits `KES-T000` before simulator discovery, while `kess simulate` remains valid without assertions.

Evaluator-owned requirements use the assertion-only `kessetsu.requirements.v1` contract defined in [ADR 0004](decisions/0004-evaluator-owned-requirements.md). Core parses exact `.kessreq` bytes, enforces the same typed assertion semantics, and returns their SHA-256 identity. Native CLI attaches the resulting assertions to the already compiled `CircuitIR` before simulation; measurement and assertion backends consume that IR and never parse the external file directly. Inline and external assertions cannot be mixed. Optional expected-hash pinning fails closed before simulation. This records and separates requirement ownership but does not replace caller-controlled filesystem permissions.

### Simulation Domain and Runner Boundary

Native simulator-process details do not leak into the shared Core simulation contract. A versioned `SimulationRequest` contains typed analyses, netlist, timeout, and artifact policy. `SimulationResult` keeps analysis, simulator/process status, measurements, warnings, errors, raw logs, and artifact references separate. Native Ngspice and future browser adapters implement the same `SimulationRunner` boundary.

The native runner creates a unique temporary directory for every run. Successful artifacts are cleaned up; failure artifacts are retained only under an explicit `retain_on_failure` policy. The runner discovers and probes the simulator executable, terminates it on timeout, and accepts a shared cancellation token. CLI `simulate` and `test` both use this runner.

Ngspice analysis data is not scraped from stdout tables. Generated SPICE writes deterministic `wrdata` artifacts after every typed analysis. OP becomes a sorted scalar map, transient and DC become a shared axis plus real signal series, and AC becomes a frequency axis plus real/imaginary signal series. The parser normalizes exponents, decimal commas, and LF/CRLF differences. Malformed, duplicate, or non-finite data fails closed with `KES-S006`.

### Assertion and Measurement Semantics

- `value` selects the final sample; `min` and `max` select signed extrema; `average` (`avg`) is the arithmetic mean; `rms` is root mean square.
- `peak` is the absolute peak, `max(abs(x))`, not the positive maximum. Generated `.meas` fallback measures positive maximum and negative minimum separately, then selects the larger magnitude.
- OP contains one scalar sample. For OP, `value`, `min`, `max`, and `average` return the same signed value; `peak` and `rms` return its magnitude.
- Equality and inclusive limits (`==`, `<=`, `>=`) use default tolerances of `abs=1e-9` and `rel=1e-6`. Strict `<` and `>` are never relaxed by tolerance.
- Current direction follows the Ngspice branch-current convention: positive current enters the component's canonical positive/reference pin. This is `plus` for a voltage source and `p1` for an inductor. A voltage source delivering power therefore often has negative OP current. The assertion engine removes sign only for explicitly magnitude-based metrics such as `peak` and `rms`.
- Because AC data is complex, raw `min`, `max`, `peak`, `average`, and `rms` reductions currently fail closed with `ERROR`. Frequency-domain magnitude and phase metrics are defined in the engineering-measurement layer.

### Output Formats

- **Human mode (default):** stage/code/message diagnostics on stderr; assertion PASS/FAIL lines may use terminal color.
- **JSON mode (`--format json`):** machine-readable structured diagnostics.

  ```json
  {
    "code": "KES-E003",
    "severity": "error",
    "stage": "erc",
    "message": "Floating Pin: R1.p1 is not connected to anything.",
    "component": "R1",
    "pin": "p1",
    "field": null,
    "line": null,
    "column": null
  }
  ```

**Important:** ERC reports detected risks. It does not replace physical validation or engineering review. Thermal behavior, PCB parasitics, ESD, and manufacturer tolerances remain outside ERC scope.

## 7. Schematic Ownership and Layout Engine

Circuit IR is the single source of electrical semantics; the versioned Schematic IR is the single source of drawing semantics. Schematic IR is generated from Circuit IR and the canonical graph. It explicitly carries component instances, canonical pin anchors, orientation, wire endpoints and segments, junctions, disconnected crossings, net labels, bounds, and quality/connectivity reports. For the same Circuit IR, serialization must be collection/declaration-order independent and byte-stable.

Render/export ownership follows this boundary:

```text
Circuit IR + Canonical Graph
        → kessetsu.schematic.v2
        → SVG / PNG / PDF / Schematic JSON / KiCad / LTspice exporters
        → CLI artifact writer or Web download UI
```

Renderers and exporters never return to the AST or legacy layout shapes. They do not redefine component pins, connections, or symbol geometry. The shared symbol/pin catalog lives in `core/src/component.rs`. The Web displays Schematic IR/SVG and adds interactions such as zoom, pan, and selection. File writes, overwrite policy, and browser downloads remain outside pure Core exporter results.

The legacy `layout.rs` behavior below remains a characterization baseline during the Phase 4 migration:

Schematic placement originally used a **chain-based vertical-layout** approach for assigning component x/y coordinates. DFS was one heuristic for traversal and initial ordering.

- **Legacy heuristic:** Voltage-source rails, GND direction, through pins, and `is_signal_pin` affect chain order and rotation. Active devices such as BJTs and MOSFETs have separate placement behavior, but ideal orientation is not guaranteed for every topology.
- **Canonical gate:** `kessetsu.schematic.v2` represents every connected graph pin with either a typed wire endpoint or a semantic net label and preserves model provenance metadata without model bodies. Missing or extra pins/nets fail closed with `KES-L001`.
- **Placement and routing:** Deterministic layered placement, shared pin-side metadata, orthogonal cost-based routing, and semantic labels for high-fan-out/power nets are canonical. Symbol/wire/label collisions, crossings, and bend counts appear in the versioned quality report.
- **Visual regression:** Deterministic SVG SHA-256 goldens for the schematic corpus are protected by Rust tests, and real-browser rendering is protected by the Playwright corpus.

## 8. Kessetsu Vision and Ecosystem Manifesto

**Kessetsu** is an AI-agent-oriented, deterministic SPICE compiler and verification platform for developing analog and mixed-signal circuits like software. It runs natively and in the browser, expresses circuits as source, simulates them, and tests them with assertions.

```text
Compile, simulate and test circuits like software.
```

The ecosystem has three pillars:

### 1. Kessetsu Core

The heart of the project. Parser, IR, ERC, SPICE generation, and layout live in one Rust crate. It builds as a library, CLI, and WASM package. The same circuit produces deterministic results across platforms.

### 2. Kessetsu CLI — Engine for AI Agents and Developers

An offline-capable Rust CLI for compilation, ERC, SPICE generation, simulation, testing, and export. Windows x86-64 releases package Ngspice as a sidecar. Linux x86-64 and macOS Intel/Apple Silicon releases discover a version-probed system Ngspice and support an explicit executable override.

- **Current distribution:** Four platform artifacts, SHA-256/release manifests, and clean-machine simulation smoke tests. `cargo install` and a VS Code extension are later distribution targets.
- **Use:** AI agents and hardware engineers use the CLI to compile circuits, test requirements, and consume structured diagnostics. See the [CLI Reference](cli_reference.md).
- **TDD loop:** An agent can use assertions like software tests, consume structured failures, revise the circuit, and repeat.

### 3. Kessetsu Web Hub — Showcase and Playground for Humans

A no-account, no-install interface that runs the Rust Core in the browser through WASM.

- **Current workspace:** Write source → debounced WASM compile/ERC → canonical schematic → Ngspice simulation in a dedicated worker → typed plot/measurement/assertion results.
- **Current exports:** SVG, PNG, PDF, Schematic JSON, SPICE, KiCad, and LTspice through Core's `kessetsu.export.v1` capability contract. The Web does not reconstruct exporter semantics.
- **Current sharing:** A `kessetsu.share.v1` URL fragment carries source, compile schema, and the exact package/version manifest through gzip + base64url. Decoding uses streaming size limits; unknown versions, malformed payloads, or post-compile package mismatches fail closed. Projects are not uploaded to a server.

**Security note:** The Web playground never inserts user input directly into a SPICE netlist. All input crosses typed IR validation. Any future raw-SPICE escape hatch such as `unsafe spice_raw {}` remains disabled by default on the Web.

**In summary:** Kessetsu is not merely a drawing application. It is a circuit compiler and verification platform. The current product offers agent-oriented compile/test/export feedback through the CLI and a Web workspace—using the same Core—for compile, simulation, measurements, schematics, exports, and sharing. Remaining product gates include independent unseen-design evaluation, resolution of demonstrated model/workflow blockers, owner acceptance of the revised workspace, production deployment proof, and the final authorized public-release transaction. See the roadmap for current evidence and execution order; passing regression tests alone does not establish comparative product value or hardware fidelity.
