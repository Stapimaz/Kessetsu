# Export Contract and Format Matrix

Kessetsu 1.2.0 supports external comparator/two-terminal instances through the same
seven formats. Export metadata/warnings identify
required local file/hash/entry dependencies; no external model body is embedded. Native
SPICE/LTspice file destinations must remain beside their `.kess` source so relative
references stay valid; when moving exports, deliberately copy matching model directories
subject to their terms. Web downloads require the same manual dependency arrangement.
Two-terminal LTspice exports use its rectangular native outline with explicit `Prefix X`,
not resistor semantics. Known symbol mappings are unchanged. See the
[model catalog](model-catalog.md) for characterized setups and simulator-compatibility losses.

Kessetsu's export layer belongs to neither Web nor CLI. Every output is produced through the `kessetsu.export.v1` contract from connectivity-verified `kessetsu.schematic.v3`, which itself derives from typed Circuit IR. CLI and Web call only this shared Core API.

Every artifact reports the exporter name/version, MIME type and extension, byte length, SHA-256, `connectivity_verified`, capability fields, warnings, and known semantic losses. Unsupported topology or symbol geometry is never approximated silently; a `KES-Xxxx` diagnostic stops the export.

## Export formats

| Format | Purpose | Connectivity | Model | Analysis | Verification / explicit loss |
|---|---|---:|---:|---:|---|
| SVG | Scalable visual, documentation, and Web | Yes | No | No | Semantic text, fixed `viewBox`, light export style |
| PNG | Presentations, reports, and quick sharing | Visual projection | No | No | Pure-Rust canonical SVG raster; `0.25..8` scale, white or transparent background |
| PDF | Printing and vector documents | Visual projection | No | No | One page, content bounds, automatic orientation, Schematic IR margin, deterministic vector glyphs; no multi-page output |
| Schematic IR JSON | Lossless machine interchange | Yes | Yes | No | `kessetsu.schematic.v3`, deterministic pretty JSON with model provenance and resolved instance parameters but no external model body |
| SPICE | Simulation and automation | Yes | Yes | Yes | Canonical Ngspice netlist |
| KiCad `.kicad_sch` | Continued editing | Yes | Metadata | No | KiCad 10 parser/netlist/ERC smoke; portable embedded symbols may produce a symbol-table warning |
| LTspice `.asc` | Editing and LTspice simulation | Yes | Yes | Yes | Real LTspice 24.1.9 `-netlist` smoke; assertions remain in the `.kess` source |

The PDF policy is deliberately single-page because splitting an electronic schematic makes connectivity harder to follow. For a very large circuit, Core must first produce readable Schematic IR; the exporter does not invent arbitrary page breaks. The exporter uses a canonical content-sized media box rather than an A4/Letter frame, avoiding unused space and deriving orientation naturally from the content.

Raster and vector font measurement does not depend on system fonts. The repository's SIL OFL 1.1-licensed Roboto Mono is loaded identically for native and WASM rendering. Semantic SVG text remains selectable in the browser; PNG is raster output, while PDF converts glyphs to deterministic vector paths to avoid platform-dependent font-subset identifiers.

An external subcircuit remains a user-owned sidecar dependency. Schematic JSON and KiCad preserve its verified provenance, entry point, pin order, compatibility mode, redistribution policy, and content hash, but never embed the model body. SPICE and LTspice reference the validated relative file and therefore must be written beside the source `.kess` file; their export result includes an explicit dependency warning. Visual exports do not require or contain the model body.

## EDA verification

`scripts/verify-eda-exports.ps1 -RequireApplications` generates the RC-filter, gain-stage, and power-amplifier fixtures through Core/CLI. It then:

- parses each file with KiCad 10, generates a KiCad XML netlist, runs ERC, and compares the exact component set, pin identities, net connectivity, displayed values and model metadata against canonical Schematic IR;
- opens each `.asc` through LTspice 24.1.9's real `-netlist` path and compares component sets, ordered pin connectivity, values/models/stimuli and the active analysis against canonical Schematic IR and SPICE, including a complete `.end` record. Inactive analyses remain in the schematic as comments. Generated net names may differ; merged or split nets fail the comparison.

LTspice supports one active analysis at a time. The exporter activates the first declared
analysis and preserves the rest as visible comments with an explicit artifact warning;
select the desired analysis in LTspice before running. This target-specific behavior follows
the [LTspice schematic reference](https://analogdevicesinc.github.io/ltspice-reference/ai_ref/SCHEMATIC-REFERENCE.html#common-spice-analysis-commands).

Maintenance note: these analysis corrections are included in `1.0.1` in the
[changelog](../../CHANGELOG.md). The immutable `1.0.0` CLI downloads are unchanged; for their
LTspice exports, correct/choose the dotted analysis directive in LTspice before simulation.
Source names without the target's V/I prefix receive an ASCII prefix (for example,
`bias` becomes `V_bias`) so DC sweeps reference the actual exported source.

When the applications are unavailable, normal local verification reports the check explicitly as `skipped`; `-RequireApplications` is mandatory on the release machine. Core unit tests independently verify byte determinism, signatures, schemas, hashes, and connectivity contracts for every format.

KiCad officially documents its modern s-expression schematic format and `.kicad_sch` extension: <https://dev-docs.kicad.org/en/file-formats/sexpr-schematic/>. LTspice defines `.asc` schematics and `.net/.cir/.sp` netlists as application formats: <https://ltspicehelpmanual.azurewebsites.net/introduction1.htm>.

## Other formats

| Target | Decision | Rationale |
|---|---|---|
| Qucs-S `.sch` | Phase 5+ adapter candidate | Qucs-S documents schematics as project input, but this release has no round-trip evidence with an installed target application: <https://qucs-s-help.readthedocs.io/en/latest/overview/understanding-file-structure.html> |
| CircuitJS | Phase 5+ adapter candidate | It supports plain-text/URL circuit transfer, but its component and analog-model semantics do not map one-to-one to Kessetsu's scope. Official source: <https://github.com/sharpie7/circuitjs1> |
| EasyEDA JSON | Phase 5+ adapter candidate | The format is open and documented, but its compressed primitive contract and editor-version maintenance cost are high; support is not advertised without a real import smoke test: <https://docs.easyeda.com/en/DocumentFormat/EasyEDA-Document-Format/> |
| EDIF | Not implemented | Although it is a general interchange standard, there is no verified low-loss flow for a target application and analog schematic behavior |

These decisions do not exist merely to keep the format count small. They preserve the distinction between “a download button exists” and “engineering data was transferred.” A new adapter becomes a supported format only with a canonical graph-connectivity fixture and target-application smoke test.

## CLI

```powershell
kess render circuit.kess --output circuit.svg
kess render circuit.kess --output circuit.png --scale 3 --background transparent
kess render circuit.kess --output circuit.pdf

kess export circuit.kess --target schematic-json --output circuit.schematic.json
kess export circuit.kess --target spice --output circuit.spice
kess export circuit.kess --target kicad --output circuit.kicad_sch
kess export circuit.kess --target ltspice --output circuit.asc
```

Replacing an existing target requires explicit `--force`. Overwriting the source file is always rejected. With `--format json`, artifact bytes are never mixed into stdout; an agent sees only structured artifact metadata and diagnostics.
