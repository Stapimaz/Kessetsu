# Schematic Quality Remediation Plan

- Status: Active — Phase 4 release blocker
- Opened: 2026-08-13
- Scope: Core-owned schematic analysis, placement, routing, rendering and every CLI/Web/export projection that consumes `kessetsu.schematic.v1`

## Why this work was reopened

The existing schematic pipeline proves electrical connectivity, deterministic output and a small set of geometry invariants. It does **not** yet prove professional drafting quality.

The current six-circuit test corpus passes because `QualityReport` checks symbol–symbol, wire–symbol and label–symbol collisions plus a permissive crossing limit. SVG hashes prove that the same drawing is produced again; they do not prove that the drawing is good. Browser E2E checks that the Core SVG is visible and reports `data-quality="pass"`; it does not inspect signal-flow clarity or page composition.

The current label policy also hides a central weakness: every non-signal net, and every explicitly named signal net with at least three pins, is represented by a separate label at every pin instead of an explicit local wire. This can reduce measured crossings and wire length while producing disconnected-looking islands.

Therefore the earlier Phase 4 visual-readability acceptance is reopened. Public release remains blocked until this plan is complete and the final benchmark sheets are explicitly reviewed.

## Reproducible baseline audit

The three Web Hub examples were rendered through the same Core/CLI path used by SVG/PNG/PDF export:

- `rc_filter.kess`: electrically understandable, but the source-to-resistor wire makes an unnecessarily large rectangular detour; the drawing is much wider than its information content; raw floating-point component values are not presentation quality.
- `gain_stage.kess`: the feedback loop is visually fragmented into repeated `OUT`/`FB` labels; supply sources, feedback resistors and load form distant islands; op-amp supply/model text overlaps; page utilization and alignment are poor.
- `power_amplifier.kess`: the four functional stages do not read as one left-to-right chain; buffer, gain, driver and output connections are mostly labels rather than visible wires; the third op-amp drops to another row without a clear reason; dual supplies dominate separate corners; output transistors and load do not form a recognizable class-B output stage.

These are not cosmetic nits. A schematic is an engineering explanation of a circuit. Correct connectivity hidden behind labels is not sufficient.

## Product-level quality contract

### Visual target

The target is not a generic node-link diagram and not merely an electrically valid auto-layout. It is the visual language of a carefully drafted engineering schematic: the kind of circuit diagram expected in an analog electronics textbook, application note or design review.

- The reader should recognize functional stages and conventional current/signal flow before reading net names.
- Main signal paths should be visually continuous; labels may simplify true global, repeated or distant connections but must not turn the circuit into disconnected islands.
- Symmetric and differential structures should look symmetric; feedback should visibly return to the correct stage; power should follow conventional top/bottom placement.
- References, values and optional model details should form a quiet information hierarchy rather than compete with the circuit itself.
- The Web viewer may use a dark neutral engineering-canvas background with a subtle dotted grid. This viewer chrome is user-configurable and independent from the exported schematic sheet: SVG/PNG/PDF retain explicit white/transparent technical-output options.

Schematic quality will have three independent layers. None substitutes for another.

### 1. Electrical hard gates

- Every connected Circuit IR pin is represented exactly once by a wire, junction or intentional global label.
- No net is invented, merged or split.
- Ground, supply and signal semantics remain typed.
- Output is deterministic across repeated compiles and declaration order.
- Every renderer/exporter consumes Core-owned Circuit IR/Schematic IR; Web does not implement layout semantics.

### 2. Geometric hard gates

- No symbol–symbol, wire–symbol, text–symbol, text–wire or text–text collision.
- No clipped symbol, label, value or model field.
- No accidental dangling visible wire, ambiguous T-junction or junction-less same-net branch.
- Local signal nets cannot pass by replacing every connection with labels.
- Bounds and scale must keep all content legible at the Web panel's reference viewport and in exported SVG/PNG/PDF.

### 3. Readability score and review gate

The structured quality report will measure at least:

- primary signal-path left-to-right monotonicity and stage-order inversions;
- explicit-wire coverage for local signal and feedback nets;
- label count and label-to-connected-pin ratio by net class;
- crossings, bends, total Manhattan wire length and avoidable detours;
- component alignment, stage cohesion and unexplained row changes;
- occupied-content ratio, whitespace imbalance and page aspect ratio;
- power/ground convention consistency;
- feedback-loop visibility;
- reference/value/model legibility and overlap margins.

Hard failures stop artifact generation. Soft metrics produce a deterministic score and per-issue diagnostics, but a numeric score alone cannot certify aesthetics. Golden hashes are regression locks only after a sheet is accepted; changing a hash is never evidence of improvement by itself.

## Target layout strategy

### A. Semantic graph analysis

