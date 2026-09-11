# Evaluation Verification-Claim Audit v1

Status: audited against `unseen-design-v1.md` and evaluator source on 2026-09-11. This document defines how U1–U6 results may be described. It does not record agent trials or hardware validation.

## Result language

- `PASS` means every evaluator-owned Boolean requirement passed under the recorded simulator, models, topology boundary, sources, loads, analyses, and measurement windows.
- `FAIL` means the candidate was valid for that bounded evaluator but at least one electrical or independent-consistency requirement failed.
- `ERROR` means the candidate, model, simulator, or dataset could not be evaluated safely. An error is never a failed electrical design and never a pass.
- Every successful or failed evaluation includes `verification_scope` using schema `kessetsu.evaluation-verification-scope.v1`. It names the frozen task, check keys, nominal operating conditions, model assumptions, excluded effects, and explicitly sets `hardware_validated` to `false`.

## Per-task boundary

| Task | What a PASS establishes | Model/operating boundary | What it does not establish |
| --- | --- | --- | --- |
| U1 | Fixed loaded-RC topology, DC gain, relative cutoff, and agreement with an independent closed-form calculation | Ideal source, resistor, capacitor, and 100 kohm load; nominal 10 Hz–1 MHz AC sweep | Tolerance, temperature, parasitics, noise, transient response, or hardware behavior |
| U2 | Fixed sources/rails/load, supported RC-ladder topology, 100 Hz gain, relative cutoff, DC offset, settled peak, independent complex-response agreement, and AC/transient agreement | One/two-section input RC ladder and declared finite-gain single-pole linear op-amp template at ±6 V | Rail saturation, current/slew limits, supply current, noise, tolerance, temperature, a manufacturer part, or hardware behavior |
| U3 | Frozen topology/model integrity, collector bias/current, inverted 1 kHz gain, transient fundamental, DC KCL, and AC/transient agreement | Exact frozen generic 2N3904 model; divider bias and unbypassed emitter degeneration at 9 V | Manufacturer-lot variation, tolerance, temperature, noise, broader clipping margin, other amplifier topologies, or hardware behavior |
| U4 | Frozen source/load/model integrity and settled ON/OFF current, ON drain voltage, KCL, and dissipation inside the accepted windows | Exact frozen generic IRF540 VDMOS model; 12 V/120 ohm load; final four periods excluding 50 us around transitions | Total switching/gate-driver loss, tolerance, temperature, package/SOA/avalanche limits, EMC, or hardware behavior |
| U5 | Frozen 4 ohm/±9 V/input conditions and bounded amplifier topology; nominal output power, harmonic THD, and positive NPN/PNP dissipation over 10–30 ms | Exact generic linear op-amp and power-BJT models | Driver supply power, total efficiency, realistic rail/current/slew or crossover behavior, tolerance, thermal/package/SOA behavior, manufacturability, or hardware behavior |
| U6 | Exact manufacturer-model identity plus fixed topology/sources/load; nominal 1 kHz AC gain and settled transient amplitude/fundamental | User-supplied hash-verified TI OPAx197 Rev. D / Final 1.3 macromodel at ±6 V | Permission to redistribute the model, tolerance/board/thermal/EMC/production behavior, or hardware validation |

## Publication rules

Do not shorten these results to “the circuit is correct,” “production ready,” or “hardware verified.” Reports may say that a named task passed its frozen nominal-simulation requirements, followed by the applicable model and excluded-effect boundary. U5 total efficiency is unavailable by construction because the generic driver exposes no supply-current branches. U4 mean dissipation deliberately excludes switching transitions. U6 authentic-model interoperability is stronger evidence than the generic-model tasks, but it still remains macromodel simulation.

The evaluator source remains authoritative for calculations and thresholds. This audit and the emitted `verification_scope` must change together if a frozen task receives a new version.
