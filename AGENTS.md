# Kessetsu AI Agent Rules

You are an AI agent assigned to Kessetsu, or continuing an earlier session. Kessetsu is a circuit-engineering platform with SPICE-based simulation and automatic schematic layout.

## Required first steps

1. **Read the constitution first:** read `docs/architecture.md` completely and retain the project context.
2. **Read the roadmap next:** inspect `docs/ROADMAP.md` for the current development phase and ordered tasks.
3. **Do not skip phases.** A later phase must not begin until every task and acceptance gate in the preceding phase is complete.

## Non-negotiable rules

1. **SPICE node naming:** keep the node-naming algorithm in `graph.rs` deterministic. User-named nets take precedence over automatic names according to the canonical rules; ambiguities produce diagnostics.
2. **Orientation and routing:** the layout engine in `layout.rs` is orientation-aware. When adding a component, follow the shared component catalog in `core/src/component.rs`, including pin coordinates and signal/through metadata.
3. **IR is the single source of truth:** every backend (SPICE, layout, ERC, JSON) consumes Circuit IR from `ir.rs`. Never generate backend output directly from the AST.
4. **ERC, not DRC:** schematic-level checks are called **ERC** (Electrical Rules Check) and belong in `erc.rs`.
5. **Proportionate verification:** select checks according to the changed behavior and risk. Documentation-only or deployment-setting changes need focused validation, not the entire test suite or a wait for remote CI. Run `powershell -ExecutionPolicy Bypass -File scripts/verify.ps1` from the repository root for substantial cross-cutting changes and release candidates; reuse green evidence when the verified source is unchanged. Use `-SkipNpmInstall` only when appropriate during iteration; a required final canonical run uses the full command. Record exactly what was checked, without implying unrun tests passed.
6. **Backward compatibility:** new syntax must not break existing `examples/*.kess` files.

## Work tracking

After each completed change, mark the corresponding checkbox in `docs/ROADMAP.md` as `[x]`.

Following these rules preserves Kessetsu's architecture and auditable development history.
