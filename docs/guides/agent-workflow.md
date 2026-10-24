# Agent design workflow

This guide is for AI agents (and the people supervising them) that use Kessetsu to design,
verify and hand off analog circuits. It is served verbatim by the `kessetsu_guide` MCP tool,
so an agent connected through `kess mcp` receives the same rules a human reads here.

The promise: when a person says **"use Kessetsu"**, the agent should produce a design that
is traceable, testable and reviewable without the person writing a long prompt about
Ngspice, output parsing or file organization.

## Connecting your AI agent (`kess mcp`)

After installing the CLI (`kess --version`), register Kessetsu's built-in stdio MCP server:

- **Claude Code:**
  ```bash
  claude mcp add kessetsu -- kess mcp
  ```
- **Cursor, Windsurf, VS Code or any MCP client (`mcpServers` JSON):**
  ```json
  {
    "mcpServers": {
      "kessetsu": {
        "command": "kess",
        "args": ["mcp"]
      }
    }
  }
  ```

Agents with terminal access can also call `kess` directly (`kess check`, `kess test`, `kess render`, `kess export` with `--format json`).

## The disciplined loop

1. **Requirements first.** Turn the request into measurable limits *before* choosing a
   topology. Write them to `requirements.kessreq` (assertion lines only). Never weaken a
   limit to make a design pass; if a limit looks impossible, report it to the human.
2. **Design in readable source.** Write `design.kess` with named nets for every signal that
   matters (`IN`, `OUT`, `VCC`...), a comment stating the target, and modules for repeated
   blocks. SPICE netlists are accepted too (see below), but `.kess` is the reviewable format.
3. **Check structure.** Call `kessetsu_check`. Fix every ERC error (floating pins, shorted
   sources, ambiguous ground) before simulating; these are the root cause of most cryptic
   simulator failures.
4. **Test against the requirements.** Call `kessetsu_test` with the design and the
   unchanged requirements text. Iterate on the design only. Each assertion returns
   PASS/FAIL/ERROR/SKIPPED with its measured value.
5. **Show the human.** Call `kessetsu_render_schematic` and save `schematic.svg` next to the
   design. A person can review a schematic; nobody reviews a netlist.
6. **Hand off.** Export what the next step needs (`kicad`, `ltspice`, `bom-csv`, `spice`).
7. **Report honestly.** Summarize: requirements, final PASS table, assumptions (ideal
   sources, generic models, no tolerances unless studied), and open risks.

Recommended project layout:

```text
my-circuit/
  requirements.kessreq   # acceptance criteria, owned by the human/evaluator
  design.kess            # the circuit, edited by the agent
  schematic.svg          # rendered for review
  design.kicad_sch       # optional handoff
  NOTES.md               # decisions, assumptions, final PASS table
```

## Kessetsu source in one screen

```kessetsu
// Target: 1 kHz RC low-pass, unity passband gain.
net GND
net IN
net OUT

source VIN ac(1V)            // also: 5V, sine(0V,1V,1kHz), sine_ac(0V,1V,1kHz,1V), pulse(...), pwl(...)
resistor R1 1k
capacitor C1 159nF

connect VIN.plus, R1.p1 to IN
connect R1.p2, C1.p1 to OUT
connect VIN.minus, C1.p2 to GND

simulate ac dec 40 10Hz 100kHz   // also: op | tran 10us 5ms | dc VIN 0V 5V 100mV
```

- Components and pins: `resistor|capacitor|inductor|diode` (`p1`,`p2`),
  `transistor Q1 npn 2N3904` (`c`,`b`,`e`), `mosfet M1 IRF540` (`d`,`g`,`s`),
  `opamp U1` (`in_p`,`in_n`,`vcc`,`vee`,`out`), `source` / `current_source` (`plus`,`minus`).
- `net GND` is always SPICE node `0`. Name every net you will measure.
- Parameters: `param fc: Hz = 1kHz`, then `capacitor C1 {1 / (2 * pi * 1k * fc)}`.
- Reusable blocks: `module LowPass(input,output,gnd) { ... }` and `use LowPass F1(fc=2kHz)`.
- Custom device models: `model diode D1 version=1.0.0 license=MIT Is=2n Rs=0.5 N=1.75`.

The [language reference](../reference/language.md) is authoritative.

## Requirements (`.kessreq`)

One assertion per line, literal limits only:

```text
// Owned by the evaluator; the design agent must not edit this file.
assert upper_cutoff(V(OUT),V(IN)) > 950Hz
assert upper_cutoff(V(OUT),V(IN)) < 1050Hz
assert gain_at(V(OUT),V(IN),100Hz) > 0.99
```

Common metrics: `value|min|max|peak|average|rms(V(net))`, `gain(out,in)`,
`gain_at(out,in,f)`, `upper_cutoff/lower_cutoff(out,in)`, `phase(out,in,f)`,
`frequency(V(net))`, `output_power(V(out),RL)`, `efficiency(...)`, `thd(...)`,
`rise_time/fall_time/settling_time/overshoot(...)`, `dissipation(device)`.
Signals: `V(net)`, `V(device)`, `I(device)`, `P(device)`. The
[measurement reference](../reference/measurements.md) defines exact semantics and the
analysis each metric needs (for example cutoff metrics need `simulate ac`).

## Using SPICE you already have

Every circuit command accepts a supported Ngspice-style netlist directly. Kessetsu converts
it through the canonical importer, then applies the same ERC, simulation, assertions,
schematic and exports. Five-terminal op-amps (`XU1 in_p in_n vcc vee out KESSETSU_OPAMP_V1`
or `OPAMP`) become native `opamp` components, numeric SPICE nodes can be referenced directly
in `.kessreq` (`V(1)`, `V(2)`), and inline `.model` cards for D/NPN/PNP/NMOS/PMOS are
imported as typed models; `.control` blocks and output directives (`.print`, `.plot`, `.meas`,
`.options`...) are skipped with warnings because Kessetsu owns measurement. Unsupported
constructs fail with line numbers instead of being dropped. See the
[SPICE import guide](spice-import.md).

## Reading results

- Top-level `status`: `"success"` means compilation, simulation and all assertions passed (exit code `0`). When any requirement fails, `kess test` / `kessetsu_test` reports `status: "test_failed"` (exit code `4`); compile/ERC errors report `"compile_failed"` (exit code `2`), and simulator failures report `"simulation_failed"` (exit code `3`).
- Assertion outcomes live in `assertions.assertions[*]`, where each entry carries `code`, `expression`, `status` (`"PASS" | "FAIL" | "ERROR" | "SKIPPED"`), `actual`, `expected` and `unit` (aggregate counts are in `summary.assertions`).
- Diagnostics carry stable codes: `KES-E*` ERC, `KES-C*` semantic, `KES-S*` simulator,
  `KES-T*` assertions, `KES-N*` SPICE input. Fix the first error first.
- `KES-S004` (convergence/timestep) usually means a floating node, an ideal source loop,
  a missing DC path to ground, or an unrealistic step. Re-run `kessetsu_check` and add
  realistic source/load resistance before touching simulator options.

## Honest limits

Simulation evidence is not hardware validation. Built-in models are generic. Passing
assertions prove only the stated conditions under the selected models and analyses. Say so in
the final report.
