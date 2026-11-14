# Components and Simulation Limits

This reference describes Kessetsu 1.5.0's component families, analyses and modeling assumptions.
Unsupported constructs produce diagnostics rather than silently approximate results.

## Component and source scope

| Family | Support | Canonical pins | Boundary |
|---|---|---|---|
| Resistor, capacitor, inductor | Supported | `p1`, `p2` | Ideal lumped element; studies can vary values explicitly, but no implicit tolerance, temperature coefficient or parasitic model |
| Coupled inductors (`couple`) | Supported | Referenced inductor IDs | Magnetic mutual coupling statement: `couple <id> <ind1> <ind2> [<ind3> ...] <k>` (SPICE `K`), supporting two-winding and uniform multi-winding transformers with `0 < k <= 1` and `{...}` coefficient expressions |
| Diode | Supported | `p1`, `p2` | Built-in (`1N4148`, `1N4007`, Schottky `1N5819`, Zeners) or typed allowlisted model with breakdown parameters |
| BJT NPN/PNP | Supported | `c`, `b`, `e` | Three-terminal model (`2N3904`, `2N3906`, `2N2222`, `BC547`, `BC557`, power models); grounded-substrate 4-terminal SPICE cards fold to 3 terminals |
| JFET NJF/PJF | Supported | `d`, `g`, `s` | Three-terminal JFET device family (`jfet <id> [model]`, SPICE `J`), built-in models `2N5457`, `J2N5457`, `J201`, `2N5460` or typed allowlisted `model jfet`; native Ngspice drain current saving (`@J_<id>[id]`) |
| MOSFET NMOS/PMOS | Supported | `d`, `g`, `s` (optional `b`) | Four-terminal or three-terminal model (`IRF540`, `2N7000`, `BSS84`, `KESSETSU_PMOS_V1`); optional bulk terminal `b` defaults to `s` when omitted in `.kess` or folds `b == s` on SPICE import; explicit separate bulk supported in both `.kess` and SPICE import (`M... d g s b model`) |
| Op-amp | Supported | `in_p`, `in_n`, `vcc`, `vee`, `out` | Safe canonical template (`KESSETSU_OPAMP_V1`, `KESSETSU_OPAMP_V2`) or hash-bound external subcircuit; browser portable-profile restrictions apply; no arbitrary raw directives |
| External comparator | Supported | `in_p`, `in_n`, `vcc`, `vee`, `out` | Exact local model; browser portable-profile restrictions apply |
| External two-terminal device | Supported | `p1`, `p2` | Exact local subcircuit, e.g. the characterized memristor demonstration; no universal device fidelity |
| Voltage/current source | Supported | `plus`, `minus` | Typed DC, `sine`, `pulse`, `exp`, `pwl`, small-signal `ac(amplitude)`, biased small-signal `ac(amplitude, dc_offset)` (mapping to SPICE `DC <offset> AC <amplitude>`), and `sine_ac` waveforms |
| VCVS (`vcvs`) | Supported | `p_out`, `n_out`, `ctrl_p`, `ctrl_n` | Linear voltage-controlled voltage source: `vcvs <id> <gain>` (SPICE `E`, dimensionless `ratio`) |
| VCCS (`vccs`) | Supported | `p_out`, `n_out`, `ctrl_p`, `ctrl_n` | Linear voltage-controlled current source: `vccs <id> <gm>` (SPICE `G`, transconductance in Siemens `S`) |
| CCVS (`ccvs`) | Supported | `p_out`, `n_out`, `ctrl_p`, `ctrl_n` | Linear current-controlled voltage source: `ccvs <id> <rm>` (SPICE `H`, transresistance in `Ohm`, internal 0 V sense branch) |
| CCCS (`cccs`) | Supported | `p_out`, `n_out`, `ctrl_p`, `ctrl_n` | Linear current-controlled current source: `cccs <id> <gain>` (SPICE `F`, dimensionless `ratio`, internal 0 V sense branch) |
| Switch (`switch`) | Supported | `p1`, `p2`, `ctrl_p`, `ctrl_n` | Voltage-controlled switch: `switch <id> [model]` (SPICE `S`), default model `KESSETSU_SW_V1` or typed user switch model |
| Module port | Flattening-only element | Defined by the module | Not a public physical component |

