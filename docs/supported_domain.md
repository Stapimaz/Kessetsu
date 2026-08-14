# Kessetsu First-Release Support Matrix

This document freezes the electrical scope advertised for Phase 4. “Supported” means more than parsing syntax: typed IR, canonical SPICE, ERC, and the relevant native/Web verification gates must all exist. Features outside this matrix must produce fail-closed diagnostics and must not be presented as approximately supported.

## Component and source scope

| Family | Support | Canonical pins | Boundary |
|---|---|---|---|
| Resistor, capacitor, inductor | Supported | `p1`, `p2` | Ideal lumped element; no tolerance, temperature, or parasitic model |
| Diode | Supported | `p1`, `p2` | Built-in or typed allowlisted model |
| BJT NPN/PNP | Supported | `c`, `b`, `e` | Three-terminal model; no substrate or thermal pin |
| MOSFET NMOS/PMOS | Supported | `d`, `g`, `s` | Three-terminal model; body is not a separate pin |
| Op-amp | Supported | `in_p`, `in_n`, `vcc`, `vee`, `out` | Safe canonical subcircuit template; no arbitrary subcircuits |
| Voltage/current source | Supported | `plus`, `minus` | Typed DC, `sine`, `pulse`, `ac`, and `sine_ac` waveforms |
| Module port | Flattening-only element | Defined by the module | Not a public physical component |

## Model scope

- Built-ins: `2N3904`, `2N3906`, `2N2222`, `KESSETSU_POWER_NPN_V1`, `KESSETSU_POWER_PNP_V1`, `1N4148`, `1N4007`, `IRF540`, `KESSETSU_PMOS_V1`, and `KESSETSU_OPAMP_V1`.
- Verified generic PMOS: `KESSETSU_PMOS_V1@1.0.1`, a portable Ngspice `MOS1` DC model with no manufacturer, datasheet, or parasitic-model claim.
- User models: typed diode/BJT/MOSFET parameter allowlists.
- User subcircuits: the typed op-amp template only.
- Package imports: `kessetsu.models.v1`/`kessetsu.lock.v1` with an exact name and version, content hash, license, and simulator capability.
- Unsupported: raw `.include`, `.model`, `.subckt`, or `.control`; floating package versions; arbitrary vendor script/model injection.

## Analysis, dataset, and measurement scope

| Area | Supported | Explicit boundary |
|---|---|---|
| Analysis | OP, transient, AC decade/linear/octave, independent voltage/current DC sweep | No noise, Monte Carlo, sensitivity, or temperature sweep |
| Dataset | OP scalar; transient/DC real series; AC complex series | Simulator raw format is not a public contract |
| Primitive | `V(net/device)`, `I(device)`, `P(device)` | Safe op-amp internal/output branch current fails closed |
| Reduction | `value`, `min`, `max`, absolute `peak`, `average`, `rms` | No real reduction over complex AC data |
| Derived | gain, bandwidth/cutoff, frequency, phase, output power, efficiency, THD, clipping, dissipation | Only under documented analysis and signal conditions |
| Assertion | `<`, `>`, `==`, `<=`, `>=`; PASS/FAIL/ERROR/SKIPPED | Missing data is never treated as `0` or PASS |

## First-release verification circuits

- RC low-pass: the first vertical Web/CLI parity path.
- Op-amp gain stage: feedback, AC bandwidth, and transient clipping.
- Four-stage 8 Ω power amplifier: the primary product eval for gain, approximately 2 W output, THD, clipping, stress, and dissipation.
- The schematic corpus also covers minimal, Wheatstone-bridge, and high-fan-out topologies.

## Deliberate physical boundaries

Kessetsu's first release is a schematic-level SPICE engineering tool. It does not provide PCB layout/DRC, transmission-line or EM-field solving, RF S-parameter workflows, digital HDL, thermal/aging/reliability analysis, package/PCB parasitic extraction, EMC/ESD analysis, manufacturing-tolerance Monte Carlo, or a datasheet-limit database. Simulation does not replace laboratory measurement or engineering review. Typed domain contracts may expand the supported analog/mixed-signal SPICE scope later; the architecture is not limited to educational circuits.
