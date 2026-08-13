# Schematic Quality Candidate — 2026-08-13

- Status: Agent-reviewed candidate; owner review and export parity are still open
- Generator: Core/CLI release renderer and the exact same Core SVG embedded by Web Hub
- Harness: `scripts/capture-schematic-corpus.ps1 -IncludeWeb`
- Corpus: thirteen circuits; CLI SVG/PNG/Schematic JSON plus fixed-viewport Chromium screenshots
- Canonical local verification: `powershell -ExecutionPolicy Bypass -File scripts/verify.ps1` PASS on 2026-08-13 (Rust/WASM/Web/Chromium/audit/package/EDA smoke)

## What changed from the rejected baseline

- Local signal and feedback nets are explicit orthogonal wires. Supply, ground and a deliberately high-fan-out bus remain semantic labels.
- Placement follows typed source→sink flow and conventional upper/main/lower/power lanes rather than undirected BFS rows.
- Wheatstone, differential pair, diode clamp and single BJT/MOSFET stages use topology-derived constraints; recognition does not depend on component reference names.
- The power amplifier reads left-to-right as input/buffer → gain/error → driver → complementary class-B output → load.
- Engineering values, op-amp polarity marks, PNP geometry, MOSFET geometry and long internal-model presentation were corrected.
- Web Hub defaults to a dark neutral dotted grid which can be disabled. The grid is viewer chrome and is not baked into exported SVG/PNG/PDF.

## Machine evidence after remediation

| Circuit | Components | Wires | Labels | Bends | Aspect | Current hard gate |
|---|---:|---:|---:|---:|---:|---|
| minimal | 2 | 1 | 2 | 2 | 1.000 | PASS |
| RC filter | 3 | 2 | 2 | 6 | 1.462 | PASS |
| Wheatstone bridge | 6 | 9 | 3 | 2 | 1.400 | PASS |
| gain stage | 7 | 7 | 9 | 9 | 1.182 | PASS |
| high fan-out bus | 9 | 0 | 18 | 0 | 0.562 | PASS |
| power amplifier | 11 | 17 | 15 | 19 | 2.174 | PASS |
| inverting amplifier | 7 | 7 | 9 | 9 | 1.889 | PASS |
| differential pair | 8 | 7 | 7 | 8 | 1.500 | PASS |
| MOSFET common-source | 6 | 5 | 6 | 4 | 1.238 | PASS |
| RLC ladder | 7 | 8 | 4 | 8 | 2.769 | PASS |
| diode clamp | 6 | 5 | 6 | 5 | 1.250 | PASS |
| BJT common-emitter (unseen probe) | 6 | 5 | 6 | 4 | 1.238 | PASS |
| summing amplifier (unseen probe) | 9 | 9 | 10 | 13 | 1.545 | PASS |

For every local signal net in the corpus, explicit-wire coverage is `1000/1000`, local signal label count is zero, connectivity is verified and flow inversion count is zero. The intentionally global `BUS` fixture remains label-based by policy.

## Agent visual review

Scale: 1 = unacceptable, 3 = usable with notable issues, 5 = textbook/application-note quality.

| Circuit/family | Flow | Grouping | Wiring | Typography | Composition | Review note |
|---|---:|---:|---:|---:|---:|---|
| RC/minimal | 5 | 5 | 4 | 5 | 4 | Conventional source → series element → shunt/load form; remaining bends are orthogonal lead routing, not hidden connectivity. |
| Wheatstone | 5 | 5 | 5 | 5 | 5 | Recognizable balanced bridge with two vertical legs and a horizontal bridge element. |
| op-amp gain/inverting/summing | 5 | 5 | 4 | 5 | 4 | Inputs, summing node, polarity and feedback are visible; DC supply sources remain intentionally outside the signal lane. |
| differential pair | 5 | 5 | 5 | 5 | 4 | Mirrored pair, matched collector loads and shared tail are immediately recognizable. |
| BJT/MOSFET stages | 5 | 5 | 5 | 4 | 4 | Conventional rail-load-active-emitter/source stack; device model remains visible only when it is a meaningful external model. |
| RLC ladder | 5 | 5 | 4 | 5 | 4 | Main series path and parallel shunts are legible; no label islands. |
| diode clamp | 5 | 5 | 5 | 5 | 4 | Upper/lower clamp branches visibly meet the loaded signal node. |
| power amplifier | 5 | 5 | 4 | 5 | 4 | Four stages and local feedback paths are traceable at default Web fit; complementary output pair and load form one output stage. |

## Deliberately open before final acceptance

1. `QualityReport` still needs component reference/value text→wire/text collision accounting and normalized detour/alignment metrics; visual inspection currently covers this gap.
2. SVG/PNG/PDF and editable KiCad/LTspice projection parity must be rerun against this candidate before SQ-8 closes.
3. The project owner must inspect RC, gain-stage and power-amplifier locally in Web Hub and explicitly accept them. Until then this is a candidate, not the public-release golden baseline.
