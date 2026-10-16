# Importing SPICE netlists

Kessetsu 1.3.0 can convert the supported Ngspice-compatible netlist subset into editable
Kessetsu source. For a complete first example, save this as `filter.cir`:

```spice
.title RC low-pass
VIN IN 0 AC 1
R1 IN OUT 1k
C1 OUT 0 159.154943n
.ac dec 40 10 100k
.end
```

With the CLI and Ngspice installed, you can check, simulate, test against `.kessreq` requirements,
render a schematic or export to KiCad **directly from the SPICE file** (`--input auto` is the default):

```bash
kess check filter.cir
kess simulate filter.cir
kess test filter.cir --requirements filter.kessreq
kess render filter.cir --output filter.svg
kess export filter.cir --target kicad --output filter.kicad_sch
```

To save the converted, editable `.kess` file explicitly, use `kess import`:

```bash
kess import filter.cir --output filter.kess
```

If `--output` is omitted for a file input, `kess import` uses the same basename with `.kess`.
Existing files require `--force`. Standard input is supported with
`kess import - --output circuit.kess` (or `kess check - --input spice`); it never chooses an
implicit file destination.

In Web Hub, choose **File → Import SPICE netlist…** or drop a supported
netlist onto the editor. Conversion runs locally through the same Core contract. A complete
import opens as an unsaved editable circuit; a rejected import leaves the current document intact.
The rejection panel names the file, shows the first diagnostic and keeps the complete Core
diagnostic list under **All import diagnostics**. Line numbers refer to the original SPICE
file, not your open `.kess`. Correct the original file, then use **Choose another netlist…**;
the panel also links to this guide and the RC example above. It does not delete unsupported
lines, substitute models or detach the existing save destination. A changed circuit while the
file picker/read is pending requires a fresh import rather than replacing your new edits.
For the example above, wait for **Source valid**, then press **Run** in Simulation and select
`V(out)` for the low-pass response. Import preserves the AC analysis but does not invent
engineering requirements: pass `--requirements limits.kessreq` in the CLI/MCP workflow or add
`assert` statements in the generated `.kess` if you want PASS/FAIL checks. See the
[first-circuit tutorial](tutorial.md) and [agent design workflow](agent-workflow.md).

## What the supported subset accepts

| SPICE form | Kessetsu result |
|---|---|
| `R`, `C`, `L` with literal values | resistor, capacitor, inductor |
| Independent `V` and `I` | DC, AC, three-field SINE, seven-field PULSE or PWL source |
| `D`, `Q`, three-terminal `M` (or four-terminal with bulk tied to source) | built-in models (`1N4148`, `1N4007`, `2N3904`, `2N3906`, `2N2222`, `IRF540`, `KESSETSU_*`) or inline `.model` |
| 5-terminal `X` op-amp (`in_p in_n vcc vee out` with `KESSETSU_OPAMP_V1` or `OPAMP`) | native `opamp` component using built-in `KESSETSU_OPAMP_V1` (with op-amp ERC and schematic symbol) |
| Inline `.model NAME D|NPN|PNP|NMOS|PMOS(...)` | typed `model` declaration with validated Ngspice parameters (descriptive vendor fields such as `MFG` or `VCEO` are ignored with a warning) |
| `.op`, bounded `.tran`, `.ac`, single-source `.dc` | typed `simulate` statements |
| `.param NAME=literal` | typed editable parameter, with unit inferred from its supported uses |
| Source-embedded `.SUBCKT` plus root `X` instances | editable `module`/`use`, preserving ports, local topology and literal typed overrides |
| First-line plain title, `.control ... .endc`, `.print`, `.plot`, `.probe`, `.save`, `.meas`, `.options` | tolerated and skipped with informational/warning diagnostics (`KES-N010`..`KES-N012`) because Kessetsu owns simulation runner control and measurement evaluation |

Continuation lines beginning with `+`, comments, `.title` and `.end` are understood. SPICE node
`0` becomes the explicit `GND` net. Other node, component and inline-model names are preserved when
valid or mapped deterministically when Kessetsu identifiers or built-in model names require it.
`--format json` returns the `kessetsu.spice-import.v1` report, source hash, name mappings,
diagnostics, summary and verified editable source.

SPICE suffix meaning is preserved rather than copied blindly. In SPICE, `10M` means 10 milliohms,
while Kessetsu uses SI casing where `10M` would mean 10 megaohms. The importer therefore writes a
normalized number and canonical recompilation confirms the resolved value.

## Fail-closed limits

The importer rejects electrical constructs that cannot be represented truthfully in Circuit IR instead of guessing or silently removing components:

- arbitrary `.include`/`.lib` paths;
- unknown `.model` families or unsupported model parameters;
- parameter expressions, unused parameters or one parameter used across conflicting electrical units;
- controlled or behavioral sources (`E`, `F`, `G`, `H`, `B`) and unsupported component families;
- unknown semiconductor models without an inline `.model`, untied MOSFET bulk nodes, non-zero AC source phase, or analysis
  options that the typed Kessetsu IR cannot represent.

Embedded subcircuits are intentionally structural, not raw simulator text. A `.SUBCKT` must have
explicit interface nodes and may contain the same supported component subset as the root circuit.
Literal defaults declared after `PARAMS:` and named literal/root-parameter overrides on a root `X`
instance remain typed module parameters. Nested `X` instances, body directives, implicit global
node `0`, positional parameters and expression-valued defaults/overrides are rejected rather than
flattened or guessed. Expose ground as an ordinary subcircuit port and pass root node `0` to it.

An external `.include`/`.lib` is different from an embedded topology: its bytes, entry pins,
provenance, license and redistribution policy cannot be inferred safely from a path. Import
therefore rejects it. Bind such a dependency through Kessetsu's hash-pinned typed external model
contract; the importer never opens an arbitrary path or silently substitutes a generic device.

An error includes the original source line and no `.kess` file is written. This is intentionally
narrower than the complete SPICE language. Support expands only when connectivity, values, models
and analysis meaning can be represented and verified without a raw-SPICE bypass.

## PySpice

Generate a netlist in the user's own Python environment, save it, then import that file with the
same command. Kessetsu does not execute Python and does not claim to preserve Python loops,
functions or intent that are absent from the generated netlist.

For example, this PySpice 1.5 circuit emits only constructs in the supported import subset:

```python
from PySpice.Spice.Netlist import Circuit
from PySpice.Unit import u_V, u_kOhm

circuit = Circuit("Voltage divider")
circuit.V("IN", "IN", circuit.gnd, 10 @ u_V)
circuit.R(1, "IN", "OUT", 1 @ u_kOhm)
circuit.R(2, "OUT", circuit.gnd, 1 @ u_kOhm)
circuit.raw_spice += ".op\n"

with open("divider.cir", "w", encoding="utf-8") as output:
    output.write(str(circuit))
```

Then run `kess import divider.cir --output divider.kess`. Compatibility is determined by the
generated SPICE text, not by the Python library version: unsupported devices or directives still
fail closed with a source-line diagnostic.

The successful `.kess` file is the normal save, share, edit, simulation and export artifact. Keep
the original netlist beside it when provenance matters; its exact SHA-256 identity appears in the
generated source header and JSON report. Imported simulation remains model-based evidence, not a
guarantee of physical hardware behavior.
