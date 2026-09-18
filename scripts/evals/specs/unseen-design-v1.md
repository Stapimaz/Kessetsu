# Unseen Design Evaluation v1

Status: specifications frozen on 2026-09-10, before candidate generation. No design trials have been run. Owner: Kessetsu maintainer. This is an evaluation protocol, not a passing-result record. Changes after the first trial require a new version; preserve failed attempts.

## Common protocol

Use the same model identifier, settings, agent harness, simulator version, available documentation/model data, and limits for both arms: Kessetsu and direct Ngspice plus scripts/EDA. Permit baseline automation. Each arm receives only its task and tool documentation, not finished repository benchmarks or another arm's candidate. Run in fresh workspaces. Use three independent attempts per task/arm, capped at 30 minutes and 60 tool calls each; record actual usage and stop reasons. These limits define an experiment, not a product development estimate.

The evaluator owns this specification and the testbench. The designer may change design components only. Fixed sources, loads, signal conditions, thresholds, required topology, and required model identities must be checked independently of candidate assertions. Reject altered or missing requirements. Preserve the complete candidate, generated netlist, simulator output, model files/hashes, and exports. Missing or unsupported measurements are not passes.

Reference checking uses raw simulator datasets and separate arithmetic: interpolate the -3 dB crossing relative to passband gain, compute AC ratios from complex voltages, and integrate transient squared voltage over time for RMS power. Test evaluator calculations against analytic passive circuits and deliberately failing traces before scoring agents. Record nominal simulation success separately from real-part/model adequacy and physical validation.

## Frozen tasks

| ID | Design task | Immutable testbench and acceptance |
|---|---|---|
| U1 | Design a loaded passive RC low-pass | Ideal 1 V AC source, 100 kohm load, one series resistor and one shunt capacitor; series R between 1 and 10 kohm. DC gain >=0.95; cutoff relative to DC gain 1.45–1.75 kHz. AC sweep 10 Hz–1 MHz, 100 points/decade. |
| U2 | Design a non-inverting active low-pass | One declared generic op-amp, ±6 V supplies, 10 kohm load, 50 mV-peak 100 Hz sine and 1 V AC stimulus in separate analyses. Gain 2.85–3.15 at 100 Hz; relative cutoff 1.8–2.2 kHz; output DC offset magnitude <20 mV; steady-state output peak <0.2 V. Record generic-model limitations explicitly. |
| U3 | Design a biased common-emitter amplifier | One 2N3904 generic model, 9 V supply, 10 kohm AC-coupled output load, ideal 5 mV-peak 1 kHz input. Collector bias 3–6 V, collector DC current 0.5–2 mA, 1 kHz small-signal gain magnitude 8–15 with inversion, steady-state output fundamental amplitude 35–80 mV. No ideal controlled-source amplifier substitutions. |
| U4 | Design a low-side switched load driver | One IRF540 generic NMOS, fixed 12 V supply, fixed 120 ohm resistor load, input pulse 0–10 V, 1 ms period, 50% duty, 1 us edges. Load current >=95 mA during settled ON and <=100 uA during settled OFF; drain <=0.6 V when ON; mean transistor dissipation <0.1 W. Measure over the final four of ten periods, excluding 50 us around transitions. |
| U5 | Design a fixed-load power amplifier | Fixed 4 ohm load, ±9 V supplies, 100 mV-peak 1 kHz input. Target output RMS power 0.9–1.1 W, THD <3%, each output transistor mean positive dissipation <2 W. At least a voltage-gain stage and complementary BJT output stage; do not alter the load, source, or acceptance limits. Transient <=2 us step for 30 ms; measure 10–30 ms. Report ideal/generic driver power separately; do not claim total hardware efficiency. |
| U6 | Design with a real manufacturer op-amp model | Non-inverting OPA197, ±6 V supply, 10 kohm load, 100 mV-peak 1 kHz source. Gain 4.75–5.25 at 1 kHz; settled output peak 0.475–0.525 V. Use an authentic manufacturer model with source URL, version/content hash and permitted use recorded before either arm runs. Both arms receive the same model. If the model cannot be acquired or the simulator cannot support it, report acquisition/capability failure. Substituting Kessetsu's generic op-amp is not success. |

For U2/U3/U6, run OP, AC 10 Hz–1 MHz at 100 points/decade, and transient with <=2 us step for at least ten input periods; evaluate the final five complete periods. Load, source, and supply values are immutable even when a candidate embeds its own testbench. Device models for U3/U4 must be identical in both arms, with their hashes recorded. Component values need not use preferred-number series in v1; report this limitation.

## Required record per attempt

- Task/spec version and SHA-256; arm; run ID; date; model ID and settings (unknown fields explicitly unknown).
- Harness/tool versions, budgets, model provenance, complete prompt and tool trace; all candidate revisions.
- Fixed-requirement integrity, independent metric results, unsupported conditions, and false-pass checks.
- Actual elapsed time, tool calls, human interventions, and reported token/cost usage where available.
- Final source/netlist, SVG/PNG and editable schematic; human readability review and EDA openability separately scored. Missing export capability is a workflow limitation, not a failed electrical measurement.

Compare per-task outcomes, median effort and observed spread; retain all failed attempts. At least two task families must show a repeatable practical benefit, with no accepted requirement tampering, before claiming incremental product value. A second accessible model checks transferability. Unavailable access remains an explicit open item. Results do not establish market demand; voluntary product feedback and willingness-to-pay evidence are separate.
