# Kessetsu

[Web Hub](https://stapimaz.github.io/Kessetsu/) · [CLI releases](https://github.com/Stapimaz/Kessetsu/releases) · [Documentation](docs/README.md) · [Tutorial](docs/tutorial.md) · [Supported domain](docs/supported_domain.md)

Kessetsu is an agent-driven circuit engineering platform for describing circuits as text, compiling and simulating them like software, and verifying them with engineering assertions. The CLI's human mode serves engineers, its versioned JSON mode serves AI agents and automation, and the zero-friction Web Hub serves browser users. Every surface uses the same Rust Core: typed Circuit IR, deterministic graph/ERC, SPICE netlists, schematics, and EDA outputs all derive from shared semantics.

```text
Electrical requirements → Kessetsu source → compile/ERC → simulate/measure/assert → structured feedback
```

The project is not limited to a particular educational scenario or circuit class. Its goal is to let people and AI agents start from measurable requirements and iteratively develop topology, value, and model choices using reliable structured feedback.

## Current capabilities

- Rust parser, module flattening, and typed Circuit IR
- Versioned `kessetsu.compile.v4` compile reports and the `kessetsu.schematic.v2` schematic contract
- Deterministic net naming and `KES-P/C/E/I/S/F` diagnostic namespaces
- SPICE generation, simulator discovery/provenance, and a Windows Ngspice sidecar runtime
- Typed OP/transient/AC/DC simulation results and PASS/FAIL/ERROR/SKIPPED assertion evaluation
- Compact `kessetsu.cli.v1` JSON, stdin-based agent loops, and opt-in debug fields through `--include`
- Typed user/package model and subcircuit resolution, provenance manifests, and `kessetsu.lock`
- Human/JSON CLI modes with safe output, overwrite, and exit-code contracts
- A React Web Hub with WASM Core and real in-browser simulation
- A canonical, connectivity-verified automatic schematic engine
- SVG, PNG, PDF, Schematic JSON, SPICE, KiCad, and LTspice exports
- Versioned, compressed, package-aware share URLs

The [roadmap](docs/ROADMAP.md) defines current work and gates. The [architecture](docs/architecture.md), [supported domain](docs/supported_domain.md), and [engineering measurement contract](docs/engineering_measurements.md) are the normative references for implementation boundaries and formulas.

## Web Hub

Kessetsu Web Hub is the primary product surface for people who want to develop circuits directly in a simple, CodePen-like browser workspace. It does not maintain a separate Web-only engine; it runs the same canonical Rust Core as the CLI through WebAssembly.

The current repository build can:

- Edit Kessetsu source in Monaco and switch between example circuits
- Run live compilation, ERC, and canonical schematic-connectivity verification through WASM
- Run real OP/transient/AC/DC simulations inside a Web Worker
- Inspect interactive waveform, Bode, and DC plots plus assertion results
- Explore the canonical schematic with zoom and fit controls
- Download seven visual, machine, and EDA formats with capability/loss information
- Share source and exact package versions in a compressed URL

The first release deliberately has no AI chat inside Web Hub. The CLI's stdin plus versioned JSON tool contract is the AI/automation surface, while Web Hub is the direct human surface; both use the same Core. A provider-independent Web AI design surface is reserved for a later phase.

![Kessetsu Web Hub RC workspace](docs/assets/web-hub-workspace.png)

![Power amplifier simulation with 12 passing requirements](docs/assets/web-hub-power-amplifier.png)

## A small Kessetsu example

```kessetsu
net GND
net out

source V1 5V
resistor R1 1k

connect V1.plus, R1.p1 to out
connect V1.minus, R1.p2 to GND

simulate op
```

Compiling the same source in the CLI or Web Hub produces the same IR, diagnostics, and SPICE result.

## Quick start

Ready-to-use CLI packages are published on [GitHub Releases](https://github.com/Stapimaz/Kessetsu/releases) for Windows x86-64, Linux x86-64, macOS Intel, and macOS Apple Silicon, together with SHA-256 files. The Windows package includes a verified Ngspice sidecar. On Linux and macOS, install the system `ngspice` package or provide a trusted absolute path through `KESSETSU_NGSPICE`. Each package includes `INSTALL.txt` and `release-manifest.json` with exact setup and provenance details.

Requirements:

- Rust `1.97.1` and the `wasm32-unknown-unknown` target
- `wasm-pack 0.13.1`
- npm with the Node version declared in [`.nvmrc`](.nvmrc)
- The repository sidecar for native Windows simulation, or `KESSETSU_NGSPICE` for an alternative simulator path

Build the CLI and check an example circuit:

```powershell
cd core
cargo build --release
cargo run --release -- check ../examples/demo_circuit.kess
cargo run --release -- compile ../examples/demo_circuit.kess --output ../examples/demo_circuit.spice
```

Machine-readable output:

```powershell
cargo run --release -- check ../examples/demo_circuit.kess --format json
```

Existing output files are not overwritten by default; intentional replacement requires `--force`. The [CLI reference](docs/cli_reference.md) defines commands, JSON fields, and exit codes. Continue with the [tutorial](docs/tutorial.md), [cookbook](docs/cookbook.md), [troubleshooting guide](docs/troubleshooting.md), or [Why Kessetsu?](docs/why_kessetsu.md).

## Build and verification

Run the repository's canonical quality gate from the root:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/verify.ps1
```

After dependency installation, this runs Rust formatting, Clippy, tests, the release build, WASM packaging, the production npm audit, Web linting, and the production build. If dependencies are already installed:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/verify.ps1 -SkipNpmInstall
```

Individual build surfaces:

```powershell
cd core
cargo test --all-targets
cargo build --release
wasm-pack build . --target web --out-dir pkg --release

cd ../webapp
npm.cmd ci
npm.cmd run build
```

`npm run build` regenerates the WASM package and completes the Web production build.

## Repository structure

- `core/`: Rust library, CLI, WASM adapter, test corpus, and Windows Ngspice runtime
- `examples/`: canonical `.kess` examples
- `webapp/`: React/TypeScript zero-friction Web Hub
- `docs/`: documentation index, roadmap, architecture, language/simulation/export contracts, and guides
- `scripts/verify.ps1`: root quality gate

## Important boundaries

- Backends consume typed Circuit IR only.
- Generated output does not replace physical validation or engineering review.
- Embedded-runtime provenance and licensing notes live in the [Ngspice runtime README](core/tools/ngspice/README.md).
- The first release targets schematic-level analog/mixed-signal work; it does not provide PCB layout/DRC, RF/EM, thermal/reliability, Monte Carlo, or laboratory validation.
- [SECURITY.md](SECURITY.md) defines vulnerability reporting; the [release contract](docs/release.md) defines release, rollback, and telemetry boundaries.

## License

Kessetsu source code is available under the [GNU Affero General Public License v3.0 only](LICENSE) (`AGPL-3.0-only`). A separate [commercial license](COMMERCIAL_LICENSE.md) may be obtained from the copyright holder for closed-source service or product use without AGPL obligations.

Circuit sources and exported artifacts created with Kessetsu do not become subject to the AGPL merely because Kessetsu produced them. Third-party components retain their own licenses; distribution notices are included in release packages and the Web build. See [CONTRIBUTING.md](CONTRIBUTING.md) for the contribution policy.
