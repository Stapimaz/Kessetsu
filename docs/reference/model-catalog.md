# Device and External Model Catalog

Local external-model workflows require Kessetsu 1.2.0; typed instance parameters require
Kessetsu 1.3.0. In Kessetsu 1.5.0, the built-in device catalog covers generic diodes,
Zener breakdown diodes, BJTs, JFETs, MOSFETs, switches, and multi-stage op-amp macromodels.
This reference documents both built-in models and characterized external models.
Model identity, simulation evidence, and physical-device accuracy remain distinct.

## Supported interfaces

| Declaration family | Instance keyword | Canonical pins, in positional SPICE order | Presentation |
|---|---|---|---|
| `opamp` | `opamp` | `in_p,in_n,vcc,vee,out` | Existing op-amp triangle |
| `comparator` | `device` | `in_p,in_n,vcc,vee,out` | Comparator triangle; no replacement op-amp model |
| `two_terminal` | `device` | `p1,p2` | Generic two-terminal box, not an R instance |

The family describes the electrical interface, not a package, footprint or device rating.
Only these catalog-backed interfaces are supported. Arbitrary pin/geometry descriptors are
not implemented. A declaration may expose up to 32 existing `.SUBCKT` header parameters
through a unit-typed allowlist; all other coefficients remain fixed by the exact file/version/hash.

## Local-file workflow

Start with [external_comparator.kess](../../examples/external_comparator.kess) or
[external_memristor.kess](../../examples/external_memristor.kess).

The memristor example exposes only `Rinit`, `Vt` and `stime` from its exact library header.
Its instance binds the first two through ordinary root parameters, so `--param`, parameter
studies and finite calibration workflows all use the same typed input path. This changes
an instance value, not the hash-bound model file.

On the CLI, keep the `models/` directory beside the source:

```console
kess test examples/external_comparator.kess
kess test examples/external_memristor.kess
kess export examples/external_memristor.kess --target ltspice --output examples/memristor.asc
```

On Web, open File → Examples → Local model files, then View → Circuit details. Download
the example's model file and select it for its declared resource. No account or upload
is involved. Compilation checks its exact hash, entry and positional terminal count;
simulation checks the browser profile before loading Ngspice. A selected file's original
filename need not match the reference: the selection explicitly binds it to that reference.

Files remain in memory only. Editing source retains bindings but revalidates its metadata;
opening a different circuit/new/example clears them. Reloading, browser draft recovery
and shared-source links require selecting them again. Changing/clearing a binding discards
prior simulation and drawing/export state. Renaming a circuit does not alter model identity.

Native files must be source-relative and contained under the source directory; traversal,
absolute paths and unsafe references are rejected. Web bindings use the same references
without reading the filesystem implicitly. Limits: 16 MiB per UTF-8 file, 64 bound browser
resources, 32 MiB combined browser bytes, 32 KiB per logical library statement and bounded
statement/nesting counts. These are resource ceilings, not a hard aggregate memory guarantee.

## Built-in device models and macromodels

### Kessetsu generic op-amp v2 (`KESSETSU_OPAMP_V2`)

- **Interface:** Five terminals (`in_p`, `in_n`, `vcc`, `vee`, `out`).
- **Version:** `1.0.0`.
- **Modeled physical behavior:**
  - High-impedance differential input stage with open-loop gain `gain = 200k` (106 dB).
  - Dominant-pole gain-bandwidth product compensation (`gbw = 1MHz`).
  - Slew-rate limiting stage (`slew_rate = 500k` V/s or `0.5 V/µs`).
  - Supply-rail saturation headroom (`vdrop = 1V` / `rail_headroom` relative to `vcc` and `vee`).
  - Linear output resistance (`rout = 50 Ω`).
- **Boundary note:** `KESSETSU_OPAMP_V2` accurately models rail headroom, dominant pole, slew rate, and linear output resistance (`rout = 50 Ω`). It does **not** model a hard nonlinear output-current limit, quiescent supply bias currents, or thermal/SOA shutdown.

### Built-in generic JFET models

Kessetsu provides built-in generic JFET models for N-channel and P-channel junction field-effect transistors:
- **`2N5457`** (alias **`J2N5457`**): Generic N-channel JFET (`NJF`, version `1.0.0`). Characterized with pinch-off voltage `vto = -1.5V`, transconductance parameter `beta = 1.125mA/V²`, channel-length modulation `lambda = 2.25m 1/V`, ohmic resistances `rd = 10 Ω`, `rs = 10 Ω`, and gate capacitances `cgs = 4pF`, `cgd = 1.5pF`.
- **`J201`**: Generic low-noise N-channel JFET (`NJF`). Characterized with pinch-off voltage `vto = -0.9V`, `beta = 1.0mA/V²`, `lambda = 5.0m 1/V`, `rd = 15 Ω`, `rs = 15 Ω`, `cgs = 4pF`, `cgd = 1.5pF`, and `is = 1e-14 A`.
- **`2N5460`**: Generic P-channel JFET (`PJF`, version `1.0.0`). Characterized with pinch-off voltage `vto = -2.5V`, transconductance parameter `beta = 1.0mA/V²`, channel-length modulation `lambda = 3.0m 1/V`, ohmic resistances `rd = 15 Ω`, `rs = 15 Ω`, and gate capacitances `cgs = 5pF`, `cgd = 2pF`.

