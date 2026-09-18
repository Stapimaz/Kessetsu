# Kessetsu Architecture Constitution

This document defines the core algorithms, compilation pipeline, and language-design standards of Kessetsu. **Every developer and AI agent working on the project must follow these rules before changing code.**

> **For product behavior:** see [documentation](README.md) and the [changelog](../CHANGELOG.md). Local execution planning follows `AGENTS.md`.

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

The canonical Core entry points are `compile_source(source, options) -> CompileReport` and its resource-aware form `compile_source_with_resources(source, options, resources) -> CompileReport`. Neither writes files, launches processes, or prints logs; frontends bind resource bytes and own those side effects. Kessetsu 1.2.0 reports are versioned as `kessetsu.compile.v5` (published 1.1.0 uses v4) and, depending on options, can carry the flattened AST, typed IR, deterministic graph summary, SPICE, canonical `kessetsu.schematic.v2`, SVG, temporary legacy layout, and KiCad output. External resource bytes never appear in the report.

Parse, flattening, semantic, and ERC failures are normalized into the shared `Diagnostic` model. If any error-severity diagnostic exists, no backend output is produced. Warnings and informational diagnostics may accompany successful output. The CLI and WASM layers must remain adapters around this entry point and must not construct parallel compilation pipelines.

The CLI renders the canonical report inside the `kessetsu.cli.v1` agent envelope without changing Core semantics. Default JSON contains only compact status, diagnostic, summary, measurement, assertion, and artifact fields. AST, IR, graph, SPICE, datasets, and raw simulator logs are serialized only through explicit `--include` options. Compile, simulation, measurement, and assertion contract versions are declared in `domain_versions`. An unknown CLI schema request is rejected with `KES-F002` before compilation or file output begins. JSON stdout remains exactly one object. File output is a frontend responsibility: an existing destination is not overwritten without `--force`, and generated output may never overwrite the source `.kess` file.

When the CLI source path is `-`, source is read from stdin. Stdin-based `compile`, `simulate`, and `test` commands do not write a SPICE file unless an explicit `--output` is supplied. The netlist remains in memory for simulation, and JSON callers can request its text through `--include spice`. This removes the need for temporary source files and makes side-effect-free stdin requests byte-stable for the same input and options. Existing safe-overwrite behavior for file-based commands remains unchanged.

Simulator discovery checks packaged executable locations and system fallbacks. Automation and packaging environments may specify an executable through `KESSETSU_NGSPICE`. This override does not change the compilation pipeline, and launch/process failures remain CLI exit code `3`.

### Circuit-tool boundary

`tools::calculate_tool` is a pure shared calculation contract (`kessetsu.tool.v1`) for
nominal engineering tools. Typed SI inputs produce selected component quantities,
analytical results, equations, assumptions and editable `.kess` source. Generated circuits
must cross `compile_source` before any simulation or backend export; tool calculations
never invent assertion PASS results or generate SPICE/drawing artifacts independently.
The CLI and WASM adapters share this contract. Frontends own file writes, downloads and
workspace recovery. Preferred E12/E24 values describe nominal selection, not tolerance analysis.

### Parameter-expression boundary

`expression.rs` parses bounded engineering expressions into typed nodes and resolves
top-level named quantities through a deterministic dependency graph. Numeric component
fields are evaluated into `Quantity` during semantic IR conversion, without textual
substitution or numeric string round-trips. Dimensional arithmetic normalizes percent/angle
scales internally and validates the destination unit. Circuit IR carries optional
`kessetsu.parameters.v1` parameter and field-binding provenance; backends use resolved values
and never evaluate expressions. Empty provenance is omitted for literal circuits.

