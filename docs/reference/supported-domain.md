# Components and Simulation Limits

This reference describes Kessetsu 1.3.0's component families, analyses and modeling assumptions.
Unsupported constructs produce diagnostics rather than silently approximate results.

## Component and source scope

| Family | Support | Canonical pins | Boundary |
|---|---|---|---|
| Resistor, capacitor, inductor | Supported | `p1`, `p2` | Ideal lumped element; studies can vary values explicitly, but no implicit tolerance, temperature coefficient or parasitic model |
| Diode | Supported | `p1`, `p2` | Built-in or typed allowlisted model |
| BJT NPN/PNP | Supported | `c`, `b`, `e` | Three-terminal model; no substrate or thermal pin |
| MOSFET NMOS/PMOS | Supported | `d`, `g`, `s` | Three-terminal model; body is not a separate pin |
| Op-amp | Supported | `in_p`, `in_n`, `vcc`, `vee`, `out` | Safe canonical template or hash-bound external subcircuit; browser portable-profile restrictions apply; no arbitrary raw directives |
| External comparator | Supported | `in_p`, `in_n`, `vcc`, `vee`, `out` | Exact local model; browser portable-profile restrictions apply |
| External two-terminal device | Supported | `p1`, `p2` | Exact local subcircuit, e.g. the characterized memristor demonstration; no universal device fidelity |
| Voltage/current source | Supported | `plus`, `minus` | Typed DC, `sine`, `pulse`, `pwl`, `ac`, and `sine_ac` waveforms |
| VCVS (`vcvs`) | Supported | `p_out`, `n_out`, `ctrl_p`, `ctrl_n` | Linear voltage-controlled voltage source: `vcvs <id> <gain>` (SPICE `E`, dimensionless `ratio`) |
| VCCS (`vccs`) | Supported | `p_out`, `n_out`, `ctrl_p`, `ctrl_n` | Linear voltage-controlled current source: `vccs <id> <gm>` (SPICE `G`, transconductance in Siemens `S`) |
| CCVS (`ccvs`) | Supported | `p_out`, `n_out`, `ctrl_p`, `ctrl_n` | Linear current-controlled voltage source: `ccvs <id> <rm>` (SPICE `H`, transresistance in `Ohm`, internal 0 V sense branch) |
| CCCS (`cccs`) | Supported | `p_out`, `n_out`, `ctrl_p`, `ctrl_n` | Linear current-controlled current source: `cccs <id> <gain>` (SPICE `F`, dimensionless `ratio`, internal 0 V sense branch) |
| Switch (`switch`) | Supported | `p1`, `p2`, `ctrl_p`, `ctrl_n` | Voltage-controlled switch: `switch <id> [model]` (SPICE `S`), default model `KESSETSU_SW_V1` or typed user switch model |
| Module port | Flattening-only element | Defined by the module | Not a public physical component |

## Model scope

Kessetsu supports catalog-backed external comparator/two-terminal
interfaces and explicit local Web bindings for a bounded self-contained Ngspice text
profile. The [model catalog](model-catalog.md) defines the characterized comparator,
threshold-memristor and native-only manufacturer op-amp cases. This does not imply
arbitrary vendor, memristor or research-device support.

- Built-ins: `2N3904`, `2N3906`, `2N2222`, `KESSETSU_POWER_NPN_V1`, `KESSETSU_POWER_PNP_V1`, `1N4148`, `1N4007`, built-in Zener breakdown diodes (`1N4733A`, `BZX84C3V3`, `BZX84C5V1`, `BZX84C12`), `IRF540`, `KESSETSU_PMOS_V1`, `KESSETSU_SW_V1`, `KESSETSU_OPAMP_V1`, and `KESSETSU_OPAMP_V2`.
- Builtin `1N4148`/`1N4007` use portable electrical directives in 1.2.0, repairing an
  issue in older downloads. Provenance version `1.0.1` records the repaired hashes without
  changing electrical coefficients. Descriptive ratings are not enforced device limits.
- Built-in Zener diodes and breakdown parameters: built-in models `1N4733A`, `BZX84C3V3`, `BZX84C5V1`, and `BZX84C12` provide characterized Zener breakdown behavior. Typed `model diode` declarations accept breakdown parameters `bv`, `ibv`, `nbv`, `ibvl`, and `nbvl`.
- Verified generic switch: `KESSETSU_SW_V1@1.0.0`, a SPICE `SW` model with `vt=0.5`, `vh=0`, `ron=1`, and `roff=1Meg`. Typed user switch models use `model switch <name> version=... license=... vt=... vh=... ron=... roff=...`.
- Verified generic PMOS: `KESSETSU_PMOS_V1@1.0.1`, a portable Ngspice `MOS1` DC model with no manufacturer, datasheet, or parasitic-model claim.
- Verified generic op-amps: `KESSETSU_OPAMP_V1@1.1.0` (anti-windup supply-rail output clamping to `vcc`/`vee`, with safe symmetric clamping when `V(vcc) <= V(vee)`) and `KESSETSU_OPAMP_V2@1.0.0` (`gain=200k`, `bandwidth=1MHz`, `rail_headroom=1V`, `slew_rate=500k` V/s, `rout=50` Ω linear output resistance).
- User models: typed diode/BJT/MOSFET/switch parameter allowlists.
- User subcircuits: the typed op-amp template (supporting required `gain` and `bandwidth`, with optional `rail_headroom`, `slew_rate`, and `rout`) and source-relative, exact-hash external references.
  Portable libraries can use explicit local browser bindings; compatibility modes such as the
  characterized manufacturer PSpice op-amp require the native CLI. External model bodies remain
  user-owned and are never embedded in Kessetsu source, manifests, or ordinary circuit exports.
