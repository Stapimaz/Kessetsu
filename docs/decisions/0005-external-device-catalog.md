# ADR 0005: Catalog-backed external devices and local browser bindings

- Status: accepted; implemented and verified in development source, not released/deployed
- Date: 2026-09-17
- Scope: Core IR/catalog, external model binding, browser simulation and exports

## Decision

Extend ADR 0003 rather than adding raw SPICE to the circuit language. An external-device
descriptor consists of a closed catalog family, its canonical pins/roles/coordinates/symbol,
and the existing exact entry/file/hash/provenance/simulator/redistribution metadata.
The initial additional families are `comparator` (in_p,in_n,vcc,vee,out) and `two_terminal`
(p1,p2). Declare the model with `external_subcircuit <family> ...` and instantiate it with
`device <id> <model>`. Existing `opamp` syntax and physical component IDs stay unchanged.
These are model interface families, not manufacturer package/footprint claims.

The same typed catalog supplies ERC pin checks, deterministic graph terminals, routing
anchors, signal/through roles, symbols and exporter order. A device's kind is resolved from
its bound model during AST-to-IR conversion; no backend reads declarations or guesses pins.
Circuit IR retains metadata only, never third-party model bodies. Default reports stay compact.
Two-terminal subcircuits instantiate with X, not an invented resistance value or diode model.
The comparator triangle is a five-pin comparator interface, not a generic op-amp substitution.

## Resource and capability boundaries

Native file commands retain source-directory containment, exact hashes and isolated staging.
Browser files require explicit user selection and reference binding; bytes remain local in
memory. Missing or changed bytes invalidate compilation and prior simulation/export state.
There is no automatic network model download, upload or compiled-plugin execution.

Browser execution accepts only a bounded self-contained Ngspice text-library profile:
subcircuit declarations/default parameters, local model/parameter/function definitions and
explicitly supported analog element cards. Reject control blocks, includes/library loading,
top-level analyses/output/options/end commands, compiled/digital/code models and unsupported
cards before simulator launch. The profile is a capability boundary, not a claim that every
accepted model converges or physically matches a part. Existing native-only Ngspice/PSpice
bindings remain available and must be identified as native-only before Web execution.
Do not replace model content, equations or default coefficients to make compatibility pass.
The first descriptor exposes no circuit-side model-coefficient overrides: defaults belong
to the exact model bytes. A future override needs declared dimensions/validation/provenance.

Only the ephemeral browser simulation request may contain locally selected text, after
Core identity/profile validation and IR-based binding. It must not leak into source shares,
model locks, schematic JSON, diagnostics or unrelated exports. Explicitly selected resource
bytes are never persisted implicitly. A source share explains that recipients must supply
matching local dependencies. An export retains dependency names/hashes and format losses;
prohibited text is never bundled. Browser export must recompile with the same bound bytes.

## Representation and formats

The unreleased compile-v5 contract gains typed device kinds/parameters. Existing model-v2
metadata can carry these additive kinds without dropping fields; new capability metadata,
if serialized, requires explicit schema review rather than silently widening old consumers.
Unknown kinds, fields, profiles and future schema versions fail closed.

Preserve known symbols. Two-terminal special devices use a visibly distinct catalog symbol
and pin labels rather than pretending to be a resistor. KiCad embeds canonical pins and
metadata. LTspice needs a matching editable native/custom symbol dependency; if unavailable,
report that format as unsupported for that device, never produce a deceptively wired file.
Completion of the full device milestone requires proper export support and stated losses.

## Acceptance gates

Before claiming implementation: native/reference electrical comparison for a real op-amp
and two non-op-amp families, plus native/browser comparison for portable families.
The exact PSpice manufacturer op-amp is intentionally native-only; refusing it before Web
execution is acceptance, not a missing generic-model substitution. Test portable external
op-amp binding separately without presenting a synthetic fixture as manufacturer evidence.
Also cover wrong hash/entry/pins, unresolved dependencies,
unsupported cards/modes and no-content leakage; unchanged literal examples/requirements;
local-selection/change/rebind/share/export behavior; rendered PNG inspection and EDA actual
pin checks for the new devices. A published resistive-switching model is preferred only
when its terms and supported setup are established. Do not relabel developer toy models
as manufacturer or research fidelity evidence.

SPICE X-instance and positional pin semantics follow the [Ngspice model documentation](https://ngspice.sourceforge.io/modelparams.html).
The [Ngspice memristor example](https://github.com/imr/ngspice/blob/master/examples/memristor/memristor.sp)
is characterized only in the bounded extracted-library setup documented by the
[model catalog](../reference/model-catalog.md). Its Modified BSD terms and exact adaptation
are retained in the accompanying example notice; this ADR itself does not grant model rights.