The parameter contract accepts passive, DC-source and supported waveform expressions in
top-level/module scopes, plus numeric analysis and inline-assertion fields at the root. Typed waveform arguments
and indexed analysis expressions resolve directly to Quantity through the shared numeric
evaluator. Literal and expression waveforms use one constructor/validation path. Component
bindings identify waveform fields; optional sorted analysis bindings carry the zero-based
IR analysis index and numeric field, rather than inventing a component identity.
Inline threshold and supported numeric metric expressions resolve into IR quantities.
Optional indexed numeric assertion arguments are authoritative for measurement evaluation;
the original signal text remains for display/legacy literal compatibility, never numeric
expression evaluation. Sorted assertion bindings record the assertion index and field.
Signal/component names and THD policy remain literal. Independent `.kessreq` files explicitly
reject expressions (including braced constants), retaining literal limits and byte identity.
`elaboration.rs` rebinds names in typed expression trees before shared IR evaluation;
it never substitutes source text or formats resolved numbers back into syntax. Module
defaults remain lexical; explicit overrides bind in the caller's scope before default
dependency resolution. Provenance records structured instance paths and effective overrides.
Declared module ports survive into IR for ERC and remain virtual graph aliases, not
drawable physical pins.
Virtual `ModulePort` parameters also preserve their structured instance path, including
literal modules without quantity provenance. Definition names identify source-embedded
modules within the document; no external circuit-module package resolution is implied.
Parameterized module analyses/assertions are explicitly rejected until their target/context
handling is implemented; place these at the root. Globals are
not implicitly captured. Existing literal module IDs and deterministic graph naming remain unchanged.
Recursion/expansion guards protect flattening. Requirements retain their independent ownership.
Root inputs use the pure `kessetsu.inputs.v1` contract through `compile_source_with_inputs`,
with source, output options and bound resources supplied separately. Existing entry points
delegate with empty inputs. Only declared root names and numeric literals are accepted;
validate discarded default names/dimensions before binding typed expression nodes. Preserve
original/default and effective override provenance. Optional `effective_source` materializes
only accepted root declaration expression spans for ordinary save/share/recompile workflows;
compilation never evaluates that edited string or generates backend output from it. Source
bytes, independent requirements and resource bindings remain caller-owned. Native and WASM
share this contract; no persistent Web override UI or hidden save/share state is introduced.
Elaboration carries internal parameter-origin metadata separately from the serialized AST/IR:
declaration and effective override positions survive nested flattening, while default
provenance stays unchanged. Structured parameter errors use these positions, not text-search
guesses. Other diagnostics still retain the existing fallback source annotation.
Source parsing is size/statement bounded before typed AST construction; waveform grammar is
non-recursive. Shared statement expression-node accounting covers all numeric roles during
parsing/expansion, with bounded numeric conversion afterward. Work ceilings are per pass,
not a claimed exact aggregate instruction count; see the language reference for limits.
The Web explicitly accepts v4 source-share envelopes when compiling with v5, recompiles source
normally and still checks exact packages. Unknown/future schema versions are not generalized
into an acceptance range. New shares identify the active compile schema.

## 2. Language Syntax and Rules

### Local experiment boundary (development source)

`experiment.rs` owns pure `kessetsu.experiment.v1` validation, deterministic case generation,
identity, shared measurements/constraints and finite feasible candidate selection. Cases
use canonical root `CompileInputs` and Circuit IR, not AST-generated backends or textual
expression substitution. Exact independent requirements replace IR assertions before
regeneration from IR. Native/browser adapters own sequential scheduling, cancellation,
binding, checkpoints and downloads. Explicit temperature config applies to IR-generated
decks; conflicting local control/temperature overrides fail closed. Seeded draws belong
to Core, not frontend RNGs. Full evidence is independent of plot decimation. Resume binds
plans/provenance/requirements and runtime fingerprints; checksums detect accidental
corruption, not independent attestation. Dynamic metrics use v3 in development source;
published 1.2.0 remains v2. See the [study guide](guides/parameter-studies.md).

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
| `device` | Catalog-backed external comparator/two-terminal | Model family pins | X |
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
  - `simulate tran 100ps 10ns uic` — model initial-condition transient, no DC operating point
  - `simulate ac dec 10 1Hz 1MHz` — AC analysis
  - `simulate dc V1 0V 5V 100mV` — independent voltage/current-source sweep
- **Assertions:**
  - `assert max(V(out)) < 3.3V`
  - `assert peak(I(D1)) < 100mA`
  - `assert output_power(V(out),RL) > 2W`
  - The canonical definitions of primitive and derived metrics, analysis requirements, and sign semantics are in `docs/reference/measurements.md`.

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

Default built-ins are `2N3904`/`2N3906` for BJTs, `IRF540` for MOSFETs, `1N4148` for diodes, and `KESSETSU_OPAMP_V1` for op-amps. Kessetsu 1.2.0 repairs the `1N4148`/`1N4007` diode directives with provenance version `1.0.1`; published CLI 1.1.0 remains affected by invalid descriptive fields in those models. Their legacy provenance is unchanged, and the repair does not establish manufacturer fidelity or enforce device ratings. Exact packaged models are unchanged. `KESSETSU_*` models are Kessetsu's own open-licensed generic verification models; they are not claimed to match a specific manufacturer's data sheet. `KESSETSU_PMOS_V1` has provenance version `1.0.1` and uses portable Ngspice `MOS1` parameters.

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

External subcircuits use the typed `external_subcircuit` contract in the [model catalog](reference/model-catalog.md). The declaration carries a source-relative resource reference, exact SHA-256, `.SUBCKT` entry, canonical pin mapping, provenance, closed simulator-compatibility mode, and explicit redistribution policy. The native CLI resolves only files contained under the `.kess` source directory; Core validates bytes before producing IR. Simulation stages an exact temporary copy and maps `ngspice_ps` only to the bounded Ngspice compatibility option. Op-amp, comparator and two-terminal families use the shared catalog. Web supports explicit in-memory local bindings and a bounded portable analog library profile. Only ephemeral browser simulation decks receive exact validated text through IR-selected references; ordinary SPICE/compile/IR/lock/export data retains dependencies, not resource bodies. Native-only compatibility fails before browser execution. No implicit upload, persistence or model substitution is permitted.

Attempts to hide `.control`, `.include`, shell syntax, or line breaks inside quoted parameters cannot cross typed numeric and metadata validation. When an error exists, the SPICE backend does not run. Regression tests protect this injection boundary.

