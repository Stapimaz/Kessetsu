# Importing SPICE netlists

Development builds can convert a deliberately bounded Ngspice-compatible netlist into editable
Kessetsu source:

```bash
kess import filter.cir --output filter.kess
kess check filter.kess
kess simulate filter.kess
```

If `--output` is omitted for a file input, the CLI uses the same basename with `.kess`.
Existing files require `--force`. Standard input is supported with
`kess import - --output circuit.kess`; it never chooses an implicit file destination.

## What the first subset accepts

| SPICE form | Kessetsu result |
|---|---|
| `R`, `C`, `L` with literal values | resistor, capacitor, inductor |
| Independent `V` and `I` | DC, AC, three-field SINE, seven-field PULSE or PWL source |
| `D` | selected built-in `1N4148`/`1N4007` models |
| `Q` | selected typed built-in NPN/PNP models |
| Three-terminal `M`, or four-terminal with bulk tied to source | selected built-in MOSFET models |
| `.op`, bounded `.tran`, `.ac`, single-source `.dc` | typed `simulate` statements |

Continuation lines beginning with `+`, comments, `.title` and `.end` are understood. SPICE node
`0` becomes the explicit `GND` net. Other node and component names are preserved when valid or
mapped deterministically when Kessetsu identifiers require it. `--format json` returns the
`kessetsu.spice-import.v1` report, source hash, name mappings, diagnostics, summary and verified
editable source.

SPICE suffix meaning is preserved rather than copied blindly. In SPICE, `10M` means 10 milliohms,
while Kessetsu uses SI casing where `10M` would mean 10 megaohms. The importer therefore writes a
normalized number and canonical recompilation confirms the resolved value.

## Fail-closed limits

The initial importer rejects these constructs instead of guessing or silently removing them:

- `.control`/`.endc`, shell-like or simulator-control content;
- arbitrary `.include`/`.lib` paths;
- inline `.model`, `.subckt`, `.param` and expressions;
- controlled or behavioral sources and unsupported component families;
- unknown semiconductor models, untied MOSFET bulk nodes, non-zero AC source phase, or analysis
  options that the typed Kessetsu IR cannot represent.

An error includes the original source line and no `.kess` file is written. This is intentionally
narrower than the complete SPICE language. Support expands only when connectivity, values, models
and analysis meaning can be represented and verified without a raw-SPICE bypass.

## PySpice

Generate a netlist in the user's own Python environment, save it, then import that file with the
same command. Kessetsu does not execute Python and does not claim to preserve Python loops,
functions or intent that are absent from the generated netlist.

The successful `.kess` file is the normal save, share, edit, simulation and export artifact. Keep
the original netlist beside it when provenance matters; its exact SHA-256 identity appears in the
generated source header and JSON report. Imported simulation remains model-based evidence, not a
guarantee of physical hardware behavior.
