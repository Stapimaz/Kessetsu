# Agent Comparison Harness v1

Status: frozen before the first comparative candidate run. This harness implements the common protocol in `unseen-design-v1.md`; it does not alter any task requirement or acceptance threshold.

Preflight note, 2026-09-11: two process launches exited before inference while the Codex CLI argument boundary was characterized. The runner first passed the unsupported `-a` shorthand after `codex exec`, then combined mutually exclusive `--approve-for-me` and `--sandbox` options. Both zero-tool-call records are retained under `.artifacts/agent-comparison-v1/preflight-failures/`; neither is a candidate attempt or model run. A top-level `codex -a never -s workspace-write exec ...` form was then CLI-parse-checked before inference. No prompt, model setting, task requirement, or scoring rule changed.

Invalid-run note, 2026-09-11: the top-level option form parsed but the resulting agent tool sandbox was read-only. The agent calculated a plausible circuit but every workspace operation was rejected, so no candidate or evaluator result existed. The complete run, including reported usage, is retained under `.artifacts/agent-comparison-v1/invalid-runs/` and excluded from both arms. The runner was corrected to use `codex exec --approve-for-me` alone, whose CLI contract supplies the workspace-write sandbox and automatic approval review. This operational correction does not change the prompt or scoring contract.

Calibration note, 2026-09-11: the first successful Kessetsu-only sandbox run passed the independent U1 evaluator and produced readable/exportable artifacts, but exposed an arm-documentation asymmetry before any direct-arm run. The direct reference named `.ac`, while the Kessetsu reference did not name the equivalent `simulate ac` source directive. The run is retained under `.artifacts/agent-comparison-v1/calibration-runs/` and excluded from the paired sample. The equivalent Kessetsu analysis syntax below was added before starting either arm's scored attempts; electrical requirements and scoring remain unchanged.

Evaluator correction, 2026-09-11: the first paired direct candidate used the standard `.print ac vm(out)` directive explicitly permitted by this harness for self-checking. U1 evaluator v1 rejected the inert line even though it never executes candidate analysis/output commands. The original ERROR record is preserved. Evaluator v2 ignores `.print`, `.plot`, and `.save` alongside the already ignored analysis/measurement commands while continuing to reject include/model/control-boundary violations; regression coverage protects both sides. Both existing paired candidates are rescored with v2, and all later U1 attempts use v2.

Quota-interruption note, 2026-09-12: direct U1 attempt 2 reached the owner's ChatGPT/Codex usage limit while its first file-change operation was in progress. No candidate file was committed to the isolated workspace and no evaluator ran. The complete trace is retained under `.artifacts/agent-comparison-v1/invalid-runs/` and excluded from both arms as an external quota interruption. The same attempt number is rerun in a fresh session after the stated reset time without changing the prompt, model settings, tools, or evaluator.

## Purpose

The comparison asks whether the same fresh-context coding agent gains a repeatable practical advantage from Kessetsu over a direct Ngspice workflow. It is not a benchmark of different language models. Both arms use the same model, reasoning setting, task text, time limit, and tool-call limit. The only intended difference is the circuit toolchain.

## Frozen first-model configuration

- Model: `gpt-5.6-sol`
- Reasoning effort: `medium`
- Authentication: Codex CLI signed in with ChatGPT; no API key and no direct API invocation
- Session policy: fresh, ephemeral Codex CLI session for every attempt
- Network/search: not enabled
- Sandbox: `--approve-for-me` workspace-write, rooted at an isolated temporary directory
- Per-attempt limit: 30 minutes and 60 tool calls
- Repetitions: three independent attempts per task and arm
- First checkpoint: U1 only, three Kessetsu and three direct attempts

The first checkpoint is part of the final first-model sample. Stop after this checkpoint if the harness, evidence capture, or scoring boundary is defective; do not spend the remaining run budget until the defect is resolved.

## Information boundary

Each attempt receives only:

1. the common task prompt below;
2. an arm-specific, minimal tool reference;
3. the executable toolchain for that arm; and
4. an `AGENTS.md` file that forbids inspecting paths outside the isolated workspace.

