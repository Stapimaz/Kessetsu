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

When working in Antigravity with subagent and swarm tools available, operate under a **Lead (Argon High) + Worker (Hydrogen High)** model to keep architectural decisions centralized while delegating execution and research work to the high-throughput worker tier:

1. **Lead / Coordinator agent (`Gemini 4 Argon High`):** The primary conversation agent acts as the lead engineer and coordinator. It owns task planning, architecture, cross-cutting design decisions, complex debugging, multi-component integration, code review of worker output, and final sign-off. Conserve lead turns by delegating broad file exploration, implementation, test authoring, and verification runs to workers whenever practical.
2. **Worker agents (`Gemini 4 Hydrogen High`):**
   - **Native Swarm (`swarm` tool — preferred for multi-part, parallel, or iterative work):** Explicitly pass `model: "gemini-4-hydrogen-high"` on `swarm{action: "add", ...}` so every worker runs on **Gemini 4 Hydrogen (High)**. Use `workspace: "share"` when multiple authors edit or build concurrently, and add a `gemini-4-hydrogen-high` verifier worker when parallel changes benefit from independent review and test execution.
   - **Single Subagents (`invoke_subagent` tool — for single bounded background tasks):** Override the default `Model: "inherit"` parameter and pass `Model: "flash"` (`TypeName: "research"` for read-only codebase exploration; `TypeName: "self"` for focused implementation, tests, or documentation updates).
3. **Delegation guardrails:**
   - **No trivial delegation:** Do not spawn workers for tiny one-file edits, quick lookups, or trivial fixes that are faster to execute directly in a single turn.
   - **Parallelize independent work safely:** Assign strictly disjoint file scopes (or `workspace: "share"` in a swarm) so parallel workers never edit the same files concurrently or make competing architectural decisions.
   - **Mandatory lead review:** Always inspect and review worker diffs, test results, and outputs before treating any delegated subtask as complete.
   - **Lead owns final sign-off:** Keep final release/architecture decisions and canonical verification approval with the lead Argon agent.

## Work tracking

After each completed change, update the configured local execution plan and mark only proven checkboxes as `[x]`. Keep public guides and the changelog aligned with delivered behavior. Keep plans outside the repository; do not include them in commits, release archives, logs or public issue text.

Following these rules preserves Kessetsu's architecture and auditable development history.
