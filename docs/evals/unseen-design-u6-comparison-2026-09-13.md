# U6 Fresh-Context Comparison — 2026-09-13

Publication maintenance, 2026-09-16: account-specific and internal execution-planning prose was removed or generalized. Recorded task outcomes, timings, exclusions and model settings are unchanged. Historical recorded file digests identify the original editions, not this publication edit.

Status: first-model U6 checkpoint complete. This task intentionally tests an authentic manufacturer macromodel boundary. Direct Ngspice completed the design; Kessetsu correctly reported that its current typed model system cannot represent the exact external PSpice subcircuit. This is an unsupported capability, not an electrical design failure or a passing Kessetsu result.

## Configuration

- Task/specification: `unseen-design-v1/U6`, SHA-256 `a43240117a88a5ea0fe3bcd04906dde67642f3e53f93ae3f9e53a09d97128bea`
- Harness: `agent-comparison-harness-v1.md`
- Model: `gpt-5.6-sol`
- Reasoning: `medium`
- Attempts: three fresh, isolated, scored sessions per arm
- Limits: 30 minutes and 60 tool calls per attempt
- Simulator: bundled Ngspice 46, SHA-256 `86c9ea5f645ca919e305639fa7bdb522355364c424d14e197f1ade617feb3453`
- Kessetsu CLI: `kess 0.1.0`, SHA-256 `b45c9ac51aaff349e70dc2bc2bbcb66acc561ba9c1d821721ca0932cf91dd30f`
- Evaluator: U6 v2, SHA-256 `ef125690e82bfad5c2143502231d7ca411f0dd5327830735a6585a41692b8791`
- Manufacturer model: Texas Instruments `OPAx197 PSpice Model (Rev. D)`, Final 1.3, dated 2022-06-23; `OPAx197.LIB` SHA-256 `fc5b020e63346e511bd808bf41c856b0150b000bcf8a41fe00eeececb1f422a5`
- Official archive: `SBOMA34D.ZIP`, reacquired from `https://www.ti.com/lit/zip/SBOMA34`; SHA-256 `9e55fcaa23d54cee025dda3fa9872b11c41c531f82e0802c2b93d51e943666e5`
- Scoring: exact model hash and five-pin identity, fixed topology/sources/load, evaluator-owned OP/AC/transient runs, and independent settled peak/fundamental checks under Ngspice PSpice compatibility mode

The official model was supplied locally to both arms and retained only in the gitignored evidence tree. Its copyright/proprietary notices remain intact. Kessetsu does not commit, bundle, or redistribute the model, and the evaluator evidence omits its body.

## Outcomes

| Arm | Attempt | Outcome | Candidate | Gain at 1 kHz | Settled peak | Fundamental | DC offset | Elapsed | Tool calls | Input tokens | Cached input | Output tokens |
|---|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Kessetsu | 1 | UNSUPPORTED | No | — | — | — | — | 96.165 s | 6 | 152,060 | 125,696 | 2,345 |
| Kessetsu | 2 | UNSUPPORTED | No | — | — | — | — | 103.965 s | 8 | 187,060 | 172,544 | 2,956 |
| Kessetsu | 3 | UNSUPPORTED | No | — | — | — | — | 134.761 s | 14 | 209,846 | 183,808 | 2,953 |
| Direct Ngspice | 1 | PASS | Yes | 4.999991 | 0.500093 V | 0.499999 V | 98.743 µV | 301.737 s | 19 | 682,123 | 592,256 | 8,865 |
| Direct Ngspice | 2 | PASS | Yes | 4.999995 | 0.500095 V | 0.500000 V | 98.850 µV | 259.690 s | 17 | 646,033 | 580,352 | 7,873 |
| Direct Ngspice | 3 | PASS | Yes | 4.999995 | 0.500095 V | 0.500000 V | 98.850 µV | 331.779 s | 24 | 1,086,643 | 1,030,016 | 9,250 |

All three Kessetsu agents independently inspected the exact supplied model and CLI surface, identified that arbitrary external PSpice subcircuits cannot be imported, declined to create a generic substitute, made no electrical PASS claim, and produced no misleading candidate or export. All three direct candidates passed every immutable topology, source, supply, load, exact-model, gain, settled-peak, and transient-fundamental check. The evaluator recorded `authentic_manufacturer_model: true` and the exact supplied hash for every direct PASS. No false acceptance occurred.

### Effort summary

| Metric | Kessetsu | Direct Ngspice | Interpretation |
|---|---:|---:|---|
| Electrical success | Not evaluable; 3/3 capability failures | 3/3 PASS | Direct is the only successful U6 workflow |
| Median elapsed time | 103.965 s | 301.737 s | Not a speed comparison: Kessetsu stopped without designing or simulating |
| Elapsed range | 96.165–134.761 s | 259.690–331.779 s | Both stayed below the limit |
| Median tool calls | 8 | 19 | Kessetsu diagnosed the unsupported boundary rather than completing the task |
| Median input tokens | 187,060 | 682,123 | Outcome scope differs, so this is not an efficiency win |
| Median non-cached input | 26,038 | 65,681 | Same non-comparability applies |
| Median output tokens | 2,953 | 8,865 | Same non-comparability applies |

Token fields are the Codex CLI's reported usage, not monetary cost. Cost was unavailable and remains recorded as `null`.

## Artifact usability

| Capability | Kessetsu | Direct Ngspice |
|---|---|---|
| Valid final candidate | 0/3; unsupported by the current model boundary | 3/3 |
| Exact-model simulation evidence | 0/3 | 3/3 |
| Capability/provenance report | 3/3 | 3/3 design/provenance reports |
| Text schematic | 0/3, intentionally not fabricated | 3/3 |
| SVG / PNG / Schematic IR JSON | 0/3 | 0/3 |
| KiCad / LTspice editable schematic | 0/3 | 0/3 |
| Visual or EDA application review | Not applicable; no successful graphical/editable artifact exists | Not applicable; text only |

U6 does not weaken the earlier evidence that Kessetsu automates artifacts for circuits it supports. It establishes that the supported circuit/model domain is presently too narrow for an authentic external manufacturer op-amp model, so no artifact advantage exists on this task.

## Protocol corrections and excluded runs

No excluded record is silently discarded:

1. Before scored U6 inference, the evaluator was versioned from v1 to v2 so prompt-permitted inert `.four`, `.print`, `.plot`, and `.save` self-check directives are ignored rather than rejected. No requirement, topology, model, threshold, or candidate was changed.
2. The runner verifies the official model hash before inference, supplies the same local file to both arms, and writes its path only into the temporary process environment used by the evaluator. The model body remains outside tracked repository output.
3. The first direct attempt 1 produced an evaluator-PASS candidate but reached an external execution usage limit before final metadata and usage were complete. It is excluded as an external quota interruption and preserved under `invalid-runs/u6-direct-attempt-1-quota-interruption/`. Attempt 1 was rerun after the reset with unchanged prompt, model, settings, and limits.

The manufacturer macromodel improves identity/fidelity over a generic template but still does not validate tolerance, board parasitics, thermal behavior, EMC, production variation, or hardware performance.

## Checkpoint decision

The six-task first-model comparison is complete. U6 selects a concrete release blocker: Kessetsu needs a typed, provenance-preserving external subcircuit reference/import path that can execute the unmodified OPAx197 PSpice model without embedding or redistributing it. Continue with an interoperability-first product direction, but do not close 4.6V or claim manufacturer-model support until that bounded capability is implemented and a versioned U6 follow-up passes. Second-model transferability remains unavailable and must stay explicit.
