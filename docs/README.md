# Kessetsu Documentation

Describe a circuit, simulate it, check measurable requirements and export its schematic.
Use the free [Web Hub](https://kessetsu.com/#editor) without an account or install the
[local CLI](https://kessetsu.com/install/) for engineering and AI-agent workflows.

## Start here

- [Tutorial](guides/tutorial.md): build your first circuit and verify its frequency response.
- [Web editor](guides/web-editor.md): examples, files, panels, simulation, exports and sharing.
- [Why Kessetsu?](guides/why-kessetsu.md): how the combined workflow complements SPICE and EDA tools.
- [Cookbook](guides/cookbook.md): calculations, reusable blocks, agent loops and device models.
- [Parameter studies](guides/parameter-studies.md): multi-condition experiments and reports (development source).
- [Research data](guides/research-data.md): local CSV mapping, evidence and scalar comparisons (development source).
- [Finite parameter fitting](guides/model-fitting.md): calibration-only selection with separate holdout validation (development source).
- [Memristor pulse protocol](guides/memristor-protocol.md): a reproducible threshold-model study with convergence and held-out stimulus checks (development source).
- [Python and Jupyter](guides/python-notebooks.md): inspect simulations, studies and research evidence in Python (development source).

## Language and command reference

- [Language](reference/language.md): components, pins, units, parameters and modules.
- [CLI](reference/cli.md): commands, overrides, JSON output, file safety and exit codes.
- [Simulation and assertions](reference/simulation-and-assertions.md): analyses and PASS/FAIL results.
- [Engineering measurements](reference/measurements.md): metric definitions, arguments and units.

## Models and engineering output

- [Device models](reference/model-catalog.md): local files, compatibility, provenance and examples.
- [Components and simulation limits](reference/supported-domain.md): what is modeled and what is not.
- [Export formats](reference/exports.md): SVG, PNG, PDF, JSON, SPICE, KiCad and LTspice.
- [Troubleshooting](guides/troubleshooting.md): diagnostics, browser storage and installation help.

## Updates and support

- [Changelog](../CHANGELOG.md): versioned features, fixes and migration notes.
- [Report an issue](https://github.com/Stapimaz/Kessetsu/issues): include the version,
  a minimal reproducible circuit and the diagnostic, without private model files.
- [Security reporting](../SECURITY.md) and [license](../LICENSE).

Simulation results describe the supplied circuit, models and conditions; they are not
a guarantee of physical hardware performance. Each reference explains relevant assumptions.

For source contributions, read [CONTRIBUTING.md](../CONTRIBUTING.md) and the
[architecture rules](architecture.md). Product planning and operational records are maintained
separately; this documentation focuses on using and extending the released product.
