# ADR 0003: Provenance-Preserving External Subcircuit References

- Status: accepted
- Date: 2026-09-14
- Scope: Core, native CLI, browser boundary, simulation, model manifests, and EDA exports

## Context

The U6 comparison demonstrated a concrete product gap. A direct Ngspice workflow could use the exact user-supplied TI OPAx197 PSpice model, while Kessetsu correctly refused to substitute its generic op-amp template but had no supported way to bind the local manufacturer file. Kessetsu already has typed model declarations, deterministic model manifests and locks, and a no-raw-directive security boundary. External model support must extend those contracts without turning `.kess` into an arbitrary SPICE-command container or redistributing third-party model text.

## Decision

Kessetsu will support a bounded, typed external-subcircuit declaration. The first component kind is `opamp`; widening the kind set requires catalog pin metadata and tests. A declaration identifies a user-owned file and the exact subcircuit inside it:

```kessetsu
external_subcircuit opamp OPA197 (in_p,in_n,vcc,vee,out) file="models/OPAx197.LIB" entry=OPAx197 sha256=fc5b020e63346e511bd808bf41c856b0150b000bcf8a41fe00eeececb1f422a5 version="Final 1.3" license="TI terms" source="https://www.ti.com/lit/zip/SBOMA34" simulator=ngspice_ps redistribution=prohibited
```

### Required fields and closed vocabularies

- `file`: a UTF-8 relative resource reference. Native file-based commands resolve it relative to the `.kess` file. Absolute paths, parent traversal, control characters, NUL, and quote characters are rejected. Stdin and browser callers bind bytes explicitly under the same resource reference.
- `entry`: the exact case-insensitive `.SUBCKT` entry name expected in the file.
- `sha256`: exactly 64 hexadecimal characters. The resolved bytes must match before any backend output or simulator launch.
- `version`, `license`, and `source`: non-empty provenance metadata. They are descriptive and never interpreted as SPICE.
- `simulator`: initially `ngspice` or `ngspice_ps`. This is a typed compatibility mode, not an arbitrary argument string.
- `redistribution`: initially `permitted` or `prohibited`. Kessetsu never infers permission. `prohibited` is the expected manufacturer-model default.
- The declared canonical pin order must exactly match the shared component catalog. For an op-amp it is `in_p,in_n,vcc,vee,out`.

Unknown, duplicate, missing, or malformed fields fail closed. External names share the existing case-insensitive model namespace.

### Resolution and validation boundary

The Core compiler receives resource bytes through a typed resource map; Core does not open arbitrary filesystem paths. Native CLI code is the filesystem adapter: it reads only validated source-relative references and passes the bytes to Core. The browser may later supply bytes selected explicitly by the user and kept locally in memory. A plain browser compile with an unresolved external reference returns a precise unsupported/missing-resource diagnostic; it never uploads the file and never falls back to a generic model.

Before IR is produced, Core:

1. verifies the exact byte-level SHA-256;
2. decodes the model as UTF-8;
3. locates one unambiguous, non-comment `.SUBCKT <entry>` declaration and matching `.ENDS`;
4. verifies its positional terminal count against the declared canonical pins;
5. records the selected component kind, pin order, compatibility mode, redistribution policy, resource reference, entry name, hash, and provenance.

The model body is not serialized into AST/IR debug JSON, model manifests, lockfiles, schematic JSON, diagnostics, or evidence reports. Diagnostics may expose the declared resource reference and hashes, but not file content or an absolute local path.

### IR and backend behavior

`CircuitIR` remains the single backend source of truth. Its model definition gains an external-subcircuit variant containing only the validated typed metadata. Backend behavior is:

- canonical SPICE references the external model with a generated `.include` directive derived from the validated resource reference and instantiates the selected `entry`;
- native simulation stages an exact temporary byte-for-byte copy inside its isolated run directory, uses a hash-stable generated filename, and deletes it under the existing artifact policy;
- `ngspice_ps` maps only to Ngspice's `-D ngbehavior=ps`; no source-controlled arbitrary simulator arguments are accepted;
- the model manifest and lock include the expected and resolved content hash, resource reference, entry, compatibility mode, and redistribution policy in deterministic order;
- schematic JSON/SVG show the component/model identity and provenance but never the model body;
- KiCad and LTspice exports preserve the external reference and provenance. They do not embed a `prohibited` model body. Export metadata must make the required sidecar dependency explicit.

If an output location cannot preserve a usable reference without copying a prohibited model, Kessetsu reports that dependency instead of silently copying or embedding it.

### Versioning

Adding external-resource fields changes serialized domain contracts. The compile, model-manifest, lock, and export schemas must be bumped wherever their payload changes. Old `.kess` files remain valid and byte-stable. New syntax must fail predictably on older binaries through the existing parse/schema behavior.

## Security and trust statement

Hash and structure validation establish identity and wiring, not safety or correctness. A third-party model is simulator input and may use expensive or unsupported constructs. Kessetsu runs it only after explicit local binding, within existing timeout/cancellation and isolated-directory controls. This feature does not claim electrical accuracy, hardware validation, or license permission.

## Acceptance evidence

The implementation is complete only when tests cover missing resource, invalid path, changed hash, malformed/ambiguous entry, pin-count/order mismatch, namespace collision, unsupported compatibility mode, metadata/directive injection, no-content leakage, deterministic lock output, native staging cleanup, and the exact local OPA197 U6 path. The original U6 unsupported report remains historical evidence; a separately versioned three-attempt follow-up records the new result.
