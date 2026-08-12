# Engineering Measurement Contract

NetLang engineering measurements are evaluated from typed `netlang.simulation.v1` datasets by the versioned `netlang.measurement.v1` model. Assertions never infer a passing value from missing data: an unavailable signal, incompatible analysis or invalid argument becomes an assertion `ERROR`.

## Primitives and sign convention

| Primitive | Meaning | Sign | Unit |
|---|---|---|---|
| `V(net)` | Node voltage relative to canonical GND | Positive at the named node | V |
| `V(device)` | Main terminal voltage | `p1→p2`, `plus→minus`, `c→e`, or `d→s` | V |
| `I(device)` | Branch/device current | Enters the first/main positive terminal; Ngspice source current therefore commonly appears negative while delivering power | A |
| `P(device)` | Instantaneous absorbed power | `V(device) × I(device)`; positive means absorption/dissipation, negative means delivery | W |

Component and net namespaces cannot collide, so `V(OUT)` is unambiguous. Independent voltage-source and inductor currents come from branch vectors. Resistor current is `V/R`, capacitor current is `C·dV/dt`, and diode/BJT/MOSFET current uses the simulator's typed device vector. Safe op-amp subcircuits do not expose an output branch current; requesting it produces `ERROR`.

Device-limit checks use the same primitives, for example:

```netlang
assert peak(V(Q1)) < 40V
assert peak(I(Q1)) < 1A
assert peak(P(Q1)) < 5W
assert dissipation(Q1) < 2W
```

These are simulated operating limits supplied by the design requirement. They are not automatically inferred datasheet ratings; datasheet knowledge belongs to the later Component Knowledge Base.

## Reductions

`value`, `min`, `max`, absolute `peak`, `average`/`avg`, and `rms` accept any real `V`, `I`, or `P` primitive. `value` is the final sample for series data and the scalar itself for OP. `peak` is `max(|x|)`, average is the arithmetic mean, and RMS is `sqrt(mean(x²))`.

Transient/DC reductions can be windowed with inclusive SI-valued time bounds:

```netlang
assert rms(V(OUT),2ms,10ms) > 3V
```

The contract requires `0 <= start < stop` and at least one sample inside the window. Complex AC data is intentionally not coerced into a real reduction; use `gain`, `bandwidth`/`cutoff`, or `phase`.

## Derived engineering metrics

| Metric | Formula / behavior | Required data | Unit |
|---|---|---|---|
| `gain(out,in)` | `RMS(out)/RMS(in)` when transient exists; otherwise complex magnitude ratio at the first AC point | Transient or AC | ratio |
| `bandwidth(out,in)` / `cutoff(out,in)` | Log-frequency interpolation of the first `-3 dB` crossing relative to the first AC gain point | AC, at least two points and a crossing | Hz |
| `frequency(signal)` | Reciprocal of the mean period between rising mean crossings | Transient, at least two crossings | Hz |
| `phase(out,in[,frequency])` | Wrapped complex phase difference in `(-180, 180]`; optional frequency selects the nearest AC point | AC | degree |
| `output_power(V(out),load)` | `Vrms²/Rload` | Transient voltage and resistor load | W |
| `efficiency(V(out),load,V(supply),I(source)[,...])` | `100 × Pout / Σ|average(Vsupply × Isource)|`; accepts one or two supply pairs | Transient | % |
| `thd(signal)` | `100 × sqrt(Σ H₂..H₅²)/H₁`; the largest non-DC DFT bin is the fundamental | Transient, at least 16 samples | % |
| `clipping(signal,lower,upper)` | Percentage of samples within `0.1%` of either explicit rail | Transient | % |
| `dissipation(device)` | Mean of `max(P(device), 0)` | Transient device voltage/current | W |

Frequency sweeps should use `ac(amplitude)` sources. A source used by both transient and AC analyses uses `sine_ac(offset, amplitude, frequency, ac_amplitude)`.

## Failure behavior

Derived metrics validate argument count, physical units, target component type, dataset shape and analysis compatibility. Zero denominators, missing vectors, absent `-3 dB` crossings, insufficient samples and unsupported safe-subcircuit currents all return a message-bearing `ERROR`; they never become `0`, `NaN`, or an accidental `FAIL`.

The canonical real-simulator fixtures are:

- `core/tests/fixtures/benchmarks/rc_filter.nl`: cutoff and AC phase/response.
- `core/tests/fixtures/benchmarks/gain_stage.nl`: bias, closed-loop gain, bandwidth and clipping.
- `core/tests/fixtures/benchmarks/power_amplifier.nl`: four stages, 8 Ω output power, gain, THD, clipping, dissipation, and device stress.