Finished repository examples, evaluator implementation, previous candidates, other attempts, and the other arm's artifacts are not provided. The independent evaluator runs only after the agent exits. It replaces candidate analyses with its own testbench and never trusts candidate assertions.

## Common U1 prompt

> Design a loaded passive RC low-pass circuit. The immutable circuit and acceptance requirements are: one ideal 1 V AC source, one series resistor, one shunt capacitor, and a fixed 100 kohm load. The series resistance must be between 1 kohm and 10 kohm. DC gain must be at least 0.95. The -3 dB cutoff, measured relative to the DC/passband gain, must be from 1.45 kHz through 1.75 kHz. The independent evaluator uses an AC sweep from 10 Hz through 1 MHz at 100 points per decade. Do not change or omit the source, load, topology, or limits. Work only in the supplied directory. You may calculate and simulate iteratively. Preserve each materially different candidate revision in `revisions/` before replacing it. Write the final circuit to the required candidate filename. Produce the schematic and editable/export artifacts that your supplied workflow genuinely supports; missing capability must be reported rather than fabricated. Finish with a short account of calculations, checks, limitations, and produced files. Do not inspect parent directories or search for Kessetsu repository examples, evaluator code, prior attempts, or another arm's work.

## Kessetsu arm tool reference

- Executable: `./kess.exe`
- Final source: `candidate.kess`
- Available declarations: `net NAME`, `source NAME ac(1V)`, `resistor NAME VALUE`, `capacitor NAME VALUE`, `simulate op`, and `simulate ac dec 100 10Hz 1MHz`.
- Pins: source `plus/minus`; resistor and capacitor `p1/p2`.
- Connect one or more comma-separated pins with `connect A.pin,B.pin to NET`.
- Useful commands:
  - `./kess.exe check candidate.kess --format json`
  - `./kess.exe compile candidate.kess --format json --include spice --output candidate.spice`
  - `./kess.exe simulate candidate.kess --format json --include datasets --force` (the source must contain the desired `simulate ...` directives)
  - `./kess.exe render candidate.kess --output schematic.svg`
  - `./kess.exe render candidate.kess --output schematic.png --scale 3`
  - `./kess.exe export candidate.kess --target kicad --output candidate.kicad_sch`
  - `./kess.exe export candidate.kess --target ltspice --output candidate.asc`
- Existing outputs are not overwritten unless `--force` is supplied.

## Direct arm tool reference

- Executable: `./tools/ngspice/bin/ngspice_con.exe`
- Final source: `candidate.cir`
- The first SPICE line is a title. Devices use `Vname positive negative AC magnitude`, `Rname node1 node2 value`, and `Cname node1 node2 value`; ground is node `0`; finish with `.end`.
- A candidate may contain `.op`, `.ac`, `.control`, and measurement/output commands for its own iteration. The evaluator ignores those commands and supplies its own analyses.
- Batch invocation: `./tools/ngspice/bin/ngspice_con.exe -n -b candidate.cir`.
- Ngspice does not itself provide a schematic-layout exporter. The agent may create a truthful schematic or editable artifact using only supplied local tools, but must not disguise a hand-authored or unsupported artifact as an EDA-verified export.

## Common U2 prompt

> Design a non-inverting active low-pass circuit using the supported topology: one or two passive input RC low-pass sections feeding the non-inverting input, a resistive non-inverting feedback divider, one declared `KESSETSU_OPAMP_V1` generic op-amp, and a fixed 10 kohm output load. The immutable sources are +6 V and -6 V rails plus a 50 mV-peak, 100 Hz sine input that is also a 1 V AC stimulus. Gain at 100 Hz must be from 2.85 through 3.15; the -3 dB cutoff relative to the 100 Hz gain must be from 1.8 kHz through 2.2 kHz; output DC offset magnitude must be below 20 mV; and settled output peak must be below 0.2 V. The independent evaluator runs OP, AC from 10 Hz through 1 MHz at 100 points per decade, and transient through 100 ms with a maximum 2 us step, measuring the final 50 ms. Do not change or omit the sources, load, supported topology, model identity, or limits. The supplied generic model is intentionally linear and does not prove rail saturation, current limits, supply consumption, manufacturer-part behavior, or hardware performance. Work only in the supplied directory. You may calculate and simulate iteratively. Preserve each materially different candidate revision in `revisions/` before replacing it. Write the final circuit to the required candidate filename. Produce the schematic and editable/export artifacts that your supplied workflow genuinely supports; missing capability must be reported rather than fabricated. Finish with a short account of calculations, checks, model limitations, and produced files. Do not inspect parent directories or search for Kessetsu repository examples, evaluator code, prior attempts, or another arm's work.

