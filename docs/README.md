# Kessetsu Documentation

Simulate analog and mixed-mode circuits with real Ngspice, verify locked electrical requirements (`.kessreq`), and generate connectivity-verified schematics from standard **SPICE netlists** (`.cir`, `.sp`) or readable **`.kess`** source.

You can use Kessetsu in three ways, all powered by the same deterministic Rust Core:

- **Interactive Web Hub:** Open the [browser workspace](https://kessetsu.com/#editor) with zero installation or account.
- **AI Agents via MCP (`kess mcp`):** [Install `kess`](https://kessetsu.com/install/) and register `kess mcp` in Claude Code, Claude Desktop, Cursor, or Windsurf so your AI agent can compile, simulate, verify requirements, and render schematics through native tools.
- **Local CLI (`kess test`):** Run ERC, simulations, parameter studies, and exports directly from your terminal or CI pipeline.

## Getting started

- [Tutorial](guides/tutorial.md): build your first circuit, run AC/transient simulation, and verify its frequency response.
- [Web Hub guide](guides/web-editor.md): workspace views (`Schematic | Simulation | Split`), interactive plots, SPICE converter, exports, and sharing.
- [SPICE netlist import and direct use](guides/spice-import.md): check, test, render, export, or convert supported Ngspice netlists without rewriting them from scratch.
- [Why Kessetsu?](guides/why-kessetsu.md): how executable requirements and automatic schematics complement SPICE and desktop EDA tools.

## AI agents and workflows

- [Agent design workflow (MCP & CLI)](guides/agent-workflow.md): connect an AI agent via `kess mcp` or JSON CLI using locked `.kessreq` requirement contracts, ERC, simulation, and schematic handoff.
- [Cookbook](guides/cookbook.md): practical design recipes, reusable module blocks, closed-loop agent tasks, and device models.
- [Parameter studies](guides/parameter-studies.md): multi-condition sweeps across component tolerances, loads, and temperatures.

## Research and data validation

- [Research data](guides/research-data.md): local CSV mapping, residual evidence, and scalar comparisons against simulated traces.
- [Finite parameter fitting](guides/model-fitting.md): calibration-only candidate selection with separate holdout validation.
- [Portable research packages](guides/portable-research-packages.md): bundle exact study evidence, permitted dependencies, and reproducible rerun instructions.
- [Python and Jupyter](guides/python-notebooks.md): inspect simulations, parameter studies, and research evidence in Python.
- [Memristor pulse protocol](guides/memristor-protocol.md): a reproducible threshold-model study with convergence and held-out stimulus checks.

## Reference

- [Language](reference/language.md): `.kess` components, pins, units, parameters, and reusable modules.
- [CLI & MCP](reference/cli.md): commands, `kess mcp` tool server, overrides, JSON schemas, file safety, and exit codes.
- [Simulation and assertions](reference/simulation-and-assertions.md): OP, DC, AC, and transient analyses with `PASS/FAIL` requirement syntax.
- [Engineering measurements](reference/measurements.md): metric definitions, arguments, and units.
- [Device models](reference/model-catalog.md): built-in generic models, local manufacturer `.lib`/`.mod` bindings, and provenance.
- [Export formats](reference/exports.md): SVG, PNG, PDF, Schematic JSON, SPICE, KiCad, LTspice, BOM CSV, and Handoff JSON.
- [Components and simulation limits](reference/supported-domain.md): what is modeled and what is out of scope.

## Help and updates

- [Troubleshooting](guides/troubleshooting.md): diagnostics, browser storage, Ngspice setup, and installation help.
- [Changelog](../CHANGELOG.md): versioned features, fixes, and migration notes.
- [Report an issue](https://github.com/Stapimaz/Kessetsu/issues): include the version, a minimal reproducible circuit, and the diagnostic, without private model files.
- [Security reporting](../SECURITY.md) and [license](../LICENSE).

Simulation results describe the supplied circuit, models, and conditions; they are not a guarantee of physical hardware performance. Each reference explains relevant assumptions.

For source contributions, read [CONTRIBUTING.md](../CONTRIBUTING.md) and the [architecture rules](architecture.md). Product planning and operational records are maintained separately; this documentation focuses on using and extending the released product.

