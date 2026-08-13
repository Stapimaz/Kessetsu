# Schematic Quality Baseline — 2026-08-13

- Status: Rejected baseline; diagnostic evidence only
- Generator: Core/CLI release render and `kessetsu.schematic.v1` export
- Harness: `scripts/capture-schematic-corpus.ps1`
- Corpus schema: `kessetsu.schematic-quality-corpus.v1`

## Machine evidence

| Circuit | Components | Wires | Labels | Bends | Aspect | Old quality gate |
|---|---:|---:|---:|---:|---:|---|
| minimal | 2 | 1 | 2 | 2 | 2.375 | PASS |
| RC filter | 3 | 2 | 2 | 6 | 3.875 | PASS |
| Wheatstone bridge | 6 | 9 | 3 | 28 | 1.615 | PASS |
| gain stage | 7 | 1 | 15 | 3 | 1.269 | PASS |
| high fan-out | 9 | 0 | 18 | 0 | 0.794 | PASS |
| power amplifier | 11 | 1 | 31 | 3 | 2.036 | PASS |
| inverting amplifier | 7 | 1 | 15 | 3 | 1.593 | PASS |
| differential pair | 8 | 4 | 10 | 18 | 0.706 | PASS |
| MOSFET common-source | 6 | 2 | 9 | 5 | 1.824 | PASS |
| RLC ladder | 7 | 2 | 10 | 3 | 3.235 | PASS |
| diode clamp | 6 | 1 | 10 | 3 | 1.320 | PASS |

The table makes the invalid acceptance visible: a multi-stage 11-component amplifier with one wire and 31 labels, and a fan-out circuit with no wires at all, both pass the old quality gate.

## Visual inspection scorecard

Scale: 1 = unacceptable, 3 = usable with notable issues, 5 = textbook/application-note quality.

| Circuit | Flow | Grouping | Wiring | Labels | Power convention | Typography | Composition | Major observations |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| minimal | 3 | 3 | 2 | 3 | 3 | 2 | 2 | Avoidable rectangular detour; raw numeric formatting. |
| RC filter | 3 | 3 | 2 | 3 | 3 | 2 | 2 | Excessive width and detour for a three-component circuit. |
| Wheatstone bridge | 1 | 1 | 1 | 3 | 2 | 2 | 1 | Does not resemble a bridge; long perimeter routes dominate the symbols. |
| gain stage | 1 | 1 | 1 | 1 | 2 | 1 | 1 | Feedback and output are fragmented into label islands; supply/model text overlaps. |
| high fan-out | 1 | 2 | 1 | 1 | 2 | 2 | 1 | Zero explicit wires; repeated BUS labels replace the actual distribution structure. |
| power amplifier | 1 | 1 | 1 | 1 | 2 | 1 | 1 | Four stages are not visually traceable; class-B pair and load do not read as one output stage. |
| inverting amplifier | 1 | 1 | 1 | 1 | 2 | 1 | 1 | Summing node and feedback loop are invisible; op-amp is separated from its resistors. |
| differential pair | 1 | 2 | 1 | 2 | 1 | 2 | 1 | Pair symmetry is lost; sources and collector loads are scattered; routing forms large frames. |
| MOSFET common-source | 1 | 1 | 1 | 1 | 2 | 2 | 1 | Drain/source network is not recognizable as a common-source stage. |
| RLC ladder | 2 | 1 | 1 | 1 | 3 | 2 | 1 | Series path stops after the inductor; shunt elements become disconnected-looking islands. |
| diode clamp | 1 | 1 | 1 | 1 | 2 | 1 | 1 | Clamp structure is invisible and source value overlaps ground geometry. |

## Cross-corpus root causes

1. Undirected breadth-first ranking treats feedback, supply and output branches as equivalent signal-flow edges.
2. Fixed four-row columns create arbitrary row changes and large unused areas.
3. Named three-pin signal nets are converted into labels at every pin; this destroys visual continuity while making routing metrics look artificially good.
4. Multi-pin routed nets use a hub beyond the rightmost component, producing large perimeter frames.
5. Component orientation is mostly based on “connected to ground” rather than functional topology.
6. Text bounds are not first-class layout obstacles; values/models overlap symbols and rail markers.
7. Existing quality checks cannot detect any of the failures above.

## Acceptance use

This document is the “before” evidence. It must not be copied into visual golden tests as an accepted target. Later iterations may update machine numbers in generated `.artifacts/schematic-quality/manifest.json`, while accepted scorecards will be stored as separate dated documents so quality progress remains auditable.