### Built-in diodes, BJTs, and MOSFETs

- **Diodes:** `1N4148` (small-signal), `1N4007` (rectifier), `1N5819` (Schottky barrier diode, `BV=40`, `IS=3.16e-5`, `N=1.08`), and Zeners below.
- **BJTs:** `2N3904` (`npn`), `2N3906` (`pnp`), `2N2222` (`npn`), `BC547` (`npn`, `BF=400`), `BC557` (`pnp`, `BF=330`), `KESSETSU_POWER_NPN_V1`, and `KESSETSU_POWER_PNP_V1`.
- **MOSFETs:** `IRF540` (`nmos` power), `2N7000` (`nmos`, `VTO=2.1`, `KP=0.15`), `BSS84` (`pmos`, `VTO=-1.7`, `KP=0.08`), and `KESSETSU_PMOS_V1`.

### `model jfet` parameter reference

Typed user JFET models use `model jfet <name> njf|pjf version=... license=... [params...]`. Supported parameters:

| Parameter | SPICE name | Description | Default | Unit |
|---|---|---|---|---|
| `vto` | `VTO` | Threshold / pinch-off voltage | -2.0 V (NJF) / -2.0 V (PJF) | V |
| `beta` | `BETA` | Transconductance parameter | 1.0e-4 A/V² | A/V² |
| `lambda` | `LAMBDA` | Channel-length modulation | 0.0 1/V | 1/V |
| `rd` | `RD` | Drain ohmic resistance | 0.0 Ω | Ω |
| `rs` | `RS` | Source ohmic resistance | 0.0 Ω | Ω |
| `cgs` | `CGS` | Zero-bias gate-source junction capacitance | 0.0 F | F |
| `cgd` | `CGD` | Zero-bias gate-drain junction capacitance | 0.0 F | F |
| `pb` | `PB` | Gate junction potential | 1.0 V | V |
| `is` | `IS` | Gate junction saturation current | 1.0e-14 A | A |
| `fc` | `FC` | Forward-bias depletion capacitance coefficient | 0.5 | ratio |
| `b` | `B` | Doping tail parameter | 1.0 | ratio |
| `kf` | `KF` | Flicker noise coefficient | 0.0 | - |
| `af` | `AF` | Flicker noise exponent | 1.0 | - |
| `tnom` | `TNOM` | Parameter measurement temperature | 27.0 °C | °C |

### Built-in Zener breakdown diodes

- `1N4733A`: 5.1 V nominal Zener breakdown diode (`bv=5.1`, `ibv=20mA`).
- `BZX84C3V3`: 3.3 V nominal Zener breakdown diode (`bv=3.3`, `ibv=5mA`).
- `BZX84C5V1`: 5.1 V nominal Zener breakdown diode (`bv=5.1`, `ibv=5mA`).
- `BZX84C12`: 12.0 V nominal Zener breakdown diode (`bv=12.0`, `ibv=5mA`).

### Built-in switch model (`KESSETSU_SW_V1`)

- `KESSETSU_SW_V1`: Generic voltage-controlled switch model mapping to SPICE `SW`. Threshold `vt = 0.5V`, hysteresis `vh = 0V`, on-resistance `ron = 1 Ω`, and off-resistance `roff = 1 MΩ`.

## Characterized external models

### Kessetsu generic comparator v1

- File: [comparator.lib](../../examples/models/comparator.lib), AGPL-3.0-only.
- Entry: `KESSETSU_COMPARATOR_V1`, five terminals.
- SHA-256: `00198a3b00ef90c78b93beb6c7e7ed61c9f0220dc597a6c71cd0661343246c34`.
- Model: smooth 1 mV input transition, 0.1 V output rail headroom, 50 Ω output resistance.
- Covered setup: 5 V single supply, 100 mV-peak / 1 kHz input around a grounded reference,
  10 kΩ load, transient at 1 µs output step for 2 ms. Both polarities of the input
  and loaded output high/low are checked against fixed independent limits.
- Intended for circuit/workflow verification, not a named manufacturer's comparator.
  No delay, hysteresis, offset, noise, input-current, common-mode/rating or thermal fidelity
  is claimed. AC around the transition is not an endorsement of a real comparator's response.
