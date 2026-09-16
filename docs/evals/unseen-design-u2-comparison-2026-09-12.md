# U2 Fresh-Context Comparison — 2026-09-12

Publication maintenance, 2026-09-16: account-specific and internal execution-planning prose was removed or generalized. Recorded task outcomes, timings, exclusions and model settings are unchanged. Historical recorded file digests identify the original editions, not this publication edit.

Status: first-model U2 checkpoint complete. This is one active-filter task family. Together with U1 it provides two bounded task-family samples, but it does not complete the six-task 4.6V gate or establish a general circuit-design success rate.

## Configuration

- Task/specification: `unseen-design-v1/U2`, SHA-256 `a43240117a88a5ea0fe3bcd04906dde67642f3e53f93ae3f9e53a09d97128bea`
- Harness: `agent-comparison-harness-v1.md`
- Model: `gpt-5.6-sol`
- Reasoning: `medium`
- Attempts: three fresh, isolated sessions per arm
- Limits: 30 minutes and 60 tool calls per attempt
- Simulator: bundled Ngspice 46, SHA-256 `86c9ea5f645ca919e305639fa7bdb522355364c424d14e197f1ade617feb3453`
- Kessetsu CLI: `kess 0.1.0`, SHA-256 `b45c9ac51aaff349e70dc2bc2bbcb66acc561ba9c1d821721ca0932cf91dd30f`
- Evaluator: U2 v2, SHA-256 `4e16c9913421bcb851aa8a5626b0c1da530be706f026e03c29c980eed3e60d78`
- Shared generic model file: `KESSETSU_OPAMP_V1`, SHA-256 `04fd6a60c8f4b0e4035b817d071c2a426693a4f6e75bb48ef2bd6c489870fdae`
- Scoring: evaluator-owned OP/AC/transient testbench, exact model/topology checks, an independent finite-gain complex-response calculation, and AC/transient agreement

Raw prompts, JSONL traces, stderr, attempt metadata, candidates, simulator evidence, and exports are retained locally under `.artifacts/agent-comparison-v1/`. That directory is intentionally gitignored because raw agent traces can contain machine-local paths. Candidate and trace hashes remain in each `attempt.json` record.

## Electrical results

| Arm | Attempt | Result | Gain at 100 Hz | Relative cutoff | DC offset | Settled peak | Elapsed | Tool calls | Input tokens | Cached input | Output tokens |
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Kessetsu | 1 | PASS | 2.996212 | 2004.85 Hz | 0 V | 0.149811 V | 291.538 s | 15 | 730,868 | 668,928 | 7,962 |
| Kessetsu | 2 | PASS | 2.996210 | 2004.27 Hz | 0 V | 0.149810 V | 323.609 s | 17 | 762,902 | 706,432 | 7,299 |
| Kessetsu | 3 | PASS | 2.996191 | 1999.32 Hz | 0 V | 0.149810 V | 268.910 s | 18 | 610,256 | 569,984 | 7,371 |
| Direct Ngspice | 1 | PASS | 2.996191 | 1999.32 Hz | 0 V | 0.149810 V | 214.694 s | 16 | 407,170 | 348,928 | 5,735 |
| Direct Ngspice | 2 | PASS | 2.996210 | 2004.27 Hz | 0 V | 0.149810 V | 175.520 s | 8 | 188,748 | 148,864 | 5,633 |
| Direct Ngspice | 3 | PASS | 2.996195 | 2000.30 Hz | 0 V | 0.149810 V | 153.554 s | 7 | 276,453 | 214,016 | 5,245 |

Both arms passed all immutable source, rail, load, supported-topology, exact-model, gain, cutoff, offset, transient-peak, independent-formula, and AC/transient-agreement checks in all three attempts. No requirement tampering or false acceptance occurred.

### Effort summary

| Metric | Kessetsu | Direct Ngspice | Interpretation |
|---|---:|---:|---|
| Electrical success | 3/3 | 3/3 | No U2 electrical-reliability advantage established |
| Median elapsed time | 291.538 s | 175.520 s | Direct was 116.018 s, or about 39.8%, faster in this sample |
| Elapsed range | 268.910–323.609 s | 153.554–214.694 s | All attempts stayed far below the limit |
| Median tool calls | 17 | 8 | Kessetsu used more calls, including checks and exports |
| Median input tokens | 730,868 | 276,453 | Kessetsu reported substantially more total input; most was cached |
| Median non-cached input | 56,470 | 58,242 | New input was similar; total-input difference was mostly cache accounting |
| Median output tokens | 7,371 | 5,633 | Kessetsu emitted more output in U2 |

Token fields are the Codex CLI's reported usage, not monetary cost. Cost was unavailable and remains recorded as `null`.

## Artifact usability

| Capability | Kessetsu | Direct Ngspice |
|---|---|---|
| Canonical source/netlist | 3/3 | 3/3 |
| SVG | 2/3 | 0/3 |
| PNG | 2/3 | 0/3 |
| Schematic IR JSON | 2/3 | 0/3 |
| KiCad schematic | 2/3 | 0/3 |
| LTspice schematic | 2/3 | 0/3 |
| Human-readable notes | 3/3 final accounts | 3/3 reports plus text-only schematic descriptions |
| Visual review | Both generated PNGs inspected; readable signal flow and feedback, clean labels, no visible overlaps | No graphical schematic was produced; no visual-quality claim is made |
| KiCad application smoke | Both generated files opened/netlisted; all 9 component references retained; 0 ERC errors and nine known `lib_symbol_issues` warnings per file | Not available |
| LTspice application smoke | Both generated files opened/netlisted with exit code 0; all 9 component references retained | Not available |

Kessetsu attempt 2 produced a valid source and compiled SPICE netlist but did not follow the prompt through the requested export workflow. This is recorded as 2/3 artifact delivery, not hidden and not converted into an electrical failure. The other two Kessetsu attempts produced all requested export formats without hand-authoring them. The direct arm accurately produced SPICE and text descriptions only; Ngspice itself has no schematic-layout exporter.

## Protocol and evidence notes

The U2 harness extension and evaluator v2 were frozen before the first scored U2 candidate was generated. Evaluator v2 accepts candidate `.print`, `.plot`, and `.save` directives as inert self-check output commands, matching the U1 policy; candidate commands are never executed by the evaluator. The distributed model file is covered by a test that requires byte-for-byte equality with the evaluator's exported model text. No U2 inference run was excluded, restarted, edited, or human-corrected.

The model is a finite-gain, single-pole linear template. These PASS results do not validate rail saturation, output-current limits, slew rate, supply current, noise, tolerances, temperature, a manufacturer part, or hardware behavior.

## Checkpoint decision

U2 shows equal electrical success and a clear automatic artifact/interoperability advantage, but Kessetsu was slower and delivered the full artifact set in only two of three attempts. Combined with U1, the current evidence satisfies the roadmap's preliminary requirement for repeatable practical benefit in two bounded task families; however, the 4.6V gate remains open until U3–U6 are run, all records are reviewed for false acceptance and unsupported cases. Second-model transferability remains unavailable and must stay explicit.