- Op-amp fidelity: `KESSETSU_OPAMP_V1` connects to `vcc` and `vee` to enforce anti-windup supply-rail output clamping (with safe symmetric clamping when `V(vcc) <= V(vee)`), and `KESSETSU_OPAMP_V2` / parameterized `subcircuit opamp` declarations model rail headroom, slew rate, and linear output resistance. Quiescent supply-current consumption, thermal/SOA effects, and manufacturer-specific input stages remain unmodeled. Amplifier power, efficiency, and clipping assertions cover only the modeled circuit and supplied measurements, not those missing device effects.
- Package/model records: `kessetsu.models.v3`/`kessetsu.lock.v3` with exact identity, content hash, license, simulator capability, typed external parameter interfaces, and dependency metadata where applicable.
- Unsupported: raw directives in `.kess`, floating package versions, arbitrary vendor scripts,
  model uploads and compiled browser model plugins. Explicit local browser execution is limited
  to the portable library profile in the model catalog.

## SPICE netlist import subset

The canonical SPICE importer (`kess import`, CLI `--input auto|spice`, and `kess mcp`) accepts a declared Ngspice-compatible subset:
- Passive components: `R`, `C`, `L` with literal quantities.
- Independent sources: DC, AC, SINE, PULSE, and PWL `V` and `I` sources.
- Linear dependent sources: linear `E` (VCVS), `G` (VCCS), `F` (CCCS), and `H` (CCVS) sources; current-controlled sources fold 0 V sense branches into Kessetsu's canonical 4-pin dependent sources. Non-linear behavioral constructs (`VALUE`, `POLY`, `TABLE`, `LAPLACE`, `FREQ`, or `B` cards) fail closed.
- Switches: `S` cards with `.model <name> SW` parameters (`vt`, `vh`, `ron`, `roff`) or default `KESSETSU_SW_V1`.
- Diodes and transistors: `D`, `Q`, and three-terminal `M` (or four-terminal with bulk tied to source) cards, using built-in models (including Zener diodes `1N4733A`, `BZX84C3V3`, `BZX84C5V1`, `BZX84C12`) or typed inline `.model` definitions (including breakdown parameters `bv`, `ibv`, `nbv`, `ibvl`, `nbvl`).
- Operational amplifiers: 5-terminal `X` cards (`in_p in_n vcc vee out`) referencing `KESSETSU_OPAMP_V1`, `KESSETSU_OPAMP_V2`, or `OPAMP`.
- Direct node testing: `.kessreq` and inline assertions can reference imported numeric SPICE node numbers directly (such as `V(1)` and `V(2)`).

## Analysis, dataset, and measurement scope

| Area | Supported | Explicit boundary |
|---|---|---|
| Single-run analysis | OP, transient, AC decade/linear/octave, independent voltage/current DC sweep | No native noise or sensitivity analysis; multi-condition sampling and temperature are handled by studies below |
| Parameter study | Finite grids, revisions, temperature cases, tolerance corners and seeded uniform/Gaussian samples | Explicit quantities/distributions and finite evaluation budgets; no automatic manufacturing-yield or global-optimum claim |
| Dataset | OP scalar; transient/DC real series; AC complex series | Simulator raw format is not a public contract |
| Primitive | `V(net/device)`, `I(device)`, `P(device)` | Safe op-amp internal/output branch current fails closed |
| Reduction | `value`, `min`, `max`, absolute `peak`, `average`, `rms` | No real reduction over complex AC data |
| Derived | gain, gain_at, bandwidth/cutoff, lower_cutoff, upper_cutoff, frequency, phase, output power, efficiency, THD, clipping, dissipation, rise/fall/settling time, overshoot, energy | Only under documented analysis, signal and window conditions |
| Assertion | `<`, `>`, `==`, `<=`, `>=`; PASS/FAIL/ERROR/SKIPPED | Missing data is never treated as `0` or PASS |
| Provided part limits | Condition-qualified peak voltage/current and average dissipation | User-supplied advisory comparison; no SOA, thermal or datasheet-compliance inference |

## Studies and measured data

Use [parameter studies](../guides/parameter-studies.md) to repeat a circuit across selected
conditions. Temperature cases pass explicit temperatures to Ngspice; only temperature-dependent
model equations respond. Ideal passive values do not acquire temperature coefficients or random
tolerances automatically. Sampled studies can estimate outcomes for the distributions you provide;
they do not establish physical production yield. For measured data and finite calibration with
separate holdout validation, see [research data](../guides/research-data.md) and
[model fitting](../guides/model-fitting.md).

## Example circuits

- RC low-pass: the first vertical Web/CLI parity path.
- Op-amp gain stage: feedback, AC bandwidth, and transient clipping.
- Four-stage 8 Ω power amplifier: the primary product eval for gain, approximately 2 W output, THD, clipping, stress, and dissipation.
- The schematic corpus also covers minimal, Wheatstone-bridge, and high-fan-out topologies.

## Deliberate physical boundaries

Kessetsu is a schematic-level SPICE engineering tool. It does not provide PCB layout/DRC,
EM-field solving, RF S-parameter workflows, digital HDL, thermal/aging/reliability analysis,
PCB parasitic extraction, EMC/ESD analysis, validated manufacturing-yield prediction or a
datasheet-limit database. It can retain and compare explicitly supplied ratings, conditions and
citations, but does not verify those records or infer unprovided derating/SOA curves. Simulation
does not replace laboratory measurement or engineering review. These are current capability
boundaries, not a restriction to educational circuits.
