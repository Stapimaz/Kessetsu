# Why Kessetsu?

Kessetsu is not a claim that existing simulators or EDA tools are inadequate. It connects several jobs that are usually separate: a compact circuit language, deterministic validation, real SPICE simulation, engineering assertions, structured agent feedback, automatic schematic rendering, editable exports and a no-account browser workspace.

## Measurable differences

| Workflow | What the established tool optimizes for | What Kessetsu adds |
|---|---|---|
| Raw Ngspice netlist | Broad open-source SPICE simulation from file/CLI; Ngspice itself does not provide schematic entry | Typed/allowlisted source, source-located diagnostics, ERC, versioned datasets/measurements/assertions, automatic schematic and seven exports |
| KiCad or LTspice | Mature graphical schematic capture plus integrated simulation/waveform workflows | One text source callable by an AI agent, deterministic JSON feedback and the same Core in a zero-install browser; editable files are outputs, not replacements for those editors |
| Python circuit DSL such as SKiDL | General-purpose Python composition, ERC and multiple netlist/PCB/graphics outputs | A small data-like DSL with no arbitrary code execution, direct measurement assertions and native/browser contract parity |

The repository tests these mechanisms through three real Ngspice benchmarks, native/browser engineering-decision parity, schematic connectivity/visual gates, KiCad 10 and LTspice round-trip smoke tests, and a replayable structured-feedback correction. The correction restores a deliberately changed load in an existing amplifier; it does not prove independent design from unseen requirements or superiority over direct simulator/EDA workflows.

## Competitive reality check — 2026-09-10

[Flux](https://www.flux.ai/p/blog/simulate-circuits-with-a-prompt) documents AI-driven SPICE simulation and iteration against specifications. [tscircuit](https://docs.tscircuit.com/) documents a code-based workflow for schematics, simulation, PCBs, and manufacturing outputs. [Quilter](https://docs.quilter.ai/using-quilter/introduction) documents automated placement, routing, and validation from a schematic and starter board. AI-assisted circuit engineering is therefore not unique to Kessetsu.

Kessetsu's proposed value is the combined provider-independent CLI, local/browser execution, executable requirements, and readable portable artifacts. Whether this combination saves effort or improves results over a strong agent using established tools is an open question tracked in [Roadmap 4.6V](ROADMAP.md#46v--unseen-design-and-incremental-value-gate). No comparative superiority or market demand has been demonstrated. Its current generic-model and subcircuit limits remain material; see the [support matrix](supported_domain.md).

## Who it is for

Kessetsu can serve learners, working engineers, hobbyists, automation and AI agents within its [declared analog/mixed-signal boundary](supported_domain.md). It is not limited to coursework, and it does not claim to replace PCB layout, RF/EM, thermal/reliability analysis, laboratory validation or engineering review.

## Product split

- The CLI is the AI/automation surface: stdin, versioned JSON, stable exit codes and deterministic artifacts.
- Web Hub is the human surface: edit, compile, simulate, inspect, export and share without an account or local installation.
- Both are thin adapters over the same Rust Core. Web Hub deliberately has no provider-specific AI chat in the first release; a future chat layer can call the same contracts without becoming the source of circuit truth.

## Sources checked for the first release

Claims were reviewed on 2026-08-13 against primary product documentation:

- Ngspice describes itself as an open-source circuit simulator driven by netlists and explicitly notes that it has no schematic entry: <https://ngspice.sourceforge.io/index.html>
- KiCad documents schematic ERC, Ngspice-backed simulation and graphical analysis workflows: <https://docs.kicad.org/master/en/eeschema/eeschema.html>
- Analog Devices describes LTspice as a SPICE simulator, schematic-capture tool and waveform viewer: <https://www.analog.com/en/resources/design-tools-and-calculators/ltspice-simulator.html>
- SKiDL documents Python circuit description, ERC and netlist/PCB/SVG generation: <https://devbisme.github.io/skidl/api/html/rst_output/skidl.html>

This comparison is scoped to documented first-release behavior, not a permanent claim about competitors.
