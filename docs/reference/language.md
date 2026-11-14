# Kessetsu Language Reference

This document describes Kessetsu 1.5.0. Kessetsu is line-oriented,
case-sensitive except for documented device polarities, and uses `//` comments. Backends
never consume syntax directly: source is parsed, flattened and validated into typed Circuit IR first.

## Names and values

Identifiers begin with an ASCII letter and continue with letters, digits or `_`. Component and net names share a namespace and must be unique. Engineering values accept SI suffixes such as `10k`, `2.2uF`, `100mV`, `1kHz`, `45deg` and `3%`; the expected physical dimension is checked from context.

## Nets, components and sources

```kessetsu
net GND
net IN
net OUT

resistor R1 1k
capacitor C1 159.154943nF
inductor L1 10mH
inductor L2 10mH
couple K1 L1 L2 0.99

diode D1 1N4148
transistor Q1 npn 2N3904
jfet J1 2N5457
mosfet M1 IRF540
switch S1 KESSETSU_SW_V1
opamp U1 KESSETSU_OPAMP_V1

vcvs E1 10
vccs G1 5mS
ccvs H1 1kOhm
cccs F1 50

source VDC 5V
source VIN sine_ac(0V,100mV,1kHz,1V)
source VBIAS ac(1V,2.5V)
source VEXP exp(0V,5V,1ms,0.5ms,5ms,1ms)
current_source IBIAS 1mA
```

Source waveforms are typed:
- Scalar DC: `5V` or `source VDC 5V`
- Unforced small-signal AC: `ac(amplitude)` (e.g. `ac(1V)`)
- Biased small-signal AC: `ac(amplitude, offset)` (e.g. `ac(1V, 2.5V)` generating SPICE `DC 2.5 AC 1`)
- Sine wave: `sine(offset, amplitude, frequency)`
- Sine with AC: `sine_ac(offset, amplitude, frequency, ac_amplitude)`
- Pulse: `pulse(low, high, delay, rise, fall, width, period)`
- Exponential transient: `exp(v1, v2, td1, tau1, td2, tau2)` where `v1` is initial value, `v2` is pulsed value, `td1` is rise delay, `tau1` is rise time constant, `td2` is fall delay, and `tau2` is fall time constant.
- Piecewise linear: `pwl(time, value, ...)` or space-paired `pwl(time value, ...)` (for example `pwl(0s 0V, 1ms 5V)` or `pwl(0s 0V 1ms 5V)`) with at least two pairs and non-negative, strictly increasing times. Supported on both `source` and `current_source` (with linear time interpolation for current-source measurements).

See [supported domain](supported-domain.md) for component and model limits.

### Magnetically coupled inductors (transformers)

The `couple` statement defines mutual magnetic coupling between two or more inductors:

```kessetsu
couple <name> <ind1> <ind2> [<ind3> ...] <coefficient>
```

For a two-winding transformer:

```kessetsu
inductor L1 10mH
inductor L2 10mH
couple K1 L1 L2 0.99
```

For multi-winding coupling with uniform coefficient:

```kessetsu
inductor L1 10mH
inductor L2 5mH
inductor L3 5mH
couple K1 L1 L2 L3 0.98
```

Coupling coefficients must satisfy `0 < coefficient <= 1` and accept braced parameter expressions (e.g. `couple K1 L1 L2 {coupling_factor}`). In SPICE netlist generation, this maps directly to `K<name> <ind1> <ind2> ... <coefficient>`.

### Node initial conditions (`initial`)

The `initial` statement sets a starting node voltage for transient simulation (SPICE `.ic`):

```kessetsu
initial V(OUT) = 2.5V
initial V(CAP_NODE) = {v_init}
```