## 6. ERC (Electrical Rules Check) Engine (`erc.rs`)

The [external-device contract](reference/model-catalog.md) extends
external subcircuits through closed catalog-backed interface families and explicit local
resource bindings. Its implementation/acceptance status is tracked separately; an accepted
ADR alone does not establish device, browser-model or export support. All new devices must
cross typed IR and use the shared catalog's pins/roles/geometry, never op-amp substitution.

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

Evaluator-owned requirements use the assertion-only `kessetsu.requirements.v1` contract in the [CLI reference](reference/cli.md#evaluator-owned-requirements). Core parses exact `.kessreq` bytes, enforces the same typed assertion semantics, and returns their SHA-256 identity. Native CLI attaches the resulting assertions to the already compiled `CircuitIR` before simulation; measurement and assertion backends consume that IR and never parse the external file directly. Inline and external assertions cannot be mixed. Optional expected-hash pinning fails closed before simulation. This records and separates requirement ownership but does not replace caller-controlled filesystem permissions.

### Simulation Domain and Runner Boundary

Native simulator-process details do not leak into the shared Core simulation contract. A versioned `SimulationRequest` contains typed analyses, netlist, timeout, and artifact policy. `SimulationResult` keeps analysis, simulator/process status, measurements, warnings, errors, raw logs, and artifact references separate. Native Ngspice and future browser adapters implement the same `SimulationRunner` boundary.

The native runner creates a unique temporary directory for every run. Successful artifacts are cleaned up; failure artifacts are retained only under an explicit `retain_on_failure` policy. The runner discovers and probes the simulator executable, terminates it on timeout, and accepts a shared cancellation token. CLI `simulate` and `test` both use this runner.

Ngspice analysis data is not scraped from stdout tables. Generated SPICE writes deterministic `wrdata` artifacts after every typed analysis. OP becomes a sorted scalar map, transient and DC become a shared axis plus real signal series, and AC becomes a frequency axis plus real/imaginary signal series. The parser normalizes exponents, decimal commas, and LF/CRLF differences. Malformed, duplicate, or non-finite data fails closed with `KES-S006`.

### Assertion and Measurement Semantics

Kessetsu 1.2.0 measurement contract `kessetsu.measurement.v2` adds AC-only `gain_at`,
`lower_cutoff` and `upper_cutoff`; published 1.1.0 uses v1. These metrics consume the
first typed AC dataset, interpolate magnitude along log frequency without extrapolation,
and select cutoff bands using an explicit reference or an unambiguous sampled peak.
Missing edges and disjoint automatic bands fail closed. Legacy metric semantics and
the assertion/requirements envelopes remain unchanged. See the
[measurement contract](reference/measurements.md) for exact boundary rules.

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
- **Canonical gate:** `kessetsu.schematic.v2` represents every connected graph pin with either a typed wire endpoint or a semantic net label and preserves model provenance metadata without model bodies. Missing or extra pins/nets fail closed with `KES-L001`. Kessetsu 1.2.0 also proves coordinate continuity, semantic-label joins and isolated transverse crossings. Foreign-net pin/junction/endpoint contacts, overlapping segments and disconnected islands fail the schematic gate; declared endpoint tags alone are insufficient. SPICE-only compilation remains independent of drawing generation.
- **Placement and routing:** Deterministic layered placement, shared pin-side metadata, orthogonal cost-based routing, and semantic labels for high-fan-out/power nets are canonical. Symbol/wire/label collisions, crossings, and bend counts appear in the versioned quality report.
- **Visual regression:** Deterministic SVG SHA-256 goldens for the schematic corpus are protected by Rust tests, and real-browser rendering is protected by the Playwright corpus.

The schematic engine legalizes symbol/rail space and reserves nearby annotation
blocks when wire-first annotation cannot fit. Shared visible symbol extents account for
off-axis strokes. LTspice adaptation matches semantic pins to native rotation/mirror transforms,
reroutes in actual native pin coordinates and repeats the geometry proof. KiCad preserves
shared annotation positions and uses canonical embedded pin geometry.

## 8. Frontend and Distribution Boundaries

The native CLI and browser workspace are adapters over the same pure Core. Native adapters
own files, subprocesses and simulator discovery; browser adapters own explicit local file
selection, worker scheduling and downloads. Neither reimplements circuit semantics.

Native releases cover Windows x86-64, Linux x86-64 and macOS Intel/Apple Silicon. Windows
packages the version-probed Ngspice sidecar; other targets discover a supported system
installation or an explicit executable override.

The Web lazily loads Core for circuit work and Ngspice on simulation. Ordinary documentation
and changelog routes are static pages, not editor sessions. Sharing carries source and exact
package identities through a bounded URL fragment; local resource bodies and datasets are
not uploaded or embedded. Unknown schemas and package mismatches fail closed.

Use the [CLI reference](reference/cli.md), [Web editor guide](guides/web-editor.md) and
[model catalog](reference/model-catalog.md) for public adapter behavior. Source contributions
must preserve these boundaries and the compatibility rules in [AGENTS.md](../AGENTS.md).
