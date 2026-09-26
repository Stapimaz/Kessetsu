# External model catalog

Local external-model workflows require Kessetsu 1.2.0; typed instance parameters require
Kessetsu 1.3.0. This is a small characterized catalog, not a universal model
marketplace. Model identity, simulation evidence and physical-device accuracy are distinct.

## Supported interfaces

| Declaration family | Instance keyword | Canonical pins, in positional SPICE order | Presentation |
|---|---|---|---|
| `opamp` | `opamp` | `in_p,in_n,vcc,vee,out` | Existing op-amp triangle |
| `comparator` | `device` | `in_p,in_n,vcc,vee,out` | Comparator triangle; no replacement op-amp model |
| `two_terminal` | `device` | `p1,p2` | Generic two-terminal box, not an R instance |

The family describes the electrical interface, not a package, footprint or device rating.
Only these catalog-backed interfaces are supported. Arbitrary pin/geometry descriptors are
not implemented. A declaration may expose up to 32 existing `.SUBCKT` header parameters
through a unit-typed allowlist; all other coefficients remain fixed by the exact file/version/hash.

## Local-file workflow

Start with [external_comparator.kess](../../examples/external_comparator.kess) or
[external_memristor.kess](../../examples/external_memristor.kess).

The memristor example exposes only `Rinit`, `Vt` and `stime` from its exact library header.
Its instance binds the first two through ordinary root parameters, so `--param`, parameter
studies and finite calibration workflows all use the same typed input path. This changes
an instance value, not the hash-bound model file.

On the CLI, keep the `models/` directory beside the source:

```console
kess test examples/external_comparator.kess
kess test examples/external_memristor.kess
kess export examples/external_memristor.kess --target ltspice --output examples/memristor.asc
```

On Web, open File → Examples → Local model files, then View → Circuit details. Download
the example's model file and select it for its declared resource. No account or upload
is involved. Compilation checks its exact hash, entry and positional terminal count;
simulation checks the browser profile before loading Ngspice. A selected file's original
filename need not match the reference: the selection explicitly binds it to that reference.

Files remain in memory only. Editing source retains bindings but revalidates its metadata;
opening a different circuit/new/example clears them. Reloading, browser draft recovery
and shared-source links require selecting them again. Changing/clearing a binding discards
prior simulation and drawing/export state. Renaming a circuit does not alter model identity.

Native files must be source-relative and contained under the source directory; traversal,
absolute paths and unsafe references are rejected. Web bindings use the same references
without reading the filesystem implicitly. Limits: 16 MiB per UTF-8 file, 64 bound browser
resources, 32 MiB combined browser bytes, 32 KiB per logical library statement and bounded
statement/nesting counts. These are resource ceilings, not a hard aggregate memory guarantee.

## Characterized models

### Kessetsu generic comparator v1

- File: [comparator.lib](../../examples/models/comparator.lib), AGPL-3.0-only.
- Entry: `KESSETSU_COMPARATOR_V1`, five terminals.
- SHA-256: `00198a3b00ef90c78b93beb6c7e7ed61c9f0220dc597a6c71cd0661343246c34`.
- Model: smooth 1 mV input transition, 0.1 V output rail headroom, 50 Ω output resistance.
- Covered setup: 5 V single supply, 100 mV-peak / 1 kHz input around a grounded reference,
  10 kΩ load, transient at 1 µs output step for 2 ms. Both polarities of the input
  and loaded output high/low are checked against fixed independent limits.
- Intended for circuit/workflow verification, not a named manufacturer's comparator.
  No delay, hysteresis, offset, noise, input-current, common-mode/rating or thermal fidelity
  is claimed. AC around the transition is not an endorsement of a real comparator's response.
- Native Ngspice and the browser portable profile; expected output low approximately
  99.5 mV and high approximately 4.876 V for this load.

### Published threshold memristor demonstration

- File: [memristor.lib](../../examples/models/memristor.lib), BSD-3-Clause; retain
  [attribution, license and extraction details](../../examples/models/MEMRISTOR_NOTICE.md).