- Native Ngspice and the browser portable profile; expected output low approximately
  99.5 mV and high approximately 4.876 V for this load.

### Published threshold memristor demonstration

- File: [memristor.lib](../../examples/models/memristor.lib), BSD-3-Clause; retain
  [attribution, license and extraction details](../../examples/models/MEMRISTOR_NOTICE.md).
- Entry: `memristor`, two terminals. Adapted from the Ngspice example based on
  [Pershin and Di Ventra's threshold model](https://arxiv.org/abs/1204.2600).
- SHA-256: `cd4bac38581fb00c1b5667e9d5ec440cf954efb5e26105ae2775853cc90301f6`.
- Covered setup: zero-offset 3 V-peak / 100 MHz sine, 100 ps output step for 10 ns,
  `uic`, original demonstration defaults: 1 kΩ/10 kΩ bounds, 7 kΩ initial state,
  1.6 V threshold, `alpha=0`, `beta=20e3/stime`, `stime=10n`.
- The initial-state and waveform histories matter. Use `simulate tran 100ps 10ns uic`:
  this skips the DC operating point and applies capacitor initial conditions. Do not
  add `uic` to ordinary circuits by habit. OP/AC and arbitrary pulse/frequency/default
  combinations are not characterized here. Direct reference comparison uses the same
  exact library/defaults and initial-condition policy.
- These coefficients are artificial demonstrations, not calibration to a fabricated
  device. A hysteresis-shaped plot is not proof of physical fidelity or a research pilot.
- The [reproducible pulse protocol](../guides/memristor-protocol.md) covers a 24-case
  amplitude/duration/starting-state matrix, 25 ps versus 12.5 ps timestep refinement and
  one separately reported held-out stimulus. It characterizes this exact synthetic model;
  it does not validate a fabricated device.
- Measure terminal current with the explicit series zero-volt `VSENSE` source; arbitrary
  subcircuit current/power is not guessed from an internal branch or a resistance value.

### Texas Instruments OPA197 / OPAx197 model

- Existing native workflow, [official manufacturer download](https://www.ti.com/lit/zip/SBOMA34),
  model Final 1.3, 2022-06-23, OPAx197 PSpice Rev. D. File `OPAx197.LIB`, entry `OPAx197`.
- SHA-256: `fc5b020e63346e511bd808bf41c856b0150b000bcf8a41fe00eeececb1f422a5`.
- Interface: `opamp (in_p,in_n,vcc,vee,out)` maps exact positional header
  `IN+ IN- VCC VEE OUT`. Declare `simulator=ngspice_ps redistribution=prohibited`.
- User-acquired local evaluation under TI terms; Kessetsu does not redistribute,
  rewrite, bundle into downloads or silently substitute this manufacturer's file.
- Characterized native OP/AC/transient non-inverting amplifier under the independent
  manufacturer-model evaluator. This evidence does not validate all data-sheet behavior.
- **Native-only:** the browser profile does not enable PSpice compatibility or its
  compiled/code-model dependencies. Selecting it can resolve compilation/geometry;
  browser simulation explicitly refuses the compatibility mode before execution.

## Capability and interchange boundaries

Browser libraries are self-contained Ngspice analog text: `.subckt`/`.ends`, header defaults,
local `.param`/`.func`, and local D/BJT/MOS/switch `.model` types; analog R/C/L/V/I/D/B,
E/G/F/H/Q/M/S and X cards. Subcircuit calls must resolve in the selected library and
instance-level SPICE parameter overrides are not in this initial profile. Conservatively
rejected tokens/directives may require the CLI even if another simulator accepts them.
Controls, includes, `.lib`, file-loading expressions, top-level devices, analysis/output/options,
digital/compiled/plugin cards and unsupported model kinds do not execute on Web. Accepted
syntax can still fail solver convergence; such failures are not rewritten to PASS.

All external libraries, including native ones, reject control/include/library/plugin-loading
directives and require complete, bounded subcircuit headers/closures. Native Ngspice/PSpice
compatibility is broader than the browser profile, not a portability guarantee. OSDI or
user-compiled plugin loading needs a separately designed opt-in workflow.

Schematics, manifests, locks and ordinary exports carry dependency identity, never external
model bodies. Only the ephemeral browser simulation deck receives validated local text.
SPICE/LTspice exports need matching files beside the export at the declared relative paths.
KiCad preserves symbols, pins and model metadata, but is not a complete simulator project.
Two-terminal LTspice instances use the native rectangular outline with explicit `Prefix X`;
they are subcircuits, not resistor primitives. Ngspice model compatibility is not proof of
LTspice simulation compatibility. Read export warnings/losses and keep the original `.kess`.

See [external model declarations](language.md), [exports](exports.md) and
[simulation/assertion contracts](simulation-and-assertions.md).
