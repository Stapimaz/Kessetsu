# Reproducible Memristor Pulse Protocol

Kessetsu includes a complete, synthetic experiment for the bundled Pershin–Di Ventra
threshold-model example. It varies write-pulse amplitude, duration and starting state,
records switching direction and energy, checks timestep sensitivity, and evaluates one
separate held-out stimulus.

This is a software and model-characterization example. The coefficients are artificial,
the model and protocol are not fitted to a fabricated device, and simulation agreement is
not evidence for a physical switching mechanism.

## Run the protocol

Keep `examples/models/` beside the specification files, then run:

```sh
kess study run examples/memristor_pulse_protocol.kessstudy.json \
  --output memristor-results.json
kess study run examples/memristor_convergence.kessstudy.json \
  --output memristor-convergence.json
kess study run examples/memristor_holdout.kessstudy.json \
  --output memristor-holdout.json
```

The main matrix contains 24 cases:

- write amplitude: -3 V, -1 V, 1 V and 3 V;
- write duration: 1 ns, 3 ns and 6 ns;
- initial modeled resistance: 3 kΩ and 7 kΩ.

Each waveform contains a 0.4 V sub-threshold read window before and after one signed write
pulse. A series zero-volt source measures terminal current. The declared study measurements
retain total protocol energy and peak current; the notebook additionally derives effective
read resistance as `V(TOP) / I(VSENSE)` inside the two read windows.

## What the bounded experiment establishes

For the exact hash-bound model and Ngspice setup in the example:

- ±1 V stays below the declared 1.6 V threshold and produces negligible state change;
- -3 V moves the modeled resistance toward its low-resistance boundary;
- +3 V moves it toward its high-resistance boundary;
- longer above-threshold pulses do not reduce the magnitude of modeled state movement;
- all recorded protocol energies are finite and positive.

The nominal model bounds are 1 kΩ and 10 kΩ. Numerical integration can place the inferred
read resistance slightly beyond those values, so the executable regression uses a documented
950 Ω–10.5 kΩ numerical envelope rather than presenting the limits as exact clamps.

## Numerical sensitivity

The convergence specification repeats 16 boundary-relevant cases at 25 ps and 12.5 ps.
In the currently characterized run, the maximum relative difference was approximately
1.39% for final read resistance and 1.14% for protocol energy. The regression gate is 2%
for both values. These differences are reported as numerical sensitivity, not hidden as
measurement noise.

The held-out case uses 2.4 V, 4 ns and a 5 kΩ starting state. It is intentionally absent
from the main amplitude/duration grid and is reported separately. It checks that the chosen
protocol still executes and switches in the expected direction; it does not constitute
independent laboratory validation because it uses the same model and simulator.

## Inspect the complete evidence

[`memristor-pulse-protocol.ipynb`](../../examples/notebooks/memristor-pulse-protocol.ipynb)
runs all three specifications headlessly, builds tables from the versioned result artifacts,
plots state change and energy with units, and reports the timestep and holdout results.

The exact dependency is [`memristor.lib`](../../examples/models/memristor.lib), SHA-256
`cd4bac38581fb00c1b5667e9d5ec440cf954efb5e26105ae2775853cc90301f6`, BSD-3-Clause.
See the [model catalog](../reference/model-catalog.md) for provenance and supported limits.
