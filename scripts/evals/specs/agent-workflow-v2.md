# Agent Workflow Evaluation v2

Status: M1 specification frozen before candidate generation. This protocol measures one
multi-condition design workflow; it does not establish general circuit-design superiority,
hardware validity or market demand. Preserve failures and revise the protocol version after any
candidate has seen the prompt.

## Common prompt

> Design a robust loaded voltage divider for a 12 V monitoring input. The final circuit must
> contain exactly one ideal DC source, one upper design resistor from the source node to OUT, one
> lower design resistor from OUT to ground, and a fixed nominal 100 kohm load from OUT to ground.
> The upper and lower resistor nominal values must each be an E24 value from 1 kohm through
> 100 kohm. Evaluate three supply values: 10.8 V, 12 V and 13.2 V. At every supply, evaluate the
> nominal components plus all eight independent uniform corners formed by upper resistor +/-1%,
> lower resistor +/-1% and load +/-10%, for exactly 27 cases. Every case must keep OUT from
> 2.75 V through 3.60 V inclusive, source-current magnitude at or below 350 uA, and dissipation
> in each design resistor below 4 mW. Do not relax or omit conditions, correlate tolerances,
> change the load nominal value, add devices or claim physical/hardware validation.
>
> Work only in the supplied directory. Use the supplied workflow to calculate and evaluate the
> design; do not inspect parent paths, prior attempts, repository examples or evaluator code.
> Preserve materially different candidates under `revisions/`. The final source must be the
> required candidate filename. Produce `study-results.json`, `summary.csv` and `REPORT.md` with
> the conditions, checks, assumptions and generated files. Produce every automatic schematic,
> BOM or editable EDA artifact genuinely supported by the supplied workflow; accurately report
> unavailable capability instead of hand-labeling an unsupported file. The independent evaluator
> ignores candidate PASS claims and recalculates every corner from the final topology and values.

## Kessetsu tool reference

- Executable: `./kess.exe`; final source: `candidate.kess`.
- Start with `./kess.exe capabilities --format json`. Use command `--help` for exact flags.
- Declare root parameters named `supply`, `upper`, `lower` and `load`, then use them in ordinary
  voltage-source and resistor declarations. Pins are source `plus/minus` and resistor `p1/p2`.
- The source should contain a short transient analysis and fixed assertions for output voltage,
  source current and design-resistor dissipation. The study varies the same complete source.
- Create `study.kessstudy.json`, then add a supply list axis and this independent corner block:

```json
"tolerances": {
  "mode": "corners",
  "parameters": [
    {"parameter": "upper", "relative": 0.01},
    {"parameter": "lower", "relative": 0.01},
    {"parameter": "load", "relative": 0.10}
  ]
}
```

- `study plan` must report 27 cases. Run it to `study-results.json`, export a summary CSV, and
  retain all cases. A nonzero run caused by a failed requirement is evidence, not permission to
  edit the limits.
- Supported automatic handoff includes SVG/PNG, KiCad, LTspice, BOM CSV and handoff JSON. Existing
  output files require explicit `--force`; generated source and evidence remain model-based.

## Direct tool reference

- Executable: `./tools/ngspice/bin/ngspice_con.exe`; final source: `candidate.cir`.
- The first SPICE line is a title. Use a literal 12 V source, two literal E24 design resistors and
  a literal 100 kohm load in the final source. Ground is node `0`; finish with `.end`.
- Ngspice batch invocation is `./tools/ngspice/bin/ngspice_con.exe -n -b FILE`. You may create a
  local Python or PowerShell corner runner. It must independently enumerate the stated 27 cases
  and write parseable `study-results.json` with a top-level `cases` array containing 27 case
  records and their status/conditions/results; also write `summary.csv`.
- Ngspice has no automatic schematic-layout, BOM or editable EDA exporter. Create only artifacts
  honestly supported by supplied local tools and state missing capability in `REPORT.md`.

## Scoring and evidence

Run one fresh checkpoint attempt per arm with the same `gpt-5.6-sol`/medium model, 15-minute and
40-tool-call limits. Continue to more attempts or a second task only if the harness and first
comparison answer a real uncertainty. Record elapsed time, observed calls, reported token usage,
human interventions, candidate/revision files and all generated evidence.

The independent evaluator validates exact topology, nominal values, E24 membership and every
declared corner using separate divider equations. Electrical status, 27-case evidence completeness,
automatic artifact delivery and truthful limitations are separate fields. A PASS is simulation/
calculation evidence for ideal nominal resistors under the declared finite corners, not a tolerance
distribution, production-yield, ADC-protection, safety or hardware claim.