Both `initial` and `V(...)` are case-insensitive. The target net must be a declared or connected non-ground net (case-insensitive lookup; `0` and `GND` are rejected, and duplicate `initial` statements for the same net fail compilation). Inside a `module`, `initial` may target a module-local net (qualified during flattening and evaluated in the module's parameter scope), but cannot target a module port directly. The voltage must resolve to a finite `Volt` quantity. In SPICE netlist generation, each declaration emits `.ic V(<net>)=<voltage>` before `.control` (commonly paired with `simulate tran <step> <stop> uic` when skipping the DC operating point), and LTspice `.asc` export emits `!.ic V(<net>)=<expr>`.

## Named quantities and arithmetic

```kessetsu
param supply: V = 12V
param resistance: Ohm = 10k
param target_gain: ratio = 20
param gm: S = 5mS
source VCC {supply}
resistor RG {resistance}
resistor RF {(target_gain - 1) * resistance}
```

Parameters are optional, declared at the top level or inside modules, and explicitly typed: `Ohm`, `F`, `H`, `S` (Siemens/mho for conductance/transconductance), `V`, `A`, `Hz`, `s`, `W`, `J`, `ratio`, `percent` or `deg`. Forward references are allowed; duplicate/unknown names and dependency cycles produce errors. Constants `pi` and `e` are read-only. Expressions support finite SI literals, names, parentheses, unary signs, arithmetic operators (`+ - * /`), power/exponentiation (`^` and `**`), and mathematical functions: `sqrt`, `sin`, `cos`, `tan`, `ln`, `log10`, `exp`, `abs`, `min`, `max`, and `pow`. European RKM value notation (e.g. `4k7`, `1u5`, `2m2`, `4r7`) is also supported in value literals.

Braces are accepted in resistor, capacitor, inductor and scalar DC voltage/current-source
values. A whole literal keeps contextual units (`10k` in an Ohm declaration); bare literals
inside arithmetic are dimensionless. Use `resistance + 1kOhm`, not `resistance + 1000`.
Percentages normalize to fractions during arithmetic (`5% * 12V` is 0.6 V), while a
`percent` quantity/threshold is expressed in percentage points. Angles cannot become ratios.

Parameters do not infer topology or guarantee a named target: the circuit must explicitly
use the relationship, then simulation/assertions verify its actual behavior. Supported
waveform and analysis numeric fields also accept braced expressions; names, model fields
and other non-numeric fields do not accept expressions in this slice. Inline assertions
accept expressions in their thresholds and supported numeric measurement arguments.
Kessetsu 1.2.0 accepts numeric root `--param` inputs; see the [CLI contract](cli.md#root-parameter-inputs).
Module defaults and component expressions
do not implicitly capture global or caller parameters. Evaluator-owned
`.kessreq` files remain independent and assertion-only.

### Independent module parameters

```kessetsu
param base_cutoff: Hz = 500Hz
module RC(input,output,gnd) {
  param resistance: Ohm = 1k
  param cutoff: Hz = 1k
  param capacitance: F = 1 / (2 * pi * resistance * cutoff)
  resistor R1 {resistance}
  capacitor C1 {capacitance}
  connect input to R1.p1
  connect R1.p2, C1.p1 to output
  connect C1.p2 to gnd
}
use RC SLOW(cutoff={base_cutoff})
use RC FAST(cutoff={base_cutoff * 4})
```

This is a block-definition example, not a complete connected simulation. See the
[two-filter fixture](https://github.com/Stapimaz/Kessetsu/blob/main/core/tests/fixtures/parameters/rc_instances.kess) for sources,
external port connections, AC analysis and fixed assertions.

`use RC DEFAULT` keeps defaults. Named overrides accept whole literals with the child's
declared unit (`cutoff=500Hz`, `resistance=2k`) or braced expressions evaluated in the
caller's parameter scope. Defaults and component expressions use only the module's own
parameters and `pi`; pass parent values explicitly. Nested instances follow the same rule.
Each instance resolves independently, including forward references and recalculated
defaults. Every parameter is overrideable, including calculated capacitance: choosing
a standard capacitor does not guarantee the originally named cutoff target.

Duplicate/unknown overrides and incompatible units fail compilation. All default
expressions must have valid names and units even when overridden. Dependency cycles are
checked in effective definitions; an override may deliberately break a default cycle.
Electrical IDs retain the existing underscore prefixes (`SLOW_R1`), while parameter
provenance carries separate instance-path segments, defaults, overrides and qualified
dependencies. Ambiguous flattened paths are rejected rather than silently renamed.
Declared interface pins are checked by ERC. Module-local named nets are scoped too.

Put analyses and assertions at the root: parameterized module-local analysis/assertion
contexts are not supported yet and fail explicitly. Kessetsu accepts root CLI inputs
such as `--param supply=15V`; see the [CLI contract](cli.md#root-parameter-inputs)
for effective-source export and reproducibility. No model-name expressions,
automatic topology generation or extra editor panel is introduced.

### Compilation safety

Source builds bound input to 8 MiB and 100,000 parsed declarations/statements; module
expansion also stops at 100,000 statements and 64 instance levels. At most 4,096 effective
parameter definitions are accepted across the expanded circuit. Each arithmetic expression
is limited to 16 KiB, 1,024 nodes and depth 64. Parsing, elaboration and numeric conversion
have separate 1,000,000-node expression-work ceilings; these are bounded passes, not an
execution-time guarantee or a claimed single aggregate operation count. Oversized/unsupported
input fails with diagnostics, without backend artifacts. Waveforms accept numeric arguments,
not nested waveform/function calls. Existing literal/quoted waveforms remain supported.

Parameter diagnostics carry the actual declaration or supplied module-override location,
including nested instances. Default provenance still identifies its original declaration.
These precise parameter origins do not imply every ERC/component/measurement diagnostic
has a complete source map.

## Connections and pins

```kessetsu
connect VIN.plus to IN
connect VIN.minus, C1.p2 to GND
connect R1.p1 to IN
connect R1.p2, C1.p1 to OUT
connect M1.d to OUT
connect M1.g to IN
connect M1.s to GND
// Optional 4th bulk terminal on MOSFETs:
connect M1.b to GND
```

Canonical pins by component family:
- Two-terminal passives (`resistor`, `capacitor`, `inductor`) and `diode`: `p1`, `p2`
- Independent sources (`source`, `current_source`): `plus`, `minus`
- BJTs (`transistor`): `c` (collector), `b` (base), `e` (emitter)
- JFETs (`jfet`): `d` (drain), `g` (gate), `s` (source)
- MOSFETs (`mosfet`): `d` (drain), `g` (gate), `s` (source), and optional `b` (bulk). When `b` is left unconnected in `.kess`, it defaults to the source terminal `s` in SPICE netlist generation (`M... d g s s model`) and is omitted from 3-pin schematic symbols. When `b` is explicitly connected (`connect M1.b to ...`), all 4 terminals are emitted in SPICE (`M... d g s b model`) and rendered on 4-terminal schematic symbols.
- Voltage-controlled switches (`switch`): `p1`, `p2` (switched path), `ctrl_p`, `ctrl_n` (controlling voltage terminals)
- Dependent sources:
  - `vcvs`: `p_out`, `n_out` (output voltage), `ctrl_p`, `ctrl_n` (controlling voltage)
  - `vccs`: `p_out`, `n_out` (output current), `ctrl_p`, `ctrl_n` (controlling voltage)
  - `ccvs`: `p_out`, `n_out` (output voltage), `ctrl_p`, `ctrl_n` (controlling current sense branch)
  - `cccs`: `p_out`, `n_out` (output current), `ctrl_p`, `ctrl_n` (controlling current sense branch)
- Operational amplifiers (`opamp`): `in_p`, `in_n`, `vcc`, `vee`, `out`

Every required pin must be connected. `GND` is the reference net; ambiguous or missing reference topology fails ERC.

## Analyses and assertions

### Parameterized excitation and analyses

```kessetsu
param amplitude: V = 1V
param frequency: Hz = 1kHz
param period: s = 1 / frequency
param points: ratio = 80
source VIN sine_ac(0V,{amplitude},{frequency},{amplitude})
simulate ac dec {points} {frequency / 100} {frequency * 100}
simulate tran {period / 100} {period * 10}
```

This excerpt illustrates numeric fields; the [complete RC fixture](https://github.com/Stapimaz/Kessetsu/blob/main/core/tests/fixtures/parameters/source_analysis.kess)
includes connections and fixed requirements. One frequency controls the excitation, the
AC range and a ten-period transient window without duplicating numbers. Existing literal
syntax remains valid; use braces only where a formula or parameter is useful.

All numeric arguments of unquoted `ac`, `sine`, `sine_ac`, `pulse`, `exp` and `pwl` calls accept
expressions. Voltage/current levels use the source's unit, frequency uses Hz, and times
use seconds. PWL accepts comma-separated arguments (`pwl(0s,0V,1ms,5V)`), comma-separated
time/value pairs (`pwl(0s 0V, 1ms 5V)`), or space-separated pairs (`pwl(0s 0V 1ms 5V)`),
including braced expressions such as `pwl(0s 0V, {t1} {v1})`, while keeping alternating
time/value arguments and strictly-increasing-time checks.
Legacy quoted SPICE-style waveforms remain literal-only: do not put `{name}` inside quotes.
Module waveform expressions use the module's own parameter scope, including overrides.

Transient step/stop, AC points/start/stop and DC start/stop/step accept expressions.
AC points must resolve to a positive dimensionless integer within the supported unsigned
32-bit range; `param points: ratio = 80` is the straightforward declaration. Scale keywords
(`dec`, `oct`, `lin`) and DC source names are not numeric fields. Existing analysis arity,
unit, sweep-direction and uniqueness checks apply after values resolve. Put analyses at
the circuit root. Independently owned requirements cannot refer to design parameters.
For combined AC/transient simulations,
use explicit transient time windows when asserting waveform reductions such as RMS.

### Parameterized inline measurements

```kessetsu
param settling: s = 2ms
param duration: s = 10ms
param frequency: Hz = 1kHz
param maximum: V = 0.55V
assert rms(V(OUT),{settling},{duration}) < {maximum}
assert gain_at(V(OUT),V(IN),{frequency}) > 0.70
```

An excerpt, not a complete circuit; the [measurement fixture](https://github.com/Stapimaz/Kessetsu/blob/main/core/tests/fixtures/parameters/assertions.kess)
provides connections, excitation and analyses. Threshold units come from the metric:
V/A/W for voltage/current/power reductions, ratio for gain, Hz for frequency/cutoff,
degrees for phase and percent for THD/clipping/efficiency. Whole-literal shorthand remains
contextual; named or compound expressions must have compatible dimensions.

Supported numeric arguments are transient start/stop windows for reductions, output_power,
dissipation and efficiency; target/reference frequency for gain_at, phase and cutoff edges;
THD fundamental/time window; and clipping voltage rails. Signals, load/device names and
THD's `hann` keyword remain literal. Measurement semantics/interpolation are unchanged.
Expressions resolve to typed quantities before simulation, not substituted SPICE text.
Place parameterized assertions at the root; module-local target qualification is not supported.

These are **design-owned checks**: a parameterized limit changes if you explicitly edit its
definition. Do not derive a fixed acceptance limit from the value being tested merely to
make a failing design pass. Independent `.kessreq` files remain literal-only and reject
braced expressions, even constants; design parameters cannot change evaluator-owned limits.

### Literal analyses and requirements

```kessetsu
simulate op
simulate tran 10us 5ms
simulate ac dec 30 10Hz 10MHz
simulate dc VIN -1V 1V 10mV

assert gain(V(OUT),V(IN)) > 9.9
assert rms(V(OUT),2ms,5ms) < 6V
assert peak(V(Q1.c,Q1.e)) < 40V
```

Analysis arguments and assertions are dimension checked. Operating-point, transient, and AC analyses may each appear once; a DC sweep may appear once per independent source. Unsupported assertion metrics and ambiguous repeated analyses fail semantic validation before simulation. Missing signals or incompatible datasets become explicit errors, never implicit zeroes. Full formulas and sign conventions are in [engineering measurements](measurements.md); execution behavior is in [simulation and assertions](simulation-and-assertions.md).

An evaluator-owned `.kessreq` file uses the same assertion syntax but permits only comments and one or more `assert` statements. It is supplied to `kess test` with `--requirements`; a design using that option must not also define inline assertions. This is a CLI composition contract, not a second language or a backend bypass.

## Modules

Modules provide reusable topology. A module instance is flattened before semantic analysis; backend-specific module shortcuts do not exist.

For complete parameterized filter/amplifier circuits and a step-by-step workflow, see
[reuse a circuit block](../guides/cookbook.md#reuse-a-circuit-block).
Module definitions are source-local, not imported circuit packages. Structured instance
paths survive in IR interface metadata independently of flattened electrical identifiers.

```kessetsu
module divider(p1,p2) {
  resistor TOP 10k
  resistor BOTTOM 10k
  connect p1 to TOP.p1
  connect TOP.p2 to BOTTOM.p1
  connect BOTTOM.p2 to p2
}

use divider DIV1
```

## Physical part assignments

An electrical component may carry a separate physical-part assignment:

```kessetsu
resistor R1 10k
part R1 manufacturer="Yageo" mpn="RC0603FR-0710KL" footprint="Resistor_SMD:R_0603_1608Metric" pin_map="p1:1,p2:2" average_dissipation_limit="0.1W" average_dissipation_conditions="70 C ambient; apply manufacturer derating" rating_source="User-supplied manufacturer record" note="Verify stock before build"
```

Identity/handoff fields are `manufacturer`, `mpn`, `footprint`, `pin_map` and `note`; omit unknown
information instead of using a guessed placeholder. A pin map is optional, but when present it
requires a footprint and must map every logical catalog pin exactly once to a unique physical pad.

Optional provided-rating pairs are:

- `peak_voltage_limit` (V) with `peak_voltage_conditions`;
- `peak_current_limit` (A) with `peak_current_conditions`;
- `average_dissipation_limit` (W) with `average_dissipation_conditions`.

Every limit must be a finite positive typed quantity and its matching conditions are mandatory.
`rating_source` is an optional user-supplied citation shared by the ratings on that assignment; a
rating without it is explicitly user-entered, not silently verified. `simulate` and `test` compare
available model stress against these values in the separate `kessetsu.part-stress.v1` report.
Peak voltage uses the component's canonical terminal pair, peak current uses its supported device
current, and average dissipation uses positive transient power. Unsupported models or missing
analysis data are reported as unavailable. Exceeding a provided rating is prominent but advisory:
it does not replace explicit design/evaluator assertions or change their exit status.

The assignment is stored in the versioned physical-part manifest and never changes component
value, model, connectivity or generated simulation netlist.

Place `part` beside a component inside a module to assign each flattened instance independently.
A module interface itself is virtual and cannot receive a physical part. Manufacturer identity is
not a device model, footprint choice is not pin mapping, and a supplied limit is not proof of full
datasheet compliance, safe operating area, thermal safety or availability. Independent
voltage/current sources are simulation stimuli and are not BOM parts; represent a real sourced
device through an appropriate typed component.

## Typed models and packages

```kessetsu
model diode SafeD version=1.0.0 license=MIT Is=2e-9 Rs=0.5
model diode 1N4733A version=1.0.0 license=MIT Is=2e-9 Rs=0.5 Bv=5.1 Ibv=10mA
model bjt SafeN npn version=1.0.0 license=MIT Is=1e-12 Bf=100
model jfet 2N5457 njf version=1.0.0 license=MIT vto=-3.0 beta=1.5m lambda=0.01
model switch SW_FAST version=1.0.0 license=MIT vt=0.5 vh=0 ron=0.1 roff=10Meg
model_include kessetsu_analog 1.0.0
external_subcircuit opamp OPA197 (in_p,in_n,vcc,vee,out) file="models/OPAx197.LIB" entry=OPAx197 sha256=<64-hex-digest> version="Final 1.3" license="vendor terms" source="vendor URL" simulator=ngspice_ps redistribution=prohibited
```

Raw `.include`, `.model`, `.subckt` and `.control` injection is intentionally rejected. Model kinds have parameter allowlists:
- **`model diode <name>`**: parameters include `is`, `rs`, `n`, `tt`, `cjo`, `vj`, `m`, `eg`, `xti`, `fc`, plus breakdown parameters `bv` (reverse breakdown voltage), `ibv` (current at breakdown voltage), `nbv` (breakdown emission coefficient), `ibvl` (low-level reverse breakdown knee current), and `nbvl` (low-level breakdown emission coefficient). Built-in diodes: `1N4148`, `1N4007`, Schottky `1N5819`, and Zener diodes `1N4733A` (5.1 V), `BZX84C3V3` (3.3 V), `BZX84C5V1` (5.1 V), and `BZX84C12` (12 V).
- **`model bjt <name> npn|pnp`**: standard Gummel-Poon BJT parameters (`is`, `bf`, `nf`, `vaf`, `ikf`, `ise`, `ne`, `br`, `nr`, `var`, `ikr`, `isc`, `nc`, `rb`, `rc`, `re`, `cje`, `vje`, `mje`, `tf`, `cjc`, `vjc`, `mjc`, `tr`, `xtb`, `xti`, `eg`). Built-in BJTs: `2N3904`, `2N3906`, `2N2222`, `BC547` (`npn`), `BC557` (`pnp`), `KESSETSU_POWER_NPN_V1`, and `KESSETSU_POWER_PNP_V1`.
- **`model jfet <name> njf|pjf`**: junction field-effect transistor models (`njf` for N-channel, `pjf` for P-channel). Parameters include `vto` (pinch-off/threshold voltage), `beta` (transconductance parameter in A/V²), `lambda` (channel-length modulation in 1/V), `rd` (drain resistance in Ω), `rs` (source resistance in Ω), `cgs` (zero-bias G-S capacitance in F), `cgd` (zero-bias G-D capacitance in F), `pb` (gate junction potential in V), `is` (gate saturation current in A), `fc` (forward-bias depletion capacitance coefficient), `b` (doping tail parameter), `kf` (flicker noise coefficient), `af` (flicker noise exponent), and `tnom` (parameter measurement temperature in °C). Built-in models: `2N5457` (and `J2N5457`) `njf`, `J201` `njf`, and `2N5460` `pjf`. Built-in MOSFETs include `IRF540`, `2N7000` (`nmos`), `BSS84` (`pmos`), and `KESSETSU_PMOS_V1`.
- **`model switch <name>`**: voltage-controlled switch model mapping to SPICE `SW`. Parameters: `vt` (threshold voltage in V), `vh` (hysteresis voltage in V), `ron` (on-state resistance in Ω), and `roff` (off-state resistance in Ω). Default generic model: `KESSETSU_SW_V1` (`vt=0.5`, `vh=0`, `ron=1`, `roff=1Meg`).

Package imports require an exact version and produce a provenance-bearing `kessetsu.lock`. External declarations bind a user-owned source-relative file by exact SHA-256, entry name, canonical pin order, provenance, simulator mode, and redistribution policy. Ordinary reports/exports never embed its body. Native stdin cannot bind files; native file commands resolve contained local files. The [model cookbook](../guides/cookbook.md#choose-and-verify-a-component-model) shows the complete directory, hash and command workflow.

### Additional external interfaces

Use `external_subcircuit comparator Name (in_p,in_n,vcc,vee,out)` or
`external_subcircuit two_terminal Name (p1,p2)` with the same required metadata fields
shown above, then instantiate with `device U1 Name` or `device X1 Name`. The model
determines the catalog-backed interface; `device` cannot masquerade as an op-amp or
infer arbitrary pin lists. Header defaults/continuations are supported; positional
terminal count excludes `.SUBCKT` parameter defaults. By default, coefficients stay in the
exact file. A declaration may explicitly expose existing header parameters with
`instance_parameters="Rinit:Ohm,Vt:V"`; instances then use
`device XM Name (Rinit={initial_resistance}, Vt=1.6V)` (or the same parenthesized form
after an external `opamp`). Names are matched case-insensitively but emitted using the
declared spelling. Unknown names, duplicates, missing library defaults and unit mismatches
fail closed; the library body is never rewritten.

Development Web builds allow explicit local selection in View → Circuit details.
Only the portable self-contained Ngspice profile can simulate in the browser;
PSpice compatibility/unsupported constructs require native CLI. Files stay in memory,
not in drafts/shares/exports. See the [model catalog](model-catalog.md) for complete
examples, supported setups, licenses and dependency handling.

For models that require capacitor initial conditions, Kessetsu 1.2.0 accepts
`simulate tran <step> <stop> uic`. This skips the DC operating point and applies the
model's initial conditions; it is not the default. Ordinary transient forms and
legacy analysis serialization are unchanged.

## Compatibility rule

Unknown statements, components, pins, metrics, waveform arguments and analysis forms fail closed with source-located diagnostics. A future syntax addition must preserve every existing `examples/*.kess` program or explicitly introduce a new language/schema version.
