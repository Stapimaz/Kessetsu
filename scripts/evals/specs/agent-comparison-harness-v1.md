# Agent Comparison Harness v1

## Purpose

The comparison asks whether the same fresh-context coding agent gains a repeatable practical advantage from Kessetsu over a direct Ngspice workflow. It is not a benchmark of different language models. Both arms use the same model, reasoning setting, task text, time limit, and tool-call limit. The only intended difference is the circuit toolchain.

## Frozen first-model configuration

- Model: `gpt-5.6-sol`
- Reasoning effort: `medium`
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

## Common U6 prompt

> Design a non-inverting gain-five amplifier around the authentic Texas Instruments `OPAx197` PSpice macromodel supplied locally as `./models/OPAx197.LIB`. Use exactly one OPAx197 instance and exactly three resistors: a two-resistor non-inverting feedback divider plus a fixed 10 kohm output load. The immutable supplies are +6 V and -6 V; the immutable input is a 100 mV-peak, 1 kHz sine that is also a 1 V AC stimulus. Gain at 1 kHz must be from 4.75 through 5.25; settled output peak and independently integrated fundamental amplitude must each be from 0.475 V through 0.525 V. The independent evaluator runs OP, AC from 10 Hz through 1 MHz at 100 points per decade, and a maximum-2-us-step transient through 10 ms, measuring the final five periods. Do not change or omit the source, supplies, load, topology, authentic model identity/content, pin order, analyses, or limits. Substituting a generic model is not success. The model is TI `OPAx197 PSpice Model (Rev. D)`, Final 1.3 dated 2022-06-23, downloaded from `https://www.ti.com/lit/zip/SBOMA34`; its required SHA-256 is `fc5b020e63346e511bd808bf41c856b0150b000bcf8a41fe00eeececb1f422a5`, and its pin order is `IN+ IN- VCC VEE OUT`. It is supplied only for this local evaluation under TI terms; retain its notices and do not copy, rewrite, embed in another artifact, or claim redistribution rights. If the supplied workflow cannot represent or execute the exact model, report a capability failure rather than fabricating support. Work only in the supplied directory; do not use the network. You may calculate and simulate iteratively. Preserve each materially different candidate revision in `revisions/` before replacing it. Write a final candidate only if the supplied workflow can truthfully represent the exact required model. Produce the schematic and editable/export artifacts that your supplied workflow genuinely supports; missing capability must be reported rather than fabricated. Finish with a short account of calculations, checks, model provenance, capability limits, and produced files. Do not inspect parent directories or search for Kessetsu repository examples, evaluator code, prior attempts, or another arm's work.

## U6 Kessetsu arm tool reference

- Executable: `./kess.exe`; final source, if representable without substitution: `candidate.kess`.
- `./models/OPAx197.LIB` is the exact local manufacturer model supplied to both arms. It is not a Kessetsu built-in or packaged model and must not be copied into another artifact.
- The current CLI accepts built-in models, the exact `kessetsu_analog@1.0.0` package, allowlisted typed diode/BJT/MOSFET declarations, and a safe parameterized op-amp template. It has no documented command for importing an arbitrary manufacturer SPICE/PSpice subcircuit body.
- A typed declaration such as `subcircuit opamp NAME (in_p,in_n,vcc,vee,out) ... gain=... bandwidth=...` generates Kessetsu's safe linear template; naming it `OPAx197` would not make it the authentic TI model and is forbidden for this task. `KESSETSU_OPAMP_V1` and `KESSETSU_PACKAGE_OPAMP` are also invalid substitutes.
- If no documented exact-model path exists, do not create `candidate.kess`; instead write `CAPABILITY.md` explaining the precise unsupported boundary, retain the supplied model notices, list no electrical PASS, and report which future capability would be required. Do not bypass the Kessetsu workflow by submitting a direct SPICE candidate in this arm.
- If an exact supported path is found using only the supplied documentation and CLI help, normal Kessetsu declarations use `source`, `resistor`, and `opamp`; pins are source `plus/minus`, resistor `p1/p2`, and op-amp `in_p/in_n/vcc/vee/out`; available analyses are OP, AC, and transient. Use `check`, `compile`, `simulate`, `render`, and `export` as documented in the U1 reference.

## U6 Direct arm tool reference

- Executable: `./tools/ngspice/bin/ngspice_con.exe`; final source: `candidate.cir`.
- `./models/OPAx197.LIB` is the exact official model. Preserve it byte-for-byte and retain its notices. The final `candidate.cir` contains only the candidate topology and must not embed, rewrite, or `.include` the model; the evaluator separately supplies the already hash-verified file.
- The first SPICE line is a title. Use `VIN in 0 SIN(0 0.1 1000) AC 1`, a +6 V source from `vcc` to ground, and a +6 V source from ground to `vee`. Resistors use `Rname node1 node2 value`; instantiate the model as `Xname in_p in_n vcc vee out OPAx197` in the exact five-pin order.
- The final topology must use one OPAx197, the fixed 10 kohm load, and only the two additional feedback-divider resistors. It must not contain a model or include directive.
- For local iteration only, create a separate `verification.cir` that includes `./models/OPAx197.LIB`, and run `./tools/ngspice/bin/ngspice_con.exe -D ngbehavior=ps -n -b verification.cir`. Do not replace the final candidate with this deck. A candidate may contain inert `.op`, `.ac`, `.tran`, `.four`, `.control`, and measurement/output commands; the evaluator ignores them and supplies its own exact-model OP/AC/transient testbench.
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
