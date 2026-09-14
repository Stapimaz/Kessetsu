# Kessetsu Language Reference

This document describes the source language accepted by the first public release. Kessetsu is line-oriented, case-sensitive except for documented device polarities, and uses `//` comments. Backends never consume syntax directly: source is parsed, flattened and validated into typed Circuit IR first.

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
mosfet M1 nmos IRF540
opamp U1 KESSETSU_OPAMP_V1

source VDC 5V
source VIN sine_ac(0V,100mV,1kHz,1V)
current_source IBIAS 1mA
```

Source waveforms are typed: scalar DC, `sine(offset,amplitude,frequency)`, `pulse(low,high,delay,rise,fall,width,period)`, `ac(amplitude)` and `sine_ac(offset,amplitude,frequency,ac_amplitude)`. See [supported domain](supported_domain.md) for component and model limits.

## Connections and pins

```kessetsu
connect VIN.plus to IN
connect VIN.minus, C1.p2 to GND
connect R1.p1 to IN
connect R1.p2, C1.p1 to OUT
```

Canonical pins are `p1/p2` for two-terminal passives and diodes, `plus/minus` for sources, `c/b/e` for BJTs, `d/g/s` for MOSFETs and `in_p/in_n/vcc/vee/out` for op-amps. Every required pin must be connected. `GND` is the reference net; ambiguous or missing reference topology fails ERC.

## Analyses and assertions

```kessetsu
simulate op
simulate tran 10us 5ms
simulate ac dec 30 10Hz 10MHz
simulate dc VIN -1V 1V 10mV

assert gain(V(OUT),V(IN)) > 9.9
assert rms(V(OUT),2ms,5ms) < 6V
assert peak(V(Q1.c,Q1.e)) < 40V
```

Analysis arguments and assertions are dimension checked. Missing signals or incompatible datasets become explicit errors, never implicit zeroes. Full formulas and sign conventions are in [engineering measurements](engineering_measurements.md); execution behavior is in [simulation and assertions](simulation_and_assertions.md).

## Modules

Modules provide reusable topology. A module instance is flattened before semantic analysis; backend-specific module shortcuts do not exist.

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

Raw `.include`, `.model`, `.subckt` and `.control` injection is intentionally rejected. Model kinds have parameter allowlists; package imports require an exact version and produce a provenance-bearing `kessetsu.lock`. External op-amp declarations bind a user-owned source-relative file by exact SHA-256, entry name, canonical pin order, provenance, simulator mode, and redistribution policy. Kessetsu validates but does not embed or redistribute that file. The native CLI supports the binding; stdin and the current browser runtime report it as unavailable without fallback. The [model cookbook](cookbook.md#choose-and-verify-a-component-model) shows the complete directory, hash and command workflow.

## Compatibility rule

Unknown statements, components, pins, metrics, waveform arguments and analysis forms fail closed with source-located diagnostics. A future syntax addition must preserve every existing `examples/*.kess` program or explicitly introduce a new language/schema version.
