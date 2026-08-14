# ADR 0001 — Phase 4 Schematic and Web Architecture

- Status: Accepted
- Date: 2026-08-13
- Scope: Web shell, SVG renderer, Schematic IR, and layout

## Context

At the beginning of Phase 4, the Web application already provided real WASM compilation/ERC, an experimental SVG schematic, and KiCad/SPICE downloads. However, the entire UI and renderer lived in `App.tsx`, symbol geometries were duplicated in TypeScript, and layout output had the form `HashMap<component, position> + net_id/polyline`. That data contained no pin endpoints, junctions, unconnected crossings, net labels, version, or connectivity proof.

A six-circuit characterization corpus was locked by the `layout_characterization` test: minimal source/resistor, RC low-pass, Wheatstone bridge, op-amp gain stage, high fan-out, and a four-stage 8 Ω power amplifier. The legacy layout's coverage of every component and connected net by at least one polyline was preserved, but that alone did not prove equivalence between visual geometry and the canonical graph.

A real Chromium smoke test exposed two otherwise hidden integration failures: Web expected `kessetsu.compile.v1` while Core produced `v2`, and Monaco's CDN loading failed in offline/CSP environments. The compile version is now read from the WASM build, while Monaco and its worker are bundled with the application.

## Decision

| Layer | Decision | Rationale |
|---|---|---|
| Web shell | Refactor | Preserve working WASM compilation, editor, and basic panel behavior; split state, runtime, rendering, and export responsibilities into separate modules. |
| React SVG renderer | Replace | Separate TypeScript symbol geometry causes drift. Web will present only Core's versioned Schematic IR/SVG output and add an interaction layer. |
| Legacy `LayoutResult` data shape | Replace | It cannot be a public contract because it lacks typed endpoints, junctions, crossings, labels, ordering, and connectivity proof. `kessetsu.schematic.v1` replaces it. |
| Chain-layout heuristic | Refactor; replace if it fails the corpus | Simple rail/chain placement is a useful starting heuristic. It will be surrounded by deterministic layered placement, explicit net-label policy, and orthogonal routing; the old heuristic will not be retained if the quality report fails the corpus gate. |

Canonical ownership is as follows: Circuit IR owns electrical truth, and Schematic IR owns drawing/topology truth. The symbol/pin catalog belongs to Rust Core. Renderers and exporters consume Schematic IR and may not define their own pin lists, connectivity, or layout algorithms. Web state is never a canonical artifact.

## First vertical path and primary eval

- First vertical product path: `rc_filter.kess` → compile/ERC → schematic → AC simulation → cutoff/assertions → export.
- Primary vision/eval path: `power_amplifier.kess` → output power into 8 Ω, gain, THD, clipping, device stress, and dissipation.

These circuits are not alternatives: RC proves the integration contract on a small surface, while the power amplifier tests the product's real engineering value.

## Consequences

- The legacy layout field is compatibility data only during migration and is not used to add new backends.
- Visual quality is not accepted merely because it “looks good.” Connectivity, collision, crossing, bend, and determinism reports are automated gates.
- No export format may exist only in Web without a corresponding CLI/Core implementation.
