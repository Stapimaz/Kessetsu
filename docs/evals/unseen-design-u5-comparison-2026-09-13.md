# U5 Fresh-Context Comparison — 2026-09-13

Status: first-model U5 checkpoint complete. This is one bounded fixed-load power-amplifier task family. U1–U5 now cover five task families, but they do not complete the six-task 4.6V gate or establish a general circuit-design success rate.

## Configuration

- Task/specification: `unseen-design-v1/U5`, SHA-256 `a43240117a88a5ea0fe3bcd04906dde67642f3e53f93ae3f9e53a09d97128bea`
- Harness: `agent-comparison-harness-v1.md`
- Model: `gpt-5.6-sol`
- Reasoning: `medium`
- Authentication: ChatGPT session; no API key or direct API calls
- Attempts: three fresh, isolated, scored sessions per arm
- Limits: 30 minutes and 60 tool calls per attempt
- Simulator: bundled Ngspice 46, SHA-256 `86c9ea5f645ca919e305639fa7bdb522355364c424d14e197f1ade617feb3453`
- Kessetsu CLI: `kess 0.1.0`, SHA-256 `b45c9ac51aaff349e70dc2bc2bbcb66acc561ba9c1d821721ca0932cf91dd30f`
- Evaluator: U5 v2, SHA-256 `2088f35fdf4cacc20ea6f96afd3227f1ec7bbf43bda14b368d8508c4967b7e46`
- Shared generic model bundle: `KESSETSU_POWER_AMPLIFIER_V1`, SHA-256 `a27b4d9468e9fbbffbb4c9ec385aa61010e8705fbf1a364253952c1a6d967d59`
- Scoring: evaluator-owned 2 µs/30 ms transient testbench, exact model/topology checks, independent 10–30 ms RMS-power and Fourier integration, and positive output-device dissipation integration

Raw prompts, JSONL traces, stderr, attempt metadata, candidates, simulator evidence, exports, and invalid runs are retained locally under `.artifacts/agent-comparison-v1/`. That directory is intentionally gitignored because raw agent traces can contain machine-local paths. Candidate and trace hashes remain in each scored `attempt.json` record.

## Electrical results

| Arm | Attempt | Result | Output power | THD | NPN dissipation | PNP dissipation | Elapsed | Tool calls | Input tokens | Cached input | Output tokens |
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Kessetsu | 1 | PASS | 0.998586 W | 0.057810% | 1.507562 W | 1.507544 W | 370.690 s | 22 | 669,979 | 580,352 | 9,078 |
| Kessetsu | 2 | PASS | 0.998586 W | 0.057810% | 1.507562 W | 1.507544 W | 307.532 s | 20 | 673,478 | 616,320 | 8,442 |
| Kessetsu | 3 | PASS | 0.998868 W | 0.057796% | 1.507725 W | 1.507695 W | 232.453 s | 20 | 694,657 | 650,496 | 7,002 |
| Direct Ngspice | 1 | PASS | 0.998868 W | 0.057796% | 1.507725 W | 1.507695 W | 365.938 s | 18 | 509,110 | 477,056 | 11,714 |
| Direct Ngspice | 2 | PASS | 0.998888 W | 0.057803% | 1.507718 W | 1.507752 W | 432.080 s | 23 | 762,443 | 676,992 | 11,031 |
| Direct Ngspice | 3 | PASS | 0.998868 W | 0.057796% | 1.507725 W | 1.507695 W | 298.296 s | 18 | 509,338 | 482,560 | 7,712 |

Both arms passed all immutable source, supply, 4 ohm load, exact-model, four-block topology, output-power, THD, and positive transistor-dissipation checks in every scored attempt. Small result differences come from the selected gain-resistor values. No requirement tampering or false acceptance occurred.

### Effort summary

| Metric | Kessetsu | Direct Ngspice | Interpretation |
|---|---:|---:|---|
| Electrical success | 3/3 | 3/3 | No U5 electrical-reliability advantage established |
| Median elapsed time | 307.532 s | 365.938 s | Kessetsu was 58.406 s, or about 16.0%, faster in this sample |
| Elapsed range | 232.453–370.690 s | 298.296–432.080 s | U5 required materially longer agent work than the simple U4 switch, but all attempts stayed below the limit |
| Median tool calls | 20 | 18 | Direct needed two fewer interactions at the median |
| Median input tokens | 673,478 | 509,338 | Direct used less total input in this sample |
| Median non-cached input | 57,158 | 32,054 | Direct also used less new input |
| Median output tokens | 8,442 | 11,031 | Kessetsu emitted less output |

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
| Human-readable notes | 3/3 final accounts, with an additional verification note in attempt 2 | 3/3 notes and text-only schematic descriptions |
| Visual review | All three PNGs inspected; readable left-to-right stage flow, clear nested feedback paths, legible labels/values, crisp BJT arrows, and no visible overlaps | Text-only schematic descriptions; no graphical visual-quality claim is made |
| KiCad application smoke | 3/3 opened/netlisted; all 11 component references retained; 0 ERC errors and 11 known `lib_symbol_issues` warnings per file | Not available |
| LTspice application smoke | 3/3 opened/netlisted with exit code 0; all 11 component references retained | Not available |

Kessetsu delivered the complete requested visual and editable export set in every scored U5 attempt without hand-authoring it. The direct workflow delivered valid SPICE, simulation evidence, and useful text documentation, but no graphical or editable EDA schematic.

## Protocol corrections and excluded runs

No excluded record is silently discarded:

1. Before scored U5 inference, the evaluator was versioned from v1 to v2 so prompt-permitted inert `.four`, `.print`, `.plot`, and `.save` self-check directives are ignored rather than rejected. The evaluator still replaces candidate analyses with its own testbench; no requirement, topology, model, threshold, or candidate was changed.
2. The first runner launch stopped before inference and before any tool call because Windows PowerShell encoded the Unicode prompt incompatibly on redirected stdin. Its metadata, prompt, empty trace, and stderr are preserved under `invalid-runs/u5-kessetsu-attempt-1-stdin-encoding-pre-inference/`.
3. The first attempted encoding fix used a `ProcessStartInfo` property unavailable in Windows PowerShell 5.1 and stopped before starting Codex. Its empty evidence directory is preserved under `invalid-runs/u5-kessetsu-attempt-1-stdin-encoding-property-pre-inference/`. The runner was then changed to write explicit UTF-8 bytes to the stdin base stream; all six scored attempts used that compatible path and unchanged task text.

The generic linear op-amp template omits rail saturation, output-current limiting, and supply-current branches. Driver power and total efficiency are therefore unavailable, not zero. The generic power-BJT models do not validate tolerance, temperature, thermal/package/SOA behavior, manufacturer fidelity, or hardware performance.

## Checkpoint decision

Continue to U6 with the same model, settings, and paired protocol. U5 establishes equal electrical success plus a fifth automatic artifact/interoperability result for Kessetsu, with a modest elapsed-time advantage despite higher input use. Alongside U4's opposite result, the evidence supports an interoperability benefit rather than a universal speed claim. The 4.6V gate remains open until U6 is run, all records are reviewed for false acceptance and unsupported cases, and the owner makes the final product-direction decision. Second-model transferability remains unavailable and must stay explicit.