## U2 Kessetsu arm tool reference

- Executable: `./kess.exe`; final source: `candidate.kess`.
- The exact generic model is built into Kessetsu as `KESSETSU_OPAMP_V1`; `./models/KESSETSU_OPAMP_V1.lib` is supplied only so both arms can inspect identical model data. Do not redeclare or alter the built-in model.
- Available declarations include `net NAME`, `source NAME VALUE`, `source NAME sine_ac(0V,50mV,100Hz,1V)`, `resistor NAME VALUE`, `capacitor NAME VALUE`, and `opamp NAME KESSETSU_OPAMP_V1`.
- Pins: source `plus/minus`; resistor/capacitor `p1/p2`; op-amp `in_p/in_n/vcc/vee/out`.
- Connect pins with `connect A.pin,B.pin to NET`. A +6 V rail uses a 6 V source from VCC to GND; a -6 V rail uses a 6 V source from GND to VEE.
- Available analyses: `simulate op`, `simulate ac dec 100 10Hz 1MHz`, and `simulate tran 2us 100ms`.
- Useful commands: `check`, `compile --include spice`, `simulate --include datasets`, `render` to SVG/PNG, and `export --target kicad|ltspice`, using the same forms documented in the U1 reference above.
- Existing outputs are not overwritten unless `--force` is supplied.

## U2 Direct arm tool reference

- Executable: `./tools/ngspice/bin/ngspice_con.exe`; final source: `candidate.cir`.
- `./models/KESSETSU_OPAMP_V1.lib` contains the exact model used by the other arm. The final candidate must embed that exact `.subckt` text; do not alter it and do not leave the final candidate dependent on `.include`.
- The first SPICE line is a title. Use `VIN in 0 SIN(0 0.05 100) AC 1`, a +6 V source from `vcc` to ground, and a +6 V source from ground to `vee`. Resistors use `Rname node1 node2 value`; capacitors use `Cname node1 node2 value`.
- Instantiate the op-amp as `Xname in_p in_n vcc vee out KESSETSU_OPAMP_V1`; the node order is immutable.
- The final topology must use one or two passive input RC sections, a resistive non-inverting feedback divider, and the fixed 10 kohm load.
- A candidate may contain `.op`, `.ac`, `.tran`, `.control`, and measurement/output commands for its own iteration. The evaluator ignores those commands and supplies its own analyses.
- Batch invocation: `./tools/ngspice/bin/ngspice_con.exe -n -b candidate.cir`.
- Ngspice has no schematic-layout exporter. Produce only artifacts supported by supplied local tools and label hand-authored output honestly.

## Common U3 prompt

> Design a biased common-emitter amplifier using exactly one 2N3904 with the supplied generic model, a 9 V supply, divider bias, an unbypassed emitter resistor, input and output coupling capacitors, and a fixed 10 kohm AC-coupled output load. The immutable input is an ideal 5 mV-peak, 1 kHz sine that is also a 1 V AC stimulus. Collector bias must be from 3 V through 6 V; collector DC current from 0.5 mA through 2 mA; 1 kHz small-signal gain magnitude from 8 through 15 with inversion; and settled output fundamental amplitude from 35 mV through 80 mV. Do not use an ideal controlled-source amplifier or change the source, supply, load, device/model, supported topology, or limits. The independent evaluator runs OP, AC from 10 Hz through 1 MHz at 100 points per decade, and transient through 10 ms with a maximum 2 us step, measuring the final five periods. The generic nominal model does not prove tolerance, temperature, thermal, manufacturer-part, or hardware performance; preferred-value compliance is not required in v1. Work only in the supplied directory. You may calculate and simulate iteratively. Preserve each materially different candidate revision in `revisions/` before replacing it. Write the final circuit to the required candidate filename. Produce the schematic and editable/export artifacts that your supplied workflow genuinely supports; missing capability must be reported rather than fabricated. Finish with a short account of calculations, checks, model limitations, and produced files. Do not inspect parent directories or search for Kessetsu repository examples, evaluator code, prior attempts, or another arm's work.

