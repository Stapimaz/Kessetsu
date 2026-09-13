# U4 Fresh-Context Comparison — 2026-09-13

Status: first-model U4 checkpoint complete. This is one bounded low-side MOSFET load-driver task family. U1–U4 now cover four task families, but they do not complete the six-task 4.6V gate or establish a general circuit-design success rate.

## Configuration

- Task/specification: `unseen-design-v1/U4`, SHA-256 `a43240117a88a5ea0fe3bcd04906dde67642f3e53f93ae3f9e53a09d97128bea`
- Harness: `agent-comparison-harness-v1.md`
- Model: `gpt-5.6-sol`
- Reasoning: `medium`
- Authentication: ChatGPT session; no API key or direct API calls
- Attempts: three fresh, isolated, scored sessions per arm
- Limits: 30 minutes and 60 tool calls per attempt
- Simulator: bundled Ngspice 46, SHA-256 `86c9ea5f645ca919e305639fa7bdb522355364c424d14e197f1ade617feb3453`
- Kessetsu CLI: `kess 0.1.0`, SHA-256 `b45c9ac51aaff349e70dc2bc2bbcb66acc561ba9c1d821721ca0932cf91dd30f`
- Evaluator: U4 v2, SHA-256 `e87cb62a7bdc3aee78774c68199984f8e6a786dec0c00c6d0735ff785a5d0a23`
- Shared generic model file: `IRF540`, SHA-256 `ea513fc5f1dba4dee2daca180eef151ebe997e178cbe0780178998ddf6580038`
- Scoring: evaluator-owned 2 µs/10 ms transient testbench, exact model/topology checks, time-weighted final-four-period measurements excluding 50 µs around transitions, and independent drain/load KCL

Raw prompts, JSONL traces, stderr, attempt metadata, candidates, simulator evidence, exports, and invalid runs are retained locally under `.artifacts/agent-comparison-v1/`. That directory is intentionally gitignored because raw agent traces can contain machine-local paths. Candidate and trace hashes remain in each scored `attempt.json` record.

## Electrical results

| Arm | Attempt | Result | On current | Off-current peak | On-state drain peak | Settled dissipation | Elapsed | Tool calls | Input tokens | Cached input | Output tokens |
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Kessetsu | 1 | PASS | 99.944812 mA | 0.024789 µA | 6.627082 mV | 0.330944 mW | 236.467 s | 12 | 443,599 | 399,872 | 7,304 |
| Kessetsu | 2 | PASS | 99.944812 mA | 0.024789 µA | 6.627082 mV | 0.330944 mW | 177.455 s | 13 | 540,569 | 466,432 | 4,850 |
| Kessetsu | 3 | PASS | 99.944812 mA | 0.024789 µA | 6.627082 mV | 0.330944 mW | 168.313 s | 13 | 525,431 | 465,536 | 4,669 |
| Direct Ngspice | 1 | PASS | 99.944812 mA | 0.024789 µA | 6.627082 mV | 0.330944 mW | 117.981 s | 7 | 161,969 | 141,568 | 4,144 |
| Direct Ngspice | 2 | PASS | 99.944812 mA | 0.024789 µA | 6.627082 mV | 0.330944 mW | 120.789 s | 6 | 149,471 | 127,616 | 4,104 |
| Direct Ngspice | 3 | PASS | 99.944812 mA | 0.024789 µA | 6.627082 mV | 0.330944 mW | 189.149 s | 10 | 249,466 | 220,544 | 5,874 |

Both arms selected the same permitted direct-gate topology and passed all immutable source, supply, load, exact-model, topology, on-current, off-current, on-drain, settled-dissipation, and independent-KCL checks in every scored attempt. The identical measurements are expected because all six candidates reduce to the same fixed circuit and exact model. No requirement tampering or false acceptance occurred.

### Effort summary

| Metric | Kessetsu | Direct Ngspice | Interpretation |
|---|---:|---:|---|
| Electrical success | 3/3 | 3/3 | No U4 electrical-reliability advantage established |
| Median elapsed time | 177.455 s | 120.789 s | Direct was 56.666 s, or about 31.9%, faster in this sample |
| Elapsed range | 168.313–236.467 s | 117.981–189.149 s | All scored attempts stayed far below the limit |
| Median tool calls | 13 | 7 | Direct needed fewer interactions for this simple fixed topology |
| Median input tokens | 525,431 | 161,969 | Direct used substantially less total input in this sample |
| Median non-cached input | 59,895 | 21,855 | Direct also used less new input |
| Median output tokens | 4,850 | 4,144 | Direct emitted slightly less output |

Token fields are the Codex CLI's reported usage, not monetary cost. Cost was unavailable and remains recorded as `null`.

## Artifact usability

| Capability | Kessetsu | Direct Ngspice |
|---|---|---|
| Canonical source/netlist | 3/3 | 3/3 |
| SVG | 3/3 | 1/3, hand-authored |
| PNG | 3/3 | 0/3 |
| Schematic IR JSON | 3/3 | 0/3 |
| KiCad schematic | 3/3 | 0/3 |
| LTspice schematic | 3/3 | 0/3 |
| Human-readable notes | 3/3 final accounts | 3/3 reports or text schematic descriptions |
| Visual review | All three PNGs inspected; clean and readable low-side-switch layouts, legible labels, no visible overlaps, and crisp MOSFET symbols | Attempts 2–3 are text-only. Attempt 1's SVG is well-formed XML with a `720 × 520` canvas, but it was not rendered by the available local image viewer; no visual-quality claim is made |
| KiCad application smoke | 3/3 opened/netlisted; all four component references retained; 0 ERC errors and four known `lib_symbol_issues` warnings per file | Not available |
| LTspice application smoke | 3/3 opened/netlisted with exit code 0; all four component references retained | Not available |

Kessetsu delivered the complete requested export set in every scored U4 attempt without hand-authoring it. The direct workflow was faster and leaner for this deliberately simple topology, but its only graphical artifact was one hand-authored SVG and it produced no editable EDA schematic.

## Protocol corrections and excluded runs

No excluded record is silently discarded:

1. Before scored U4 inference, the evaluator was versioned from v1 to v2 so prompt-permitted inert `.print`, `.plot`, and `.save` self-check directives are ignored rather than rejected. The evaluator still replaces candidate analyses with its own testbench; no requirement, topology, model, threshold, or candidate was changed.
2. One direct attempt 3 inference produced an electrically passing candidate, but the Codex process reached the owner's usage limit before reporting complete usage/final metadata and exited unsuccessfully. It is excluded as an external quota interruption and preserved under `invalid-runs/u4-direct-attempt-3-quota-interruption/`. The numbered attempt was rerun after the stated reset time with unchanged inputs.

The generic nominal IRF540 model does not validate manufacturer fidelity, tolerance, temperature, thermal/package behavior, SOA, avalanche, EMC, or hardware performance. Settled-window dissipation deliberately excludes switching intervals and is not total switching or gate-driver loss.

## Checkpoint decision

Continue to U5 with the same model, settings, and paired protocol. U4 establishes equal electrical success and the fourth automatic artifact/interoperability result for Kessetsu, while also showing that direct Ngspice is materially faster and more token-efficient on a trivial fixed-topology switch. This is useful counterevidence against claiming that the DSL always improves design speed. The 4.6V gate remains open until U5–U6 are run, all records are reviewed for false acceptance and unsupported cases, and the owner makes the final product-direction decision. Second-model transferability remains unavailable and must stay explicit.
