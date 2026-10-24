# Kessetsu AI Agent Rules

You are an AI agent assigned to Kessetsu, or continuing an earlier session. Kessetsu is a circuit-engineering platform with SPICE-based simulation and automatic schematic layout.

## Required first steps

1. **Read the constitution first:** read `docs/architecture.md` completely and retain the project context.
2. **Read product documentation next:** read `docs/README.md` and `CHANGELOG.md` for public behavior and version status. If `git config --local --get kessetsu.privatePlan` returns a path, read that local plan for the current development phase and ordered tasks. Never publish that path or its contents. If no local plan is configured, discuss task scope through the contributor process; do not invent internal priorities.
3. **Do not skip phases.** A later phase must not begin until every task and acceptance gate in the preceding phase is complete.

## Non-negotiable rules

1. **SPICE node naming:** keep the node-naming algorithm in `graph.rs` deterministic. User-named nets take precedence over automatic names according to the canonical rules; ambiguities produce diagnostics.
2. **Orientation and routing:** the layout engine in `layout.rs` is orientation-aware. When adding a component, follow the shared component catalog in `core/src/component.rs`, including pin coordinates and signal/through metadata.
3. **IR is the single source of truth:** every backend (SPICE, layout, ERC, JSON) consumes Circuit IR from `ir.rs`. Never generate backend output directly from the AST.
4. **ERC, not DRC:** schematic-level checks are called **ERC** (Electrical Rules Check) and belong in `erc.rs`.
5. **Proportionate verification:** select checks according to the changed behavior and risk. Documentation-only or deployment-setting changes need focused validation, not the entire test suite or a wait for remote CI. Run `powershell -ExecutionPolicy Bypass -File scripts/verify.ps1` from the repository root for substantial cross-cutting changes and release candidates; reuse green evidence when the verified source is unchanged. Use `-SkipNpmInstall` only when appropriate during iteration; a required final canonical run uses the full command. Record exactly what was checked, without implying unrun tests passed.
6. **Backward compatibility:** new syntax must not break existing `examples/*.kess` files.

## Multi-agent delegation workflow

When working in Antigravity with subagent tools available, operate under a **Lead + Flash Worker** model to keep architectural decisions centralized while delegating bounded execution work:

1. **Lead agent (Gemini 4 Argon High):** The primary conversation agent acts as the lead engineer and owns task planning, architecture, cross-cutting design decisions, complex debugging, multi-component integration, code review of subagent output, and final verification.
2. **Worker subagents (`Model: "flash"` — use the `flash` worker tier, currently intended as Gemini 3.8 Flash High):** Override the default `Model: "inherit"` parameter on `invoke_subagent` and explicitly pass `Model: "flash"` when delegating suitable worker tasks:
   - Use `TypeName: "research"` with `Model: "flash"` for broad codebase exploration, multi-file searches, and read-only analysis.
   - Use `TypeName: "self"` (or a session worker defined via `define_subagent` with `EnableWriteTools: true`) with `Model: "flash"` for focused implementation tasks, unit/integration test authoring, documentation updates, repetitive refactoring, and independent subtasks.
3. **Delegation guardrails:**
   - **No trivial delegation:** Do not spawn subagents for tiny one-file edits, quick lookups, or trivial fixes that are faster to execute directly.
   - **Parallelize independent work safely:** For larger tasks, split independent work across multiple `Model: "flash"` subagents when useful, assigning strictly disjoint file scopes so parallel subagents never edit the same files or make competing architectural decisions.
   - **Mandatory lead review:** Always inspect and review subagent diffs, test results, and outputs directly in the lead agent before treating any delegated subtask as complete.
   - **Lead owns final sign-off:** Keep final verification (`scripts/verify.ps1`, focused test suites, release/doc audits) and completion decisions with the lead Argon agent.

## Work tracking

After each completed change, update the configured local execution plan and mark only proven checkboxes as `[x]`. Keep public guides and the changelog aligned with delivered behavior. Keep plans outside the repository; do not include them in commits, release archives, logs or public issue text.

Following these rules preserves Kessetsu's architecture and auditable development history.
