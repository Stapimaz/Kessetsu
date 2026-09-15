# Export Contract and Format Matrix

Kessetsu's export layer belongs to neither Web nor CLI. Every output is produced through the `kessetsu.export.v1` contract from connectivity-verified `kessetsu.schematic.v2`, which itself derives from typed Circuit IR. CLI and Web call only this shared Core API.

Every artifact reports the exporter name/version, MIME type and extension, byte length, SHA-256, `connectivity_verified`, capability fields, warnings, and known semantic losses. Unsupported topology or symbol geometry is never approximated silently; a `KES-Xxxx` diagnostic stops the export.

## First-release formats

| Format | Purpose | Connectivity | Model | Analysis | Verification / explicit loss |
|---|---|---:|---:|---:|---|
| SVG | Scalable visual, documentation, and Web | Yes | No | No | Semantic text, fixed `viewBox`, light export style |
| PNG | Presentations, reports, and quick sharing | Visual projection | No | No | Pure-Rust canonical SVG raster; `0.25..8` scale, white or transparent background |
| PDF | Printing and vector documents | Visual projection | No | No | One page, content bounds, automatic orientation, Schematic IR margin, deterministic vector glyphs; no multi-page output |
| Schematic IR JSON | Lossless machine interchange | Yes | Yes | No | `kessetsu.schematic.v2`, deterministic pretty JSON with model provenance but no external model body |
| SPICE | Simulation and automation | Yes | Yes | Yes | Canonical Ngspice netlist |
| KiCad `.kicad_sch` | Continued editing | Yes | Metadata | No | KiCad 10 parser/netlist/ERC smoke; portable embedded symbols may produce a symbol-table warning |
| LTspice `.asc` | Editing and LTspice simulation | Yes | Yes | Yes | Real LTspice 24.1.9 `-netlist` smoke; assertions remain in the `.kess` source |

The PDF policy is deliberately single-page because splitting an electronic schematic makes connectivity harder to follow. For a very large circuit, Core must first produce readable Schematic IR; the exporter does not invent arbitrary page breaks. The first release uses a canonical content-sized media box rather than an A4/Letter frame, avoiding unused space and deriving orientation naturally from the content.

Raster and vector font measurement does not depend on system fonts. The repository's SIL OFL 1.1-licensed Roboto Mono is loaded identically for native and WASM rendering. Semantic SVG text remains selectable in the browser; PNG is raster output, while PDF converts glyphs to deterministic vector paths to avoid platform-dependent font-subset identifiers.

An external subcircuit remains a user-owned sidecar dependency. Schematic JSON and KiCad preserve its verified provenance, entry point, pin order, compatibility mode, redistribution policy, and content hash, but never embed the model body. SPICE and LTspice reference the validated relative file and therefore must be written beside the source `.kess` file; their export result includes an explicit dependency warning. Visual exports do not require or contain the model body.

## EDA verification

`scripts/verify-eda-exports.ps1 -RequireApplications` generates the RC-filter, gain-stage, and power-amplifier fixtures through Core/CLI. It then:

- parses each file with KiCad 10, generates a KiCad XML netlist, runs ERC, and verifies that every component reference remains present;
- opens each `.asc` through LTspice 24.1.9's real `-netlist` path and verifies every component reference plus a complete `.end` record in the resulting `.net` file.

When the applications are unavailable, normal local verification reports the check explicitly as `skipped`; `-RequireApplications` is mandatory on the release machine. Core unit tests independently verify byte determinism, signatures, schemas, hashes, and connectivity contracts for every format.

KiCad officially documents its modern s-expression schematic format and `.kicad_sch` extension: <https://dev-docs.kicad.org/en/file-formats/sexpr-schematic/>. LTspice defines `.asc` schematics and `.net/.cir/.sp` netlists as application formats: <https://ltspicehelpmanual.azurewebsites.net/introduction1.htm>.

## Evaluated targets not included in the first release

| Target | Decision | Rationale |
|---|---|---|
| Qucs-S `.sch` | Phase 5+ adapter candidate | Qucs-S documents schematics as project input, but this release has no round-trip evidence with an installed target application: <https://qucs-s-help.readthedocs.io/en/latest/overview/understanding-file-structure.html> |
| CircuitJS | Phase 5+ adapter candidate | It supports plain-text/URL circuit transfer, but its component and analog-model semantics do not map one-to-one to Kessetsu's scope. Official source: <https://github.com/sharpie7/circuitjs1> |
| EasyEDA JSON | Phase 5+ adapter candidate | The format is open and documented, but its compressed primitive contract and editor-version maintenance cost are high; support is not advertised without a real import smoke test: <https://docs.easyeda.com/en/DocumentFormat/EasyEDA-Document-Format/> |
| EDIF | Outside the first release | Although it is a general interchange standard, there is no verified low-loss flow for a target application and analog schematic behavior |

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
