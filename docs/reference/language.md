# Kessetsu Language Reference

This document describes Kessetsu 1.2.0. Kessetsu is line-oriented,
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
diode D1 1N4148
transistor Q1 npn 2N3904
mosfet M1 IRF540
opamp U1 KESSETSU_OPAMP_V1

source VDC 5V
source VIN sine_ac(0V,100mV,1kHz,1V)
current_source IBIAS 1mA
```

Source waveforms are typed: scalar DC, `sine(offset,amplitude,frequency)`, `pulse(low,high,delay,rise,fall,width,period)`, `pwl(time,value,...)`, `ac(amplitude)` and `sine_ac(offset,amplitude,frequency,ac_amplitude)`. PWL requires at least two time/value pairs with non-negative, strictly increasing times. See [supported domain](supported-domain.md) for component and model limits.

## Named quantities and arithmetic

```kessetsu
param supply: V = 12V
param resistance: Ohm = 10k
param target_gain: ratio = 20
source VCC {supply}
resistor RG {resistance}
resistor RF {(target_gain - 1) * resistance}
```

Parameters are optional, declared at the top level or inside modules, and explicitly typed: `Ohm`, `F`, `H`, `V`, `A`, `Hz`,
`s`, `W`, `ratio`, `percent` or `deg`. Forward references are allowed; duplicate/unknown
names and dependency cycles produce errors. `pi` is read-only. Expressions support finite
SI literals, names, parentheses, unary signs and `+ - * /`, with ordinary precedence.

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
contexts are not supported yet and fail explicitly. Kessetsu 1.2.0 accepts root CLI inputs
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
```

Canonical pins are `p1/p2` for two-terminal passives and diodes, `plus/minus` for sources, `c/b/e` for BJTs, `d/g/s` for MOSFETs and `in_p/in_n/vcc/vee/out` for op-amps. Every required pin must be connected. `GND` is the reference net; ambiguous or missing reference topology fails ERC.

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

All numeric arguments of unquoted `ac`, `sine`, `sine_ac`, `pulse` and `pwl` calls accept
expressions. Voltage/current levels use the source's unit, frequency uses Hz, and times
use seconds. PWL keeps its alternating time/value arguments and increasing-time checks.
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

## Typed models and packages

```kessetsu
model diode SafeD version=1.0.0 license=MIT Is=2e-9 Rs=0.5
model bjt SafeN npn version=1.0.0 license=MIT Is=1e-12 Bf=100
model_include kessetsu_analog 1.0.0
external_subcircuit opamp OPA197 (in_p,in_n,vcc,vee,out) file="models/OPAx197.LIB" entry=OPAx197 sha256=<64-hex-digest> version="Final 1.3" license="vendor terms" source="vendor URL" simulator=ngspice_ps redistribution=prohibited
```

Raw `.include`, `.model`, `.subckt` and `.control` injection is intentionally rejected. Model kinds have parameter allowlists; package imports require an exact version and produce a provenance-bearing `kessetsu.lock`. External declarations bind a user-owned source-relative file by exact SHA-256, entry name, canonical pin order, provenance, simulator mode, and redistribution policy. Ordinary reports/exports never embed its body. Native stdin cannot bind files; native file commands resolve contained local files. The [model cookbook](../guides/cookbook.md#choose-and-verify-a-component-model) shows the complete directory, hash and command workflow.

### Additional external interfaces

Use `external_subcircuit comparator Name (in_p,in_n,vcc,vee,out)` or
`external_subcircuit two_terminal Name (p1,p2)` with the same required metadata fields
shown above, then instantiate with `device U1 Name` or `device X1 Name`. The model
determines the catalog-backed interface; `device` cannot masquerade as an op-amp or
infer arbitrary pin lists. Header defaults/continuations are supported; positional
terminal count excludes `.SUBCKT` parameter defaults. Coefficients stay in the exact file.

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
