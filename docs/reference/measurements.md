# Engineering Measurement Contract

Kessetsu engineering measurements are evaluated from typed `kessetsu.simulation.v1` datasets. Current source uses `kessetsu.measurement.v2`; published 1.1.0 downloads and the deployed Web Hub still use v1. The new AC metrics below are source-only until the next release. Assertions never infer a passing value from missing data: an unavailable signal, incompatible analysis or invalid argument becomes an assertion `ERROR`.

## Primitives and sign convention

| Primitive | Meaning | Sign | Unit |
|---|---|---|---|
| `V(net)` | Node voltage relative to canonical GND | Positive at the named node | V |
| `V(device)` | Main terminal voltage | `p1→p2`, `plus→minus`, `c→e`, or `d→s` | V |
| `I(device)` | Branch/device current | Enters the first/main positive terminal; Ngspice source current therefore commonly appears negative while delivering power | A |
| `P(device)` | Instantaneous absorbed power | `V(device) × I(device)`; positive means absorption/dissipation, negative means delivery | W |

Component and net namespaces cannot collide, so `V(OUT)` is unambiguous. Independent voltage-source and inductor currents come from branch vectors. Resistor current is `V/R`, capacitor current is `C·dV/dt`, and diode/BJT/MOSFET current uses the simulator's typed device vector. Safe op-amp subcircuits do not expose an output branch current; requesting it produces `ERROR`.

Device-limit checks use the same primitives, for example:

```kessetsu
assert peak(V(Q1)) < 40V
assert peak(V(Q1.c,Q1.e),2ms,10ms) < 40V
assert peak(I(Q1)) < 1A
assert peak(P(Q1)) < 5W
assert dissipation(Q1) < 2W
```

These are simulated operating limits supplied by the design requirement. They are not automatically inferred datasheet ratings; datasheet knowledge belongs to the later Component Knowledge Base.

## Reductions

`value`, `min`, `max`, absolute `peak`, `average`/`avg`, and `rms` accept any real `V`, `I`, or `P` primitive. `value` is the final sample for series data and the scalar itself for OP. `peak` is `max(|x|)`, average is the arithmetic mean, and RMS is `sqrt(mean(x²))`.

Transient/DC reductions can be windowed with inclusive SI-valued time bounds:

```kessetsu
assert rms(V(OUT),2ms,10ms) > 3V
```

The contract requires `0 <= start < stop` and at least one sample inside the window. Complex AC data is intentionally not coerced into a real reduction; use `gain`, `bandwidth`/`cutoff`, or `phase`.

## Derived engineering metrics

| Metric | Formula / behavior | Required data | Unit |
|---|---|---|---|
| `gain(out,in)` | `RMS(out)/RMS(in)` when transient exists; otherwise complex magnitude ratio at the first AC point | Transient or AC | ratio |
| `gain_at(out,in,frequency)` | Complex magnitude ratio interpolated at the requested frequency; never uses transient RMS | AC | ratio |
| `lower_cutoff(out,in[,reference_frequency])` | Lower half-power edge of the band containing the reference | AC with an observed lower edge | Hz |
| `upper_cutoff(out,in[,reference_frequency])` | Upper half-power edge of the band containing the reference | AC with an observed upper edge | Hz |
| `bandwidth(out,in)` / `cutoff(out,in)` | Log-frequency interpolation of the first `-3 dB` crossing relative to the first AC gain point | AC, at least two points and a crossing | Hz |
| `frequency(signal)` | Reciprocal of the mean period between rising mean crossings | Transient, at least two crossings | Hz |
| `phase(out,in[,frequency])` | Wrapped complex phase difference in `(-180, 180]`; optional frequency selects the nearest AC point | AC | degree |
| `output_power(V(out),load[,start,stop])` | `Vrms²/Rload`; optional bounds isolate steady state | Transient voltage and resistor load | W |
| `efficiency(V(out),load,V(supply),I(source)[,...][,start,stop])` | `100 × Pout / Σ|average(Vsupply × Isource)|`; accepts one or two supply pairs and an optional shared steady-state window | Transient | % |
| `thd(signal,fundamental,start,stop,hann)` | Explicit-frequency Hann-windowed projection after deterministic linear resampling; `100 × sqrt(Σ H₂..H₅²)/H₁` | Transient data, at least 32 samples and two fundamental periods | % |
| `clipping(signal,lower,upper)` | Percentage of samples within `0.1%` of either explicit rail | Transient | % |
| `dissipation(device[,start,stop])` | Mean of `max(P(device), 0)`; optional bounds isolate steady state | Transient device voltage/current | W |

