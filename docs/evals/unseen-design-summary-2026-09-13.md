# Unseen-Design Gate Summary — 2026-09-13

Status: the six-task comparison with the first accessible model is complete. Product recommendation: **continue, interoperability-first**. The manufacturer-model blocker selected here was subsequently closed by the versioned 2026-09-14 U6 follow-up; the original results below remain unchanged.

## Comparative result

| Task | Family | Kessetsu electrical outcome | Direct electrical outcome | Median elapsed | Automatic full visual/editable Kessetsu set | Direct graphical/editable result |
|---|---|---|---|---|---:|---|
| U1 | Passive loaded RC filter | 3/3 PASS | 3/3 PASS | 139.899 s vs 187.333 s; Kessetsu faster | 3/3 | Three hand-authored SVGs; no EDA export |
| U2 | Active low-pass | 3/3 PASS | 3/3 PASS | 291.538 s vs 175.520 s; direct faster | 2/3 | Text only; no graphical/EDA export |
| U3 | Common-emitter amplifier | 3/3 PASS | 3/3 PASS | 203.280 s vs 235.512 s; Kessetsu faster | 3/3 | Text only; no graphical/EDA export |
| U4 | Low-side MOSFET driver | 3/3 PASS | 3/3 PASS | 177.455 s vs 120.789 s; direct faster | 3/3 | One hand-authored SVG, otherwise text; no EDA export |
| U5 | Fixed-load power amplifier | 3/3 PASS | 3/3 PASS | 307.532 s vs 365.938 s; Kessetsu faster | 3/3 | Text only; no graphical/EDA export |
| U6 | Authentic OPA197 macromodel | 3/3 UNSUPPORTED | 3/3 PASS | Not comparable because Kessetsu stopped at capability diagnosis | 0/3 | Text only; no graphical/EDA export |

Across the five task families supported by both workflows, both achieved 15/15 electrical PASS. Kessetsu produced 14/15 complete automatic SVG/PNG/Schematic-JSON/KiCad/LTspice sets, and every generated editable export opened/netlisted in its target application with all component references retained. Direct Ngspice produced no editable EDA schematic in 18 attempts and produced four hand-authored SVGs. Kessetsu was faster in three jointly successful families and direct was faster in two; neither workflow has a universal speed advantage.

U6 is intentionally separate from the 15/15 comparison. Direct Ngspice used the exact hash-verified TI model and passed 3/3. Kessetsu agents correctly refused generic substitution in 3/3 attempts because the current Core model boundary cannot import arbitrary external manufacturer subcircuits. Treating those short diagnostic runs as speed wins or electrical failures would be misleading.

## What the evidence supports

- Kessetsu's demonstrated incremental value is automatic, consistent, cross-format circuit representation and EDA interoperability after a supported circuit has been expressed—not a higher electrical success rate than a strong agent using direct Ngspice.
- The benefit repeats across five materially different supported families, exceeding the gate's minimum two-family practical-benefit threshold.
- The evaluator found no false acceptance of changed immutable requirements in the scored set. All protocol corrections and external quota interruptions are retained and disclosed in the per-task reports.
- The schematic engine generalized beyond its original fixtures: 14 fresh PNGs were reviewed, and corresponding KiCad/LTspice artifacts passed installed-application smoke checks.
- A provider-independent CLI remains strategically useful even as LLMs improve: the model can reason in either workflow, while Kessetsu supplies deterministic verification records and portable artifacts that the direct simulator does not automatically create.

## What the evidence does not support

- It does not show that Kessetsu makes an LLM more likely to reach a valid electrical design on currently supported tasks; both arms were perfect in this small bounded sample.
- It does not establish general circuit-design success, PCB layout quality, hardware correctness, market demand, or willingness to pay.
- U1–U5 use generic/idealized model boundaries; their PASS results are not manufacturer-part or hardware validation.
- Only `gpt-5.6-sol` at medium reasoning was available. Cross-model transferability was not tested and is not implied.
- The prompts expose bounded supported topologies. Results must not be generalized to arbitrary analog, RF, power, mixed-signal, or PCB design.

## Decision and release consequence

Continue the project with an interoperability-first positioning: Kessetsu should be the deterministic circuit-engineering layer that an AI agent or human uses to compile, simulate, verify, visualize, and export a design through one typed source of truth. Do not market it as an AI model, a universal autonomous circuit designer, or a guaranteed speedup.

The selected blocker is external manufacturer subcircuit support. Before 4.6V closes, implement a bounded typed reference/import contract that preserves exact content hash, provenance, pin order, license/redistribution status, and simulator capability; keeps Circuit IR authoritative; never treats raw model text as Kessetsu source; and fails closed when the model or required runtime mode is absent. Then run a versioned U6 follow-up without replacing the original negative result.

The owner previously directed the project to continue toward a complete private release candidate and deferred broad interface review until after the technical phases. This evidence supports that direction while narrowing the strongest public claim to the value actually demonstrated.

## Follow-up addendum — 2026-09-14

Rewritten commit `71d9634` added the bounded typed external-subcircuit path selected by this comparison. Three fresh `gpt-5.6-sol`/medium Kessetsu attempts then passed the exact hash-verified TI OPA197 evaluator 3/3 and produced complete automatic visual/editable artifact sets. Original PNGs passed visual review; KiCad opened/netlisted with zero ERC errors beyond the known embedded-symbol warnings. Real LTspice review found that the first exports used the model alias rather than the library entry; that adapter defect was fixed, regression-tested, disclosed, and the same immutable candidate sources were deterministically re-exported. Corrected files opened and preserved the exact entry and sidecar dependency. See `docs/evals/unseen-design-u6-followup-2026-09-14.md`. This closes the selected blocker without replacing the original 3/3 unsupported record or expanding the broader claims above.