### Independent sources

Independent voltage (`source`) and current (`current_source`) elements support typed waveforms:
- DC scalar: `source VIN 5V`
- Small-signal AC stimulus: `ac(amplitude)` (e.g. `source VIN ac(1V)`) for unforced AC frequency sweeps.
- Biased small-signal AC stimulus: `ac(amplitude, dc_offset)` (e.g. `source VIN ac(1V, 5V)`), mapping to SPICE `DC <offset> AC <amplitude>` for small-signal AC analysis around an operating bias point.
- Sine wave: `sine(offset, amplitude, frequency)`
- Sine with AC: `sine_ac(offset, amplitude, frequency, ac_amplitude)` for combined transient and AC frequency-domain sweeps.
- Pulse: `pulse(low, high, delay, rise, fall, width, period)`
- Exponential transient: `exp(v1, v2, td1, tau1, td2, tau2)` (mapping to SPICE `EXP(...)`) for exponential rise/fall stimulus.
- Piecewise linear: `pwl(time, value, ...)` or space-paired `pwl(time value, ...)` (`pwl(0s 0V, 1ms 5V)`) with non-negative, strictly increasing times (with linear time interpolation for `current_source` measurements).
- Node initial conditions: `initial V(NET) = <voltage>` (literal or `{expr}`) emits SPICE `.ic V(<net>)=<voltage>` before `.control` and LTspice `!.ic V(<net>)=<expr>`.

## Model scope

Kessetsu supports catalog-backed external comparator/two-terminal
interfaces and explicit local Web bindings for a bounded self-contained Ngspice text
profile. The [model catalog](model-catalog.md) defines the characterized comparator,
threshold-memristor and native-only manufacturer op-amp cases. This does not imply
arbitrary vendor, memristor or research-device support.

- Built-ins: `2N3904`, `2N3906`, `2N2222`, `BC547`, `BC557`, `KESSETSU_POWER_NPN_V1`, `KESSETSU_POWER_PNP_V1`, `1N4148`, `1N4007`, `1N5819` (Schottky), built-in Zener breakdown diodes (`1N4733A`, `BZX84C3V3`, `BZX84C5V1`, `BZX84C12`), built-in JFETs (`2N5457`, `J2N5457`, `J201`, `2N5460`), `IRF540`, `2N7000`, `BSS84`, `KESSETSU_PMOS_V1`, `KESSETSU_SW_V1`, `KESSETSU_OPAMP_V1`, and `KESSETSU_OPAMP_V2`.
- Builtin `1N4148`/`1N4007` use portable electrical directives in 1.2.0, repairing an
  issue in older downloads. Provenance version `1.0.1` records the repaired hashes without
  changing electrical coefficients. Descriptive ratings are not enforced device limits.
- Built-in Zener diodes and breakdown parameters: built-in models `1N4733A`, `BZX84C3V3`, `BZX84C5V1`, and `BZX84C12` provide characterized Zener breakdown behavior. Typed `model diode` declarations accept breakdown parameters `bv`, `ibv`, `nbv`, `ibvl`, and `nbvl`.
- Verified generic JFETs: `2N5457@1.0.0`, `J2N5457@1.0.0`, and `J201` (N-channel JFETs), `2N5460@1.0.0` (P-channel JFET) with pinch-off and transconductance parameters. Native SPICE runs save JFET drain currents via `@J_<id>[id]`.
- Verified generic switch: `KESSETSU_SW_V1@1.0.0`, a SPICE `SW` model with `vt=0.5`, `vh=0`, `ron=1`, and `roff=1Meg`. Typed user switch models use `model switch <name> version=... license=... vt=... vh=... ron=... roff=...`.
- Verified generic PMOS: `KESSETSU_PMOS_V1@1.0.1`, a portable Ngspice `MOS1` DC model with no manufacturer, datasheet, or parasitic-model claim.
- Verified generic op-amps: `KESSETSU_OPAMP_V1@1.1.0` (anti-windup supply-rail output clamping to `vcc`/`vee`, with safe symmetric clamping when `V(vcc) <= V(vee)`) and `KESSETSU_OPAMP_V2@1.0.0` (differential input stage, dominant-pole gain-bandwidth product `gbw` with `gain=200k` and `bandwidth=1MHz`, slew-rate limiting `slew_rate=500k` V/s, supply-rail output saturation headroom `rail_headroom=1V`, and linear output resistance `rout=50` Ω; does not model a hard nonlinear output-current limit).
- User models: typed diode/BJT/JFET/MOSFET/switch parameter allowlists.
- User subcircuits: the typed op-amp template (supporting required `gain` and `bandwidth`, with optional `rail_headroom`, `slew_rate`, and `rout`) and source-relative, exact-hash external references.
  Portable libraries can use explicit local browser bindings; compatibility modes such as the
  characterized manufacturer PSpice op-amp require the native CLI. External model bodies remain
  user-owned and are never embedded in Kessetsu source, manifests, or ordinary circuit exports.