## U3 Kessetsu arm tool reference

- Executable: `./kess.exe`; final source: `candidate.kess`.
- The exact generic model is built into Kessetsu as `2N3904`; `./models/2N3904.lib` is supplied only so both arms can inspect identical model data. Do not redeclare or alter the built-in model.
- Available declarations include `net NAME`, `source NAME VALUE`, `source NAME sine_ac(0V,5mV,1kHz,1V)`, `resistor NAME VALUE`, `capacitor NAME VALUE`, and `transistor NAME npn 2N3904`.
- Pins: source `plus/minus`; resistor/capacitor `p1/p2`; NPN transistor `c/b/e`.
- Connect pins with `connect A.pin,B.pin to NET`. A +9 V rail uses a 9 V source from the rail to GND.
- Available analyses: `simulate op`, `simulate ac dec 100 10Hz 1MHz`, and `simulate tran 2us 10ms`.
- Useful commands: `check`, `compile --include spice`, `simulate --include datasets`, `render` to SVG/PNG, and `export --target kicad|ltspice`, using the forms documented in the U1 reference above.
- Existing outputs are not overwritten unless `--force` is supplied.

## U3 Direct arm tool reference

- Executable: `./tools/ngspice/bin/ngspice_con.exe`; final source: `candidate.cir`.
- `./models/2N3904.lib` contains the exact model used by the other arm. The final candidate must embed that exact `.model` line; do not alter it and do not leave the final candidate dependent on `.include`.
- The first SPICE line is a title. Use `VIN in 0 SIN(0 0.005 1000) AC 1` and a +9 V source from the supply rail to ground. Resistors use `Rname node1 node2 value`; capacitors use `Cname node1 node2 value`; instantiate the transistor as `Qname collector base emitter 2N3904`.
- The final topology must contain five resistors: collector, emitter, upper/lower base-divider, and the fixed 10 kohm load; plus exactly two input/output coupling capacitors and one transistor.
- A candidate may contain `.op`, `.ac`, `.tran`, `.control`, and measurement/output commands for its own iteration. The evaluator ignores those commands and supplies its own analyses.
- Batch invocation: `./tools/ngspice/bin/ngspice_con.exe -n -b candidate.cir`.
- Ngspice has no schematic-layout exporter. Produce only artifacts supported by supplied local tools and label hand-authored output honestly.

## Common U4 prompt

> Design a low-side switched resistive-load driver using exactly one IRF540 with the supplied generic model, a fixed 12 V supply, a fixed 120 ohm resistor load from the supply rail to the drain, and the MOSFET source/body grounded. The immutable gate stimulus is `PULSE(0,10 V,0,1 us,1 us,500 us,1 ms)`. Direct gate drive is allowed; alternatively use at most one series gate resistor and at most one gate-to-ground pull-down resistor. During settled ON intervals load current must be at least 95 mA and drain voltage at most 0.6 V; during settled OFF intervals load-current magnitude must be at most 100 uA; settled-window mean positive transistor dissipation must be below 0.1 W. The independent evaluator runs a 2 us-step transient through 10 ms and measures the final four periods while excluding 50 us around every switching transition. Do not change the source, supply, load, device/model, topology, timing, windows, or limits. This settled-window dissipation intentionally excludes switching loss; the generic nominal model does not prove tolerance, thermal, package, avalanche, gate-driver, manufacturer-part, or hardware performance. Preferred-value compliance is not required in v1. Work only in the supplied directory. You may calculate and simulate iteratively. Preserve each materially different candidate revision in `revisions/` before replacing it. Write the final circuit to the required candidate filename. Produce the schematic and editable/export artifacts that your supplied workflow genuinely supports; missing capability must be reported rather than fabricated. Finish with a short account of calculations, checks, exclusions, model limitations, and produced files. Do not inspect parent directories or search for Kessetsu repository examples, evaluator code, prior attempts, or another arm's work.