1. Extend the shared component catalog with explicit pin/flow roles where the current `is_signal` boolean is insufficient: signal input, signal output/control, passive-through, power and reference.
2. Classify nets as primary signal, feedback, local interconnect, bias/control, supply, ground or high-fan-out global.
3. Discover likely source-to-output paths from typed topology, not component names such as `VIN` or `OUT` alone.
4. Separate feedback edges before layering, retain them as explicit constraints, and restore them through dedicated outer routing channels.
5. Group components into functional stages from graph adjacency and flow roles. Stage detection is a layout hint only and must never alter Circuit IR semantics.

### B. Constraint-based placement

1. Place the primary signal path from left to right.
2. Keep components of one stage close and align equivalent branches.
3. Put positive supply connections above, negative supply/ground below, and keep DC source symbols outside the main signal lane.
4. Choose orientation from typed entry/exit pins and neighboring placement cost.
5. Use deterministic layer ordering and bounded local improvement to minimize inversions, crossings, wire length and empty area.
6. Treat feedback paths, bridges and differential/symmetric branches as explicit constraints rather than BFS accidents.

The current breadth-first rank plus fixed “four items per column” placement is a baseline to replace, not an architecture to preserve.

### C. Routing and label policy

1. Prefer explicit orthogonal wires for the primary path, short local nets and feedback loops.
2. Reserve labels for true global rails, intentionally repeated named buses and cases where a wire would materially reduce readability.
3. Never label both ends of an ordinary two-point local connection.
4. Use junction-aware Steiner-style trunks for multi-pin local nets instead of sending every branch to a hub beyond the rightmost component.
5. Route feedback through stable outer channels and power/reference connections vertically.
6. Penalize crossings, bends, parallel near-overlaps, long detours and routes passing through text clearance boxes.
7. Place labels only after routing, with measured font bounds and deterministic alternative anchors.

### D. Rendering polish

1. Apply engineering-value formatting (`159 nF`, `55.6 kΩ`, `8 Ω`) instead of raw floating-point strings.
2. Define reference, value and optional model tiers; long model names must not collide with supply symbols or dominate the sheet.
3. Tune grid, stroke, font and symbol scale using the same Core SVG projection for Web, SVG, PNG and PDF.
4. Fit the canvas to useful content with controlled margins and a sane aspect ratio; do not stretch sparse layouts to fill arbitrary space.
5. Give the Web viewer a dark-gray, subtle dotted-grid canvas with grid visibility control; do not bake that viewer grid into canonical Schematic IR or default engineering exports.

## Iterative development and visual verification loop

Each layout iteration will follow the same observable loop:

1. Generate Schematic JSON, SVG and PNG for the canonical corpus through the CLI/Core path.
2. Produce fixed-viewport Web screenshots for the same sources; do not use a separate test renderer.
3. Emit a machine-readable quality report and fail hard invariants automatically.
4. Inspect the actual PNG/screenshots at readable resolution and record a short per-circuit scorecard: signal flow, grouping, wiring, labels, power convention, typography and composition.
5. Change one bounded part of analysis/placement/routing/rendering, regenerate the entire corpus, and compare metrics plus images against the previous accepted candidate.
6. Add a regression fixture for every newly discovered topology failure before accepting the algorithm change.
7. Update visual goldens only after the scorecard improves without electrical/geometric regression.

The review corpus will include the existing minimal, RC, Wheatstone, op-amp gain, high-fan-out and power-amplifier circuits, plus targeted fixtures for feedback, bridge symmetry, dual rails, differential branches, local multi-drop nets, long labels and mixed-orientation passives.

The agent can perform the generate → inspect → critique → revise cycle locally. CI will enforce deterministic electrical/geometric metrics and approved image baselines; it cannot replace the final engineering-readability review.

## Implementation order