- Op-amp fidelity: `KESSETSU_OPAMP_V1` connects to `vcc` and `vee` to enforce anti-windup supply-rail output clamping (with safe symmetric clamping when `V(vcc) <= V(vee)`), and `KESSETSU_OPAMP_V2` / parameterized `subcircuit opamp` declarations model rail headroom, slew rate, and linear output resistance. Quiescent supply-current consumption, thermal/SOA effects, manufacturer-specific input stages, and hard nonlinear output-current limits remain unmodeled. Amplifier power, efficiency, and clipping assertions cover only the modeled circuit and supplied measurements, not those missing device effects.
- Package/model records: `kessetsu.models.v3`/`kessetsu.lock.v3` with exact identity, content hash, license, simulator capability, typed external parameter interfaces, and dependency metadata where applicable.
- Unsupported: raw directives in `.kess`, floating package versions, arbitrary vendor scripts,
  model uploads and compiled browser model plugins. Explicit local browser execution is limited
  to the portable library profile in the model catalog.

## SPICE netlist import subset

The canonical SPICE importer (`kess import`, CLI `--input auto|spice`, and `kess mcp`) accepts a declared Ngspice-compatible subset:
- Passive components: `R`, `C`, `L` with literal quantities and European RKM value notation (`4k7`, `1u5`, `2m2`, `4r7`). Trailing SPICE instance parameters (`tc1`, `tc2`, `tol`, `ic`, `m`, `temp`) are tolerated with advisory warning `KES-N011`.
- Coupled inductors: `K` cards (e.g. `K1 L1 L2 0.99` or multi-winding `K1 L1 L2 L3 0.98`) translate to canonical `couple` statements.
- Independent sources: DC, AC, SINE, PULSE, EXP, and PWL `V` and `I` sources, including combined DC bias + AC stimulus (`V1 in 0 DC 5 AC 1 0` -> `ac(1, 5)`) and `EXP(...)` transient waveforms.
- Linear dependent sources: linear `E` (VCVS), `G` (VCCS), `F` (CCCS), and `H` (CCVS) sources; current-controlled sources fold 0 V sense branches into Kessetsu's canonical 4-pin dependent sources.
- Switches: `S` cards with `.model <name> SW` parameters (`vt`, `vh`, `ron`, `roff`) or default `KESSETSU_SW_V1`.
- Diodes and transistors: `D`, `Q`, `J` (JFET), and three- or four-terminal `M` (MOSFET) cards, using built-in models (including Schottky `1N5819`, Zener diodes `1N4733A`, `BZX84C3V3`, `BZX84C5V1`, `BZX84C12`, BJTs `2N3904`, `2N3906`, `2N2222`, `BC547`, `BC557`, MOSFETs `IRF540`, `2N7000`, `BSS84`, and JFETs `2N5457`, `J2N5457`, `J201`, `2N5460`) or typed inline `.model` definitions (including breakdown parameters `bv`, `ibv`, `nbv`, `ibvl`, `nbvl` and JFET parameters). Four-terminal BJT cards automatically fold the substrate terminal when tied to ground (`0`/`GND`) or emitter (emitting advisory warning `KES-N011`). Four-terminal MOSFET cards with bulk tied to source fold to 3-pin `mosfet`, while explicit separate bulk nodes (`b != s`) import with all 4 pins (`d`, `g`, `s`, `b`) connected. Trailing MOSFET instance parameters (`W`, `L`, `AD`, `AS`, `PD`, `PS`, `M`) are tolerated with `KES-N011` warnings.
- Operational amplifiers: 5-terminal `X` cards (`in_p in_n vcc vee out`) referencing `KESSETSU_OPAMP_V1`, `KESSETSU_OPAMP_V2`, or `OPAMP`.
- Initial conditions: root-level `.ic V(<node>)=<value>` directives (one or more space- or comma-separated pairs, including `{expr}` expressions) import cleanly into `initial V(<NET>) = <value>`.
- Parameter evaluation: bounded `.param` assignments and `{...}` expressions evaluate constant arithmetic and math functions (`+`, `-`, `*`, `/`, `^`, `**`, `pi`, `e`, `sqrt`, `sin`, `cos`, `tan`, `exp`, `ln`, `log10`, `abs`, `min`, `max`, `pow`).
- Direct node testing: `.kessreq` and inline assertions can reference imported numeric SPICE node numbers directly (such as `V(1)` and `V(2)`).
- Unsupported constructs (fail closed): non-linear behavioral sources (`VALUE`, `POLY`, `TABLE`, `LAPLACE`, `FREQ`, or `B` cards), arbitrary `.include`/`.lib` file paths, `.step` parameter sweeps, transmission lines (`T`, `TX`), and unknown devices fail closed with source-located diagnostics.