## U4 Kessetsu arm tool reference

- Executable: `./kess.exe`; final source: `candidate.kess`.
- The exact generic model is built into Kessetsu as `IRF540`; `./models/IRF540.lib` is supplied only so both arms can inspect identical model data. Do not redeclare or alter the built-in model.
- Available declarations include `net NAME`, `source NAME VALUE`, `source NAME pulse(0V,10V,0s,1us,1us,500us,1ms)`, `resistor NAME VALUE`, and `mosfet NAME IRF540`.
- Pins: source `plus/minus`; resistor `p1/p2`; NMOS `d/g/s` (body is tied to source by the backend model contract).
- Connect pins with `connect A.pin,B.pin to NET`. A +12 V rail uses a 12 V source from the rail to GND.
- Available analysis: `simulate tran 2us 10ms`.
- Useful commands: `check`, `compile --include spice`, `simulate --include datasets`, `render` to SVG/PNG, and `export --target kicad|ltspice`, using the forms documented in the U1 reference above.
- Existing outputs are not overwritten unless `--force` is supplied.

## U4 Direct arm tool reference

- Executable: `./tools/ngspice/bin/ngspice_con.exe`; final source: `candidate.cir`.
- `./models/IRF540.lib` contains the exact model used by the other arm. The final candidate must embed that exact `.model` line; do not alter it and do not leave the final candidate dependent on `.include`.
- The first SPICE line is a title. Use `VG in 0 PULSE(0 10 0 1u 1u 500u 1m)` and a +12 V source from the supply rail to ground. Resistors use `Rname node1 node2 value`; instantiate the MOSFET as `Mname drain gate source body IRF540`, with source and body grounded.
- The final topology contains the fixed 120 ohm load and may use direct gate drive or exactly one series gate resistor plus at most one gate-to-ground pull-down resistor. No other devices are supported.
- A candidate may contain `.op`, `.tran`, `.control`, and measurement/output commands for its own iteration. The evaluator ignores those commands and supplies its own analysis and settled windows.
- Batch invocation: `./tools/ngspice/bin/ngspice_con.exe -n -b candidate.cir`.
- Ngspice has no schematic-layout exporter. Produce only artifacts supported by supplied local tools and label hand-authored output honestly.

## Common U5 prompt

> Design a fixed-load power amplifier using the supported four-block signal path: a unity-gain input buffer, a non-inverting voltage-gain stage with a two-resistor feedback divider, an output-feedback error/driver stage, and a complementary emitter-follower output made from exactly one `KESSETSU_POWER_NPN_V1` and one `KESSETSU_POWER_PNP_V1`. Use exactly three `KESSETSU_OPAMP_V1` instances and exactly three resistors: the two gain-setting resistors and the fixed 4 ohm output load. The immutable supplies are +9 V and -9 V; the immutable input is a 100 mV-peak, 1 kHz sine that is also a 1 V AC stimulus. Output RMS power must be from 0.9 W through 1.1 W, THD must be below 3%, and each output transistor's mean positive dissipation must be below 2 W. The independent evaluator runs a transient with a maximum 2 us step through 30 ms and integrates 10–30 ms. Do not change or omit the source, supplies, load, supported topology, exact models, measurement window, or limits. The generic linear op-amp model has no rail saturation, output-current limit, or supply-current branches; therefore driver power and total efficiency are unavailable. The generic transistor models do not prove tolerance, thermal/package/SOA, manufacturer-part, or hardware behavior. Preferred-value compliance is not required in v1. Work only in the supplied directory. You may calculate and simulate iteratively. Preserve each materially different candidate revision in `revisions/` before replacing it. Write the final circuit to the required candidate filename. Produce the schematic and editable/export artifacts that your supplied workflow genuinely supports; missing capability must be reported rather than fabricated. Finish with a short account of calculations, checks, exclusions, model limitations, and produced files. Do not inspect parent directories or search for Kessetsu repository examples, evaluator code, prior attempts, or another arm's work.

