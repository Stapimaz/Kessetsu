# U3 Fresh-Context Comparison — 2026-09-12

Publication maintenance, 2026-09-16: account-specific and internal execution-planning prose was removed or generalized. Recorded task outcomes, timings, exclusions and model settings are unchanged. Historical recorded file digests identify the original editions, not this publication edit.

Status: first-model U3 checkpoint complete. This is one bounded common-emitter-amplifier task family. U1–U3 now cover three task families, but they do not complete the six-task 4.6V gate or establish a general circuit-design success rate.

## Configuration

- Task/specification: `unseen-design-v1/U3`, SHA-256 `a43240117a88a5ea0fe3bcd04906dde67642f3e53f93ae3f9e53a09d97128bea`
- Harness: `agent-comparison-harness-v1.md`
- Model: `gpt-5.6-sol`
- Reasoning: `medium`
- Attempts: three fresh, isolated, scored sessions per arm
- Limits: 30 minutes and 60 tool calls per attempt
- Simulator: bundled Ngspice 46, SHA-256 `86c9ea5f645ca919e305639fa7bdb522355364c424d14e197f1ade617feb3453`
- Kessetsu CLI: `kess 0.1.0`, SHA-256 `b45c9ac51aaff349e70dc2bc2bbcb66acc561ba9c1d821721ca0932cf91dd30f`
- Evaluator: U3 v3, SHA-256 `31a4035f480542f78699ca7c8a4b99c2d93fc251cfa2aff76390ea5f1c11b479`
- Shared generic model file: `2N3904`, SHA-256 `5a872e760aa43eab7bc3ad042402e95a21ecbba7378d9c95426fd05929f6f546`
- Scoring: evaluator-owned OP/AC/transient testbench, exact model/topology checks, independent DC KCL, transient Fourier integration, and AC/transient agreement

Raw prompts, JSONL traces, stderr, attempt metadata, candidates, simulator evidence, exports, original evaluator records, and invalid runs are retained locally under `.artifacts/agent-comparison-v1/`. That directory is intentionally gitignored because raw agent traces can contain machine-local paths. Candidate and trace hashes remain in each scored `attempt.json` record.

## Electrical results

| Arm | Attempt | Result | Collector | Collector current | Gain at 1 kHz | Phase | Fundamental | Elapsed | Tool calls | Input tokens | Cached input | Output tokens |
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Kessetsu | 1 | PASS | 5.357446 V | 1.103804 mA | 10.078699 | -179.785° | 50.392008 mV | 224.104 s | 14 | 624,990 | 568,576 | 7,411 |
| Kessetsu | 2 | PASS | 4.172099 V | 1.072867 mA | 9.470818 | -179.818° | 47.353183 mV | 203.280 s | 12 | 525,332 | 470,528 | 6,601 |
| Kessetsu | 3 | PASS | 5.468693 V | 0.784736 mA | 10.028199 | -169.545° | 50.146480 mV | 171.040 s | 10 | 368,280 | 339,456 | 5,214 |
| Direct Ngspice | 1 | PASS | 4.717911 V | 0.995835 mA | 8.357061 | -178.164° | 41.762275 mV | 309.592 s | 20 | 669,510 | 627,200 | 9,043 |
| Direct Ngspice | 2 | PASS | 4.584911 V | 0.939381 mA | 10.624648 | -179.808° | 53.121929 mV | 235.512 s | 12 | 500,014 | 452,224 | 7,709 |
| Direct Ngspice | 3 | PASS | 5.667623 V | 0.774972 mA | 9.804889 | -179.873° | 49.023537 mV | 229.950 s | 12 | 487,210 | 449,152 | 7,578 |

Both arms passed all immutable source, supply, load, topology, exact-model, collector-bias/current, gain, inversion, fundamental-amplitude, independent-KCL, and AC/transient-agreement checks in all three scored attempts. No requirement tampering or false acceptance occurred.

### Effort summary

| Metric | Kessetsu | Direct Ngspice | Interpretation |
|---|---:|---:|---|
| Electrical success | 3/3 | 3/3 | No U3 electrical-reliability advantage established |
| Median elapsed time | 203.280 s | 235.512 s | Kessetsu was 32.232 s, or about 13.7%, faster in this sample |
| Elapsed range | 171.040–224.104 s | 229.950–309.592 s | All scored attempts stayed far below the limit |
| Median tool calls | 12 | 12 | Equal median interaction count |
| Median input tokens | 525,332 | 500,014 | Similar total-input scale; most input was cached |
| Median non-cached input | 54,804 | 42,310 | Direct used less new input in this sample |
| Median output tokens | 6,601 | 7,709 | Kessetsu emitted less output |

Token fields are the Codex CLI's reported usage, not monetary cost. Cost was unavailable and remains recorded as `null`.

## Artifact usability

| Capability | Kessetsu | Direct Ngspice |
|---|---|---|
| Canonical source/netlist | 3/3 | 3/3 |
| SVG | 3/3 | 0/3 |
| PNG | 3/3 | 0/3 |
| Schematic IR JSON | 3/3 | 0/3 |
| KiCad schematic | 3/3 | 0/3 |
| LTspice schematic | 3/3 | 0/3 |
| Human-readable notes | 3/3 final accounts | 3/3 reports plus text-only schematic descriptions |
| Visual review | All three PNGs inspected; readable left-to-right signal flow and bias network, clean labels, crisp BJT arrows, no visible overlaps | No graphical schematic was produced; no visual-quality claim is made |
| KiCad application smoke | 3/3 opened/netlisted; all 10 component references retained; 0 ERC errors and ten known `lib_symbol_issues` warnings per file | Not available |
| LTspice application smoke | 3/3 opened/netlisted with exit code 0; all 10 component references retained | Not available |

Kessetsu delivered the complete requested export set in every scored U3 attempt without hand-authoring it. The direct arm accurately produced SPICE and text descriptions only; Ngspice itself has no schematic-layout exporter.

## Protocol corrections and excluded runs

No excluded or corrected record is silently discarded:

1. One U3 Kessetsu preflight stopped before inference because the generalized runner used a PowerShell path form unsupported by Windows PowerShell 5.1. The empty run directory is preserved; the runner was fixed and its syntax checked. No model usage occurred.
2. One Kessetsu inference produced an electrically passing candidate but hit an external execution usage limit before export and final response. It is excluded as an external quota interruption and preserved under `invalid-runs/`. The numbered attempt was rerun after execution access resumed with unchanged inputs.
3. Direct attempt 1 initially received evaluator v2 `ERROR` because its permitted, inert `.four` self-check directive was rejected. The original Kessetsu v2 PASS and direct v2 ERROR records are preserved beside their attempts. Evaluator v3 accepts `.four` without executing it; it changes no requirement, topology, model, testbench, threshold, or candidate. Both unchanged first-pair candidates were rescored under v3, and all subsequent attempts used v3.

The generic nominal 2N3904 model does not validate tolerance, temperature, thermal behavior, a specific manufacturer part, preferred-value compliance, or hardware performance.

## Checkpoint decision

U3 again shows equal electrical success plus a repeatable automatic artifact/interoperability advantage; Kessetsu also had a modest elapsed-time advantage. The 4.6V gate remains open until U4–U6 are run, all records are reviewed for false acceptance and unsupported cases. Second-model transferability remains unavailable and must stay explicit.