## Analysis, dataset, and measurement scope

| Area | Supported | Explicit boundary |
|---|---|---|
| Single-run analysis | OP, transient, AC decade/linear/octave, independent voltage/current DC sweep | No native noise or sensitivity analysis; multi-condition sampling and temperature are handled by studies below |
| Parameter study | Finite grids, revisions, temperature cases, tolerance corners and seeded uniform/Gaussian samples | Explicit quantities/distributions and finite evaluation budgets; no automatic manufacturing-yield or global-optimum claim |
| Dataset | OP scalar; transient/DC real series; AC complex series | Simulator raw format is not a public contract |
| Primitive | `V(net/device)`, `V(A, B)`, `I(device)`, `P(device)` | `V(A, B)` supports net pairs, terminal pairs, and mixed `component.pin`/net endpoints across OP, TRAN, DC, and AC; safe op-amp internal/output branch current fails closed |
| Reduction | `value`, `min`, `max`, absolute `peak`, `pk2pk` (`p2p`, `vpp`), `average`, `rms` | No real reduction over complex AC data |
| Derived | `gain`, `gain_at`, `bandwidth`/`cutoff`, `lower_cutoff`, `upper_cutoff`, `frequency`/`freq`, `period`, `duty_cycle`/`duty`, `phase`, `output_power`, `efficiency`, `thd` (2/4/5-arg), `clipping`, `dissipation`, `rise_time`/`fall_time`/`settling_time`, `overshoot`, `energy` | Only under documented analysis, signal and window conditions |
| Engineering calculators | `divider`, `rc-lowpass`, `non-inverting-opamp`, `zener-regulator` | Analytical sizing with `exact`/`e12`/`e24` selection across Core, `kess tool`, `kessetsu_tool` MCP, and Web Hub `/tools/`; generates `.kess` for Ngspice verification |
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
- Zener shunt regulator (`zener_shunt_regulator.kess`): 12 V to 5.1 V shunt regulation with built-in `BZX84C5V1` Zener breakdown diode.
- Switched sample-and-hold (`switched_sample_hold.kess`): pulsed clocking with `switch` (`model switch SW_SH`) and hold capacitor.
- Current-sense amplifier (`current_sense_amplifier.kess`): load current sensing using `ccvs` and rail-clamped op-amp (`KESSETSU_OPAMP_V2`).
- The schematic corpus also covers minimal, Wheatstone-bridge, and high-fan-out topologies.

## Deliberate physical boundaries

Kessetsu is a schematic-level SPICE engineering tool. It does not provide PCB layout/DRC,
EM-field solving, RF S-parameter workflows, digital HDL, thermal/aging/reliability analysis,
PCB parasitic extraction, EMC/ESD analysis, validated manufacturing-yield prediction or a
datasheet-limit database. It can retain and compare explicitly supplied ratings, conditions and
citations, but does not verify those records or infer unprovided derating/SOA curves. Simulation
does not replace laboratory measurement or engineering review. These are current capability
boundaries, not a restriction to educational circuits.