- [x] SQ-1 — Add the reproducible capture/scorecard harness and save the current six-circuit baseline without accepting it as a golden quality target. _Implemented as `scripts/capture-schematic-corpus.ps1`; the corpus now contains thirteen circuits, including two unseen generalization probes, and the rejected baseline is recorded in `docs/evals/schematic-quality-baseline-2026-08-13.md`._
- [x] SQ-2 — Expand `QualityReport` with text collisions, explicit-wire coverage, flow/stage, label-use, compactness and routing metrics; make label-based metric gaming impossible. _Implemented as symbol/label/text/wire hard gates, semantic-label anchoring, local/global label counts, 1000-based explicit-wire and aligned-wire coverage, flow inversions, normalized detour, bend and composition metrics. Reference/value/model positions are now owned by the versioned Schematic IR rather than guessed by the SVG renderer._
- [x] SQ-3 — Add typed pin-flow/net-role analysis to the shared component catalog and prove it on passive, op-amp, transistor, bridge and dual-supply fixtures. _`PinFlow` is catalog-owned; thirteen-circuit regression plus direct catalog assertions cover passive, source, op-amp, BJT, MOSFET, bridge, clamp and dual-rail cases._
- [x] SQ-4 — Replace fixed-row BFS placement with deterministic constrained stage placement; first make RC and gain-stage sheets conventionally readable. _Typed source→sink ranks, placement lanes and topology-derived bridge/differential/single-transistor constraints replace the old undirected fixed-row behavior._
- [x] SQ-5 — Implement local trunk/branch routing, explicit feedback paths and the restricted semantic-label policy. _Every local signal pin in the thirteen-circuit corpus has 1000/1000 explicit-wire coverage; labels are restricted to supplies, ground and true high-fan-out global nets._
- [x] SQ-6 — Tune symbols, engineering values, text hierarchy, margins and viewport fitting; add an optional dark dotted-grid Web canvas without coupling it to exported sheet backgrounds. _Semantic rail/GND direction, Schematic IR-owned component text, collision-aware clearance, compact supply blocks and a default-on toggleable Web-only dotted grid are implemented. The Core corpus has been inspected through more than five real PNG iterations; current Web re-capture remains part of SQ-8._
- [x] SQ-7 — Iterate on the power-amplifier sheet until buffer → gain/error → driver → class-B output → load is visually traceable without reading the source code. _After the 2026-08-13 candidate was rejected in owner review, more than five actual PNG iterations corrected detached semantic glyphs, text-on-wire placement, feedback spacing and supply composition. The current Core candidate renders all four stages left-to-right with explicit feedback and a conventional complementary output pair._
- [x] SQ-8 — Verify SVG/PNG/PDF and Web parity, then verify KiCad/LTspice receive the accepted placement/connectivity without exporter-specific layout forks. _All seven exporters consume `kessetsu.schematic.v1`; the thirteen-circuit fixed-viewport Web capture passes; RC/gain/power open and netlist in installed KiCad/LTspice. KiCad reports zero errors and only the documented portable embedded-library table warnings. Evidence: `docs/evals/schematic-quality-candidate-2026-08-14.md`._
- [x] SQ-9 — Lock accepted corpus metrics/hashes/screenshots and run canonical local/remote verification. _The accepted scorecard is recorded in `docs/evals/schematic-quality-candidate-2026-08-14.md`; deterministic SVG hashes and geometric contracts lock the reproducible render baseline. The full local gate and remote CI run `31850078955` pass on accepted commit `5308f3b`._
- [x] SQ-10 — Obtain explicit owner review of the three Web examples before restoring the Phase 4 visual-quality acceptance checkboxes or publishing. _The owner explicitly accepted RC, gain-stage and power-amplifier on 2026-08-20; later reported defects remain valid regression work rather than invalidating the historical acceptance record._
  - [x] SQ-10a — Refine owner-reported distant reference/value fields with a sub-grid proximity contract while preserving accepted routing and collision-free sheets. _Completed with deterministic eighth-grid typography offsets and a visually inspected thirteen-circuit Core corpus; component and wire coordinates are unchanged._
  - [x] SQ-10b — Add linked Web hover/click highlighting for each component symbol and its reference/value/model fields without changing exported engineering artwork. _Completed with Web-only hit areas/linked styling and Chromium hover, click, clear and Escape coverage._
  - [x] SQ-10c — Replace the visually splayed BJT emitter arrow with symmetric polarity-correct geometry and verify both NPN and PNP projections. _Completed with filled triangular markers, NPN/PNP SVG regressions and Core/Web visual review._
  - [x] SQ-10d — Close the final owner-reported geometry and pointer-coordinate regressions without changing routing. _Right-side fields clear rotated symbol overhang, emitter-arrow centroids lie on their branches, schematic zoom is pointer-anchored, and result cursors map to the actual plot frame. Core contracts and Chromium regressions protect all four fixes._

The rejected 2026-08-13 candidate remains as historical evidence in `docs/evals/schematic-quality-candidate-2026-08-13.md`. The refined replacement in `docs/evals/schematic-quality-candidate-2026-08-14.md` became the accepted visual golden on 2026-08-20. Future reported defects should add focused regressions and a new dated candidate when they materially change the accepted output.

## Exit criteria

This remediation is complete only when:

- all electrical and geometric hard gates pass;
- RC, gain-stage and power-amplifier scorecards have no open major issue;
- the main path and feedback path of the power amplifier are visually traceable at the default Web zoom;
- the same accepted drawing is available from CLI and Web in every supported visual format;
- KiCad/LTspice connectivity remains verified and no exporter invents a separate layout engine;
- the owner has reviewed the local Web Hub and explicitly accepted the three example schematics.
