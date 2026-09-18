# Why Kessetsu?

Kessetsu connects jobs that are often separate: describe a circuit, validate connections,
simulate, check engineering requirements, draw a schematic and export editable artifacts.
It uses Ngspice for simulation, not a replacement physics engine.

## What it adds to a SPICE workflow

- A compact circuit language with units, named parameters and reusable modules.
- Structural ERC and diagnostics before simulation.
- Executable assertions with actual measurements and explicit failure reasons.
- Automatic schematics and seven visual, numeric and editable engineering exports.
- The same circuit semantics in a no-install browser workspace and a local CLI.

You can use Ngspice directly when its broader netlist language is what you need. Kessetsu
is useful when you want the complete edit → verify → draw → handoff loop in one workflow.
It does not promise every SPICE construct, better electrical results or faster execution.

## Use your own AI agent

An agent writes `.kess`, invokes `kess check` or `kess test --format json`, reads the
diagnostics and measured failures, then revises the design. No Kessetsu AI subscription
is needed. Keep acceptance limits in a separate `.kessreq` when the agent must not change them.
See the [agent cookbook](cookbook.md#give-an-external-agent-a-complete-task).

## Keep working in your preferred EDA tool

KiCad and LTspice remain full graphical design environments. Kessetsu produces editable
schematics so you can continue there; source-level assertions and model dependencies do
not automatically become native EDA features. Review the [export reference](../reference/exports.md).

## Know what a result means

A passing assertion establishes a result for the supplied model and simulation conditions,
not a physical hardware guarantee. Generic models can omit important device effects.
The [component reference](../reference/supported-domain.md) and
[model catalog](../reference/model-catalog.md) explain those boundaries.

Start with the [tutorial](tutorial.md), [Web editor](web-editor.md) or
[CLI installation](https://kessetsu.com/install/).