## U5 Kessetsu arm tool reference

- Executable: `./kess.exe`; final source: `candidate.kess`.
- The exact generic models are built into Kessetsu as `KESSETSU_OPAMP_V1`, `KESSETSU_POWER_NPN_V1`, and `KESSETSU_POWER_PNP_V1`; `./models/KESSETSU_POWER_AMPLIFIER_V1.lib` is supplied only so both arms can inspect identical model data. Do not redeclare or alter the built-in models.
- Available declarations include `net NAME`, `source NAME VALUE`, `source NAME sine_ac(0V,100mV,1kHz,1V)`, `resistor NAME VALUE`, `opamp NAME KESSETSU_OPAMP_V1`, and `transistor NAME npn|pnp MODEL`.
- Pins: source `plus/minus`; resistor `p1/p2`; op-amp `in_p/in_n/vcc/vee/out`; BJT `c/b/e`.
- Connect pins with `connect A.pin,B.pin to NET`. A +9 V rail uses a 9 V source from VCC to GND; a -9 V rail uses a 9 V source from GND to VEE. In the complementary emitter follower, the NPN collector goes to VCC, the PNP collector to VEE, their bases share the driver node, and their emitters share the output node.
- Available analysis: `simulate tran 2us 30ms`.
- Useful commands: `check`, `compile --include spice`, `simulate --include datasets`, `render` to SVG/PNG, and `export --target kicad|ltspice`, using the forms documented in the U1 reference above.
- Existing outputs are not overwritten unless `--force` is supplied.

## U5 Direct arm tool reference

- Executable: `./tools/ngspice/bin/ngspice_con.exe`; final source: `candidate.cir`.
- `./models/KESSETSU_POWER_AMPLIFIER_V1.lib` contains the exact op-amp subcircuit and NPN/PNP model lines used by the other arm. The final candidate must embed that exact `.subckt` block and both exact `.model` lines; do not alter them and do not leave the final candidate dependent on `.include`.
- The first SPICE line is a title. Use `VIN in 0 SIN(0 0.1 1000) AC 1`, a +9 V source from `vcc` to ground, and a +9 V source from ground to `vee`. Resistors use `Rname node1 node2 value`; instantiate each op-amp as `Xname in_p in_n vcc vee out KESSETSU_OPAMP_V1`; instantiate each BJT as `Qname collector base emitter MODEL`.
- The final topology must use exactly three op-amps, the exact NPN/PNP pair, the fixed 4 ohm load, and only the two additional gain-setting resistors. The unity buffer feeds the non-inverting gain stage; that stage feeds an error/driver whose inverting input senses the final output; the driver's output drives both transistor bases; both emitters join at the loaded output.
- A candidate may contain `.op`, `.ac`, `.tran`, `.four`, `.control`, and measurement/output commands for its own iteration. The evaluator ignores those commands and supplies its own transient analysis and integration window.
- Batch invocation: `./tools/ngspice/bin/ngspice_con.exe -n -b candidate.cir`.
- Ngspice has no schematic-layout exporter. Produce only artifacts supported by supplied local tools and label hand-authored output honestly.

## Evidence retained per attempt

The runner refuses to overwrite an existing run directory and records:

- exact prompt and its SHA-256;
- task-specification SHA-256;
- model, reasoning effort, limits, arm, run ID, UTC timestamps, and elapsed time;
- Codex CLI, Kessetsu, and Ngspice versions and executable hashes where applicable;
- complete Codex JSONL event trace and stderr;
- the complete isolated workspace except for copied tool binaries;
- final candidate, revisions, simulator/compiler output, and generated exports;
- independent evaluator JSON and process status.

Token or cost fields remain explicitly unavailable if Codex does not report them. A CLI/runtime failure is an attempt outcome and is never silently rerun under the same run ID.