- Entry: `memristor`, two terminals. Adapted from the Ngspice example based on
  [Pershin and Di Ventra's threshold model](https://arxiv.org/abs/1204.2600).
- SHA-256: `cd4bac38581fb00c1b5667e9d5ec440cf954efb5e26105ae2775853cc90301f6`.
- Covered setup: zero-offset 3 V-peak / 100 MHz sine, 100 ps output step for 10 ns,
  `uic`, original demonstration defaults: 1 kΩ/10 kΩ bounds, 7 kΩ initial state,
  1.6 V threshold, `alpha=0`, `beta=20e3/stime`, `stime=10n`.
- The initial-state and waveform histories matter. Use `simulate tran 100ps 10ns uic`:
  this skips the DC operating point and applies capacitor initial conditions. Do not
  add `uic` to ordinary circuits by habit. OP/AC and arbitrary pulse/frequency/default
  combinations are not characterized here. Direct reference comparison uses the same
  exact library/defaults and initial-condition policy.
- These coefficients are artificial demonstrations, not calibration to a fabricated
  device. A hysteresis-shaped plot is not proof of physical fidelity or a research pilot.
- The [reproducible pulse protocol](../guides/memristor-protocol.md) covers a 24-case
  amplitude/duration/starting-state matrix, 25 ps versus 12.5 ps timestep refinement and
  one separately reported held-out stimulus. It characterizes this exact synthetic model;
  it does not validate a fabricated device.
- Measure terminal current with the explicit series zero-volt `VSENSE` source; arbitrary
  subcircuit current/power is not guessed from an internal branch or a resistance value.

### Texas Instruments OPA197 / OPAx197 model

- Existing native workflow, [official manufacturer download](https://www.ti.com/lit/zip/SBOMA34),
  model Final 1.3, 2022-06-23, OPAx197 PSpice Rev. D. File `OPAx197.LIB`, entry `OPAx197`.
- SHA-256: `fc5b020e63346e511bd808bf41c856b0150b000bcf8a41fe00eeececb1f422a5`.
- Interface: `opamp (in_p,in_n,vcc,vee,out)` maps exact positional header
  `IN+ IN- VCC VEE OUT`. Declare `simulator=ngspice_ps redistribution=prohibited`.
- User-acquired local evaluation under TI terms; Kessetsu does not redistribute,
  rewrite, bundle into downloads or silently substitute this manufacturer's file.
- Characterized native OP/AC/transient non-inverting amplifier under the independent
  manufacturer-model evaluator. This evidence does not validate all data-sheet behavior.
- **Native-only:** the browser profile does not enable PSpice compatibility or its
  compiled/code-model dependencies. Selecting it can resolve compilation/geometry;
  browser simulation explicitly refuses the compatibility mode before execution.

## Capability and interchange boundaries

Browser libraries are self-contained Ngspice analog text: `.subckt`/`.ends`, header defaults,
local `.param`/`.func`, and local D/BJT/MOS/switch `.model` types; analog R/C/L/V/I/D/B,
E/G/F/H/Q/M/S and X cards. Subcircuit calls must resolve in the selected library and
instance-level SPICE parameter overrides are not in this initial profile. Conservatively
rejected tokens/directives may require the CLI even if another simulator accepts them.
Controls, includes, `.lib`, file-loading expressions, top-level devices, analysis/output/options,
digital/compiled/plugin cards and unsupported model kinds do not execute on Web. Accepted
syntax can still fail solver convergence; such failures are not rewritten to PASS.

All external libraries, including native ones, reject control/include/library/plugin-loading
directives and require complete, bounded subcircuit headers/closures. Native Ngspice/PSpice
compatibility is broader than the browser profile, not a portability guarantee. OSDI or
user-compiled plugin loading needs a separately designed opt-in workflow.

Schematics, manifests, locks and ordinary exports carry dependency identity, never external
model bodies. Only the ephemeral browser simulation deck receives validated local text.
SPICE/LTspice exports need matching files beside the export at the declared relative paths.
KiCad preserves symbols, pins and model metadata, but is not a complete simulator project.
Two-terminal LTspice instances use the native rectangular outline with explicit `Prefix X`;
they are subcircuits, not resistor primitives. Ngspice model compatibility is not proof of
LTspice simulation compatibility. Read export warnings/losses and keep the original `.kess`.

See [external model declarations](language.md), [exports](exports.md) and
[simulation/assertion contracts](simulation-and-assertions.md).