`V(component.pin,component.pin)` is the explicit terminal-pair primitive for stress checks. It validates both component pins against the shared catalog and preserves the written polarity; for example `V(Q1.c,Q1.e)` is VCE and `V(M1.g,M1.s)` is VGS. The shorthand `V(Q1)` remains the device's canonical main terminal pair for compatibility.

`bandwidth`/`cutoff` remains intentionally low-pass-only in v2, preserving v1 behavior: the first AC point must be within 1% of the maximum response and a later downward −3 dB crossing must exist. Band-pass, high-pass or multi-peak responses still fail closed for these legacy metrics. Existing `gain`, `phase`, and all other legacy metrics are unchanged.

### Frequency-specific AC gain and cutoff edges (source-only v2)

```kessetsu
simulate ac dec 80 1Hz 10MHz
assert gain_at(V(OUT),V(IN),1kHz) > 8.8
assert lower_cutoff(V(OUT),V(IN),1kHz) < 110Hz
assert upper_cutoff(V(OUT),V(IN),1kHz) > 8kHz
```

These three metrics accept node-voltage primitives `V(net)` only, not device voltages, terminal pairs, currents or powers. They use the first AC dataset, even when transient datasets or further AC analyses are present. Frequencies must be positive and inside that sweep; no extrapolation or nearest-point substitution occurs. Magnitude is interpolated linearly along log frequency, not linearly in dB.

Cutoff edges use `reference_gain / sqrt(2)` as the half-power threshold. An explicit reference frequency selects the connected above-threshold band containing that frequency. Without a reference, the sampled global peak supplies the reference; multiple disjoint above-threshold bands produce `ERROR` requesting an explicit reference rather than selecting a band silently. This is a response-based rule, not a filter-topology classifier.

Each edge is measured independently: a low-pass response can have an upper edge without an observed lower edge, and a high-pass response can have a lower edge without an observed upper edge. An absent edge returns `ERROR` asking to extend the sweep or choose another reference. Touching the threshold within a band does not split it; a sample exactly at threshold on the sweep boundary is an observed edge. Zero reference gain is invalid.

The complete selected response must have finite, positive, strictly increasing frequencies, aligned finite real/imaginary samples and nonzero input magnitude at every sample. Non-finite ratios and malformed data fail closed. The [loaded AC amplifier fixture](https://github.com/Stapimaz/Kessetsu/blob/main/core/tests/fixtures/benchmarks/ac_coupled_amplifier.kess) demonstrates the three metrics together. These assertions can also be used in evaluator-owned `.kessreq` files without changing the requirements contract.

Frequency sweeps should use `ac(amplitude)` sources. A source used by both transient and AC analyses uses `sine_ac(offset, amplitude, frequency, ac_amplitude)`.

## Failure behavior

Derived metrics validate argument count, physical units, target component type, dataset shape and analysis compatibility. Zero denominators, missing vectors, absent `-3 dB` crossings, insufficient samples and unsupported safe-subcircuit currents all return a message-bearing `ERROR`; they never become `0`, `NaN`, or an accidental `FAIL`.

The canonical real-simulator fixtures are:

- [`examples/rc_low_pass.kess`](../../examples/rc_low_pass.kess): cutoff and AC phase/response.
- [`examples/gain_stage.kess`](../../examples/gain_stage.kess): bias, closed-loop gain, bandwidth and clipping.
- [`examples/power_amplifier.kess`](../../examples/power_amplifier.kess): four stages, 8 Ω output power, gain, THD, clipping, dissipation, and device stress.
