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
| Module port | Flattening-only element | Defined by the module | Not a public physical component |

## Model scope

Kessetsu supports catalog-backed external comparator/two-terminal
interfaces and explicit local Web bindings for a bounded self-contained Ngspice text
profile. The [model catalog](model-catalog.md) defines the characterized comparator,
threshold-memristor and native-only manufacturer op-amp cases. This does not imply
arbitrary vendor, memristor or research-device support.

- Built-ins: `2N3904`, `2N3906`, `2N2222`, `KESSETSU_POWER_NPN_V1`, `KESSETSU_POWER_PNP_V1`, `1N4148`, `1N4007`, `IRF540`, `KESSETSU_PMOS_V1`, and `KESSETSU_OPAMP_V1`.
- Builtin `1N4148`/`1N4007` use portable electrical directives in 1.2.0, repairing an
  issue in older downloads. Provenance version `1.0.1` records the repaired hashes without
  changing electrical coefficients. Descriptive ratings are not enforced device limits.
- Verified generic PMOS: `KESSETSU_PMOS_V1@1.0.1`, a portable Ngspice `MOS1` DC model with no manufacturer, datasheet, or parasitic-model claim.
- User models: typed diode/BJT/MOSFET parameter allowlists.
- User subcircuits: the typed op-amp template and source-relative, exact-hash external references.
  Portable libraries can use explicit local browser bindings; compatibility modes such as the
  characterized manufacturer PSpice op-amp require the native CLI. External model bodies remain
  user-owned and are never embedded in Kessetsu source, manifests, or ordinary circuit exports.
- Op-amp fidelity: the current generic template is a controlled voltage source with an RC pole. Its declared supply pins are unused internally; it does not model supply consumption, rail saturation, or a realistic output-current limit. Amplifier power, efficiency, and clipping assertions cover only the modeled circuit and supplied measurements, not those missing device effects.
- Package/model records: `kessetsu.models.v3`/`kessetsu.lock.v3` with exact identity, content hash, license, simulator capability, typed external parameter interfaces, and dependency metadata where applicable.
- Unsupported: raw directives in `.kess`, floating package versions, arbitrary vendor scripts,
  model uploads and compiled browser model plugins. Explicit local browser execution is limited
  to the portable library profile in the model catalog.

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
