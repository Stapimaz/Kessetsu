# U6 External-Model Follow-up — 2026-09-14

Status: **PASS after one disclosed post-run exporter correction.** Three fresh Kessetsu agents independently produced valid exact-model designs and passed the evaluator-owned electrical checks. Visual, provenance, and editable-artifact review then exposed an LTspice adapter defect: the first exports used the Kessetsu alias `OPA197` instead of the external file's real entry `OPAx197`. The candidate sources and electrical results were unaffected. The adapter was corrected, the original `.asc` files were retained in the gitignored evidence tree, and deterministic re-exports passed real KiCad/LTspice application checks. This correction is product-development evidence, not hidden agent success.

## Configuration

- Frozen follow-up specification: `docs/evals/unseen-design-u6-followup-v1.md`, SHA-256 `b46d817be10aea4b14592cf99c084cd1b72d501130fcc4810dbbb82aecf18938`
- Capability/harness checkpoint: commit `a4a0801`; original unsupported U6 evidence remains unchanged
- Model: `gpt-5.6-sol`, medium reasoning, fresh ephemeral Codex CLI sessions authenticated through ChatGPT; no API key, API call, network access, or human intervention during an attempt
- Codex CLI: `0.154.0-alpha.6.2`, SHA-256 `2271526227b06ca13ab2b975b88546460fc61b2a29225b6dda0fdc803024ccc9`
- Scored Kessetsu CLI: `kess 0.1.0`, SHA-256 `1bd44a4b49290223c2773410eeda30ccec47c51c3729005c43124ff7e25b84cb`
- Follow-up evaluator v1: SHA-256 `da55dc00b55bf828abf47963b8700ddcbce5a494e83fa49ffe30dfbc6191d925`
- Bundled Ngspice 46: SHA-256 `86c9ea5f645ca919e305639fa7bdb522355364c424d14e197f1ade617feb3453`
- User-supplied model: TI `OPAx197 PSpice Model (Rev. D)`, Final 1.3, SHA-256 `fc5b020e63346e511bd808bf41c856b0150b000bcf8a41fe00eeececb1f422a5`; retained locally under TI terms and never committed or emitted in a report/export
- Limits: 30 minutes and 60 tool calls per attempt

The follow-up evaluator compiles the file-based `.kess` candidate, requires exactly one `kessetsu.models.v2` external entry with the frozen hash/resource/entry/pins/runtime/redistribution policy, removes the candidate's include, and independently supplies the same verified model to its own OP/AC/transient testbench. It does not trust candidate assertions or simulation output.

## Outcomes

| Attempt | Outcome | Candidate SHA-256 | Gain at 1 kHz | Settled peak | Fundamental | DC offset | Elapsed | Tool calls | Input / cached / output tokens |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|
| 1 | PASS | `5b2a72b…31085` | 4.99999523 | 0.50009515 V | 0.49999954 V | 98.850 µV | 330.339 s | 16 | 644,665 / 585,472 / 7,340 |
| 2 | PASS | `a3e3e9a4…54328` | 4.99999523 | 0.50009515 V | 0.49999954 V | 98.850 µV | 496.938 s | 37 | 1,792,012 / 1,728,768 / 10,014 |
| 3 | PASS | `6a437d84…0d06` | 4.99999523 | 0.50009515 V | 0.49999954 V | 98.850 µV | 841.992 s | 56 | 3,749,103 / 3,640,576 / 16,852 |

Median elapsed time was 496.938 seconds, median tool use 37, median input 1,792,012 tokens (1,728,768 cached), and median output 10,014 tokens. These are follow-up execution measurements, not a new speed comparison: the direct arm was not rerun because its original exact-model result was already 3/3 PASS. Reported monetary cost remains unavailable.

All three independent candidates used a 40 kohm / 10 kohm non-inverting divider, the immutable 10 kohm load and ±6 V supplies, and the exact typed external declaration. Every evaluator record reports `authentic_manufacturer_model: true`, the exact content hash, `model_body_serialized: false`, and all four fixed/gain/peak/fundamental checks true. No generic substitution or immutable-requirement false acceptance occurred.

## Artifact and application review

Each attempt produced SVG, PNG, Schematic IR JSON, SPICE, KiCad, and LTspice artifacts plus the deterministic model lock. All three PNGs were opened at original resolution: signal flow, feedback divider, load, supplies, reference designators, and values were legible with no component/text collision or ambiguous connectivity. The power wiring is longer than a hand-optimized textbook drawing but remains readable and electrically explicit.

The initial KiCad artifacts opened in KiCad 10, retained every component, generated netlists, and produced zero ERC errors; each had seven already-characterized `lib_symbol_issues` warnings from portable embedded symbols. The corrected LTspice artifacts opened in LTspice 24.1.9 and generated complete netlists retaining every component, the relative `models/OPAx197.LIB` dependency, and the exact `OPAx197` subcircuit entry. Text-export scans found no manufacturer model-body marker outside the separately retained user-owned sidecar.

### Disclosed post-run correction

The three scored agents ran against commit `a4a0801`. Their original LTspice files used `SYMATTR Value OPA197`, the local typed alias, while the included library defines `.SUBCKT OPAx197`. The real application smoke exposed this semantic mismatch after all three electrical evaluations had passed. The original files and hashes remain under `.artifacts/agent-comparison-v1/U6-followup-v1/eda-review/*-pre-entry-fix.asc`. Core now derives the LTspice symbol value from `ExternalSubcircuit.metadata.entry`; a regression test distinguishes alias from entry. Only `.asc` files were deterministically regenerated after the fix. Candidate source, simulator input, evaluator output, PNG/SVG/Schematic JSON, and KiCad artifacts were not revised.

## Decision

The evidence-selected external-manufacturer-model blocker is closed. The original U6 result remains 3/3 correctly unsupported before the capability; this versioned follow-up is 3/3 exact-model electrical PASS with complete, reviewed, provenance-preserving artifacts after the disclosed adapter fix. Kessetsu's demonstrated position remains interoperability-first: it provides typed model identity, repeatable simulation evidence, schematics, and editable exports, but does not turn a macromodel result into hardware validation.

This result does not establish cross-model transferability, arbitrary manufacturer-model compatibility, general autonomous circuit design, or real-board correctness. Only `gpt-5.6-sol` was available, and the OPAx197 PSpice path covers its declared Ngspice compatibility mode and test conditions.
