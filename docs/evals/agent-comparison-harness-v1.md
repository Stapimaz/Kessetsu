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
