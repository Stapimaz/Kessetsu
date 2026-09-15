# Schematic Quality Candidate — 2026-08-14

- Status: **Accepted visual golden on 2026-08-20 after explicit owner review of RC, gain-stage and power-amplifier**
- Accepted implementation: rewritten commit `dbb02cf` (`Fix schematic and plot pointer geometry`)
- Remote verification: GitHub Actions run `31850078955`, successful on the accepted commit
- Source of truth: Circuit IR → `kessetsu.schematic.v1` → Core render/export projections
- Harness: `scripts/capture-schematic-corpus.ps1`
- Local evidence directory: `.artifacts/schematic-quality-candidate-2026-08-14`
- Corpus: thirteen circuits, each with Schematic JSON, SVG, PNG and fixed-viewport Web capture

## What this candidate fixes

The owner rejected the 2026-08-13 candidate after the real images exposed detached rail/ground glyphs, label islands, avoidable wire doglegs, weak supply composition and component text that could collide with or drift away from its symbol. The replacement addresses those failures in the common Core engine:

- semantic ground and supply glyphs are anchored at the exact typed pin and follow electrical rail direction;
- short aligned connections remain direct, local signal nets remain explicit, and true high-fan-out globals alone use signal labels;
- reference, value and model fields are versioned Schematic IR objects rather than SVG-only guesses;
- reference/value fields are optimized as a pair, with outward placement for repeated vertical banks;
- text-to-symbol, text-to-wire, text-to-text, text-to-label, detached-text and broken-pair conditions are hard quality gates;
- DC supply sources form compact power blocks outside the main signal lane;
- Web clears stale compiled artifacts immediately after source edits, preventing the previous circuit from being captured or displayed as the new result;
- Web uses a default-on, user-toggleable dark dotted grid that is not baked into engineering exports.
- Component fields receive deterministic eighth-grid typography refinement after coarse placement, keeping references and values close without moving symbols or routing.
- Web-only component hit areas link hover/click selection across a symbol and all of its reference/value/model fields; exported artwork remains unchanged.
- NPN and PNP emitter arrows use crisp, filled, polarity-correct triangular markers instead of open splayed polylines.
- small parallel two-net networks use shared horizontal rails, while a direct-feedback op-amp's grounded output load aligns below the output pin; both topologies have permanent regressions.

## Machine evidence

All thirteen circuits pass connectivity and the complete geometric/text quality gate. Every circuit has zero symbol, wire-symbol, label-symbol and text collisions, zero detached semantic labels, zero detached component text fields, zero component text pair violations, zero flow inversions and `1000/1000` explicit local-signal wire coverage.

| Circuit | Components | Wires | Labels | Bends | Crossings | Detour ‰ | Aligned direct ‰ | Bounds |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| minimal | 2 | 1 | 2 | 0 | 0 | 0 | 1000 | 18×8 |
| RC filter | 3 | 2 | 2 | 0 | 0 | 0 | 1000 | 24×10 |
| Wheatstone bridge | 6 | 9 | 3 | 2 | 0 | 0 | 1000 | 26×16 |
| gain stage | 7 | 7 | 9 | 7 | 0 | 0 | 1000 | 31×21 |
| high-fan-out bus | 9 | 0 | 18 | 0 | 0 | 0 | 1000 | 24×27 |
| power amplifier | 11 | 17 | 15 | 17 | 0 | 321 | 1000 | 54×29 |
| inverting amplifier | 7 | 7 | 9 | 7 | 0 | 0 | 1000 | 39×20 |
| differential pair | 8 | 7 | 7 | 2 | 0 | 0 | 1000 | 38×24 |
| MOSFET common-source | 6 | 5 | 6 | 1 | 0 | 0 | 1000 | 32×23 |
| RLC ladder | 7 | 8 | 4 | 0 | 0 | 0 | 1000 | 41×10 |
| diode clamp | 6 | 5 | 6 | 2 | 0 | 95 | 800 | 31×22 |
| BJT common-emitter | 6 | 5 | 6 | 1 | 0 | 0 | 1000 | 32×23 |
| summing amplifier | 9 | 9 | 10 | 7 | 0 | 0 | 1000 | 39×28 |

The diode clamp's `800/1000` aligned-direct coverage and `95‰` detour come from the intentionally shared clamp/load node. The power amplifier's `321‰` detour comes from explicit feedback and complementary-output routing; both remain collision-free and visually traceable rather than being hidden behind local labels.

## Projection parity

- SVG, PNG, PDF and Schematic JSON are generated directly from the same `Schematic` object.
- KiCad and LTspice consume the same component origins, orientations, pin anchors, wires and semantic labels; neither owns a second layout engine.
- `core/tests/exporter_contract.rs`: 5/5 tests pass across all seven advertised formats, including deterministic bytes, real signatures, fail-closed connectivity and complete benchmark component coverage.
- Installed-application smoke: RC, gain-stage and power-amplifier open in KiCad and LTspice, retain every component reference and produce netlists. KiCad ERC reports **0 errors**; its warnings are limited to the documented portable embedded `Kessetsu` symbol-library namespace not being present in the user's global library table.
- `webapp/tests/e2e/schematic-corpus.spec.ts`: all thirteen Core SVGs render on the actual Web canvas with verified quality and the dotted-grid contract. The test also covers linked component hover/click selection, generous symbol hit areas, empty/repeat/Escape clearing and caught the earlier stale schematic state during debounced source changes.
- Full local `scripts/verify.ps1` passed on 2026-08-14: Rust fmt/Clippy/tests, release and WASM builds, 10 Web unit tests, 11 Chromium E2E tests, dependency/security audits, external-agent replay, clean release smoke and installed KiCad/LTspice smoke.

## Agent visual review

The actual exported PNGs and Web captures were inspected after more than seven full or targeted iterations.

- Minimal, RC and RLC sheets now use straight primary paths and conventional shunt/load placement.
- Wheatstone, differential pair, diode clamp and single-transistor stages are immediately recognizable from topology and symmetry.
- Gain, inverting and summing stages keep polarity, summing node and feedback visible; reference/value fields remain paired when routing forces an alternate text pocket.
- The high-fan-out fixture intentionally uses a BUS label, but its two resistor banks place both text fields outward so ownership is unambiguous.
- The power amplifier reads left-to-right as input/buffer → gain/error → driver → complementary class-B output → load. Local feedback paths and the load remain explicit.

After the owner's second review, all thirteen exported PNGs and the linked-highlight Web capture were regenerated and inspected again. The reported distant text and BJT-arrow defects were resolved without changing accepted routing. A final follow-up placed right-side fields clear of rotated symbol overhang, centered NPN/PNP arrows on their emitter branches, made schematic wheel zoom pointer-anchored, and corrected result-plot pointer mapping. The owner explicitly accepted the RC, gain-stage and power-amplifier schematics on 2026-08-20.

## Acceptance record

1. The full local canonical gate passed on the accepted implementation, including 13 Chromium E2E tests, release smoke and installed KiCad/LTspice smoke.
2. Remote CI run `31850078955` passed on exact rewritten commit `dbb02cf`.
3. The owner accepted all three required Web examples. This acceptance freezes the current output as the regression baseline; it does not prohibit future schematic improvements or fixes when a new defect is reported.
