# Kessetsu Documentation

This page is the entry point for Kessetsu documentation. The repository keeps user guides, engineering contracts, architectural decisions, and dated verification evidence separate because they have different audiences and update rules.

## Start here

- [Project overview](../README.md): product vision, current capabilities, Web Hub, and local setup.
- [Tutorial](tutorial.md): build, verify, simulate, and export a first circuit.
- [Why Kessetsu?](why_kessetsu.md): product scope and measurable differences from adjacent tools.
- [Cookbook](cookbook.md): short recipes for common circuit-engineering tasks.
- [Troubleshooting](troubleshooting.md): common compiler, simulator, assertion, sharing, and export failures.

## User and integration reference

- [Language reference](language_reference.md): source syntax, components, connections, analyses, assertions, modules, and models.
- [CLI reference](cli_reference.md): commands, output schemas, exit codes, and agent-oriented usage.
- [Simulation and assertions](simulation_and_assertions.md): analysis execution and assertion states.
- [Engineering measurement contract](engineering_measurements.md): formulas, units, sign conventions, and failure behavior.
- [Supported domain](supported_domain.md): the explicit electrical and physical boundary of the first release.
- [Export formats](export_formats.md): visual, machine-readable, SPICE, and editable EDA outputs.

## Maintainer contracts

- [Architecture](architecture.md): normative pipeline, ownership boundaries, and invariants.
- [Roadmap](ROADMAP.md): ordered work, acceptance gates, and completion evidence.
- [Release contract](release.md): packaging, deployment, rollback, provenance, and telemetry policy.
- [Release security audit](security_audit.md): dependency, license, and runtime risk review.
- [Kessetsu identity migration](kessetsu_migration.md): completed rename contract plus its one remaining local-folder step.

Repository-level governance is documented in [AGENTS.md](../AGENTS.md), [CONTRIBUTING.md](../CONTRIBUTING.md), [SECURITY.md](../SECURITY.md), the [changelog](../CHANGELOG.md), and the licensing files at the repository root. Package-specific operational notes stay beside the package they govern, such as the [Web Hub README](../webapp/README.md), [Windows Ngspice runtime README](../core/tools/ngspice/README.md), and bundled [Web third-party notices](../webapp/public/THIRD_PARTY_NOTICES.md).

## Decisions and quality evidence

Architecture Decision Records explain decisions that should remain understandable after the implementation changes:

- [ADR 0001: Phase 4 schematic and Web architecture](decisions/0001-phase-4-schematic-and-web-architecture.md)
- [ADR 0002: browser simulation runtime](decisions/0002-browser-simulation-runtime.md)

The [schematic quality plan](schematic_quality_plan.md) defines the current visual acceptance method. Its dated evidence is retained as a sequence:

- [Rejected baseline — 2026-08-13](evals/schematic-quality-baseline-2026-08-13.md)
- [Rejected/superseded candidate — 2026-08-13](evals/schematic-quality-candidate-2026-08-13.md)
- [Accepted visual golden — candidate 2026-08-14, accepted 2026-08-20](evals/schematic-quality-candidate-2026-08-14.md)

Files under [`evals/`](evals/) are evidence, not general user documentation. Rejected candidates are intentionally retained so that a later layout regression cannot be mistaken for progress.

## Language policy

English is the primary language for source comments, UI text, diagnostics, and documentation. New documents and newly written sections must use English. Exact identifiers, commands, schema fields, and quoted external text should retain their original spelling.

Three long-lived documents still contain Turkish prose: `architecture.md`, `cli_reference.md`, and `ROADMAP.md`. They remain authoritative while translation is completed incrementally. A section should be translated as a coherent unit; avoid mixing languages sentence by sentence or changing technical meaning merely to finish the migration quickly.

## Document lifecycle

- User guides and references describe the current released behavior and must be updated with the implementation.
- Architecture and release contracts are normative. Conflicting code or prose must not be silently accepted; resolve the conflict explicitly.
- ADRs are append-only decision history. Supersede an old decision with a new ADR instead of rewriting the old context.
- Dated evals and audits are immutable evidence except for factual corrections clearly marked in the document.
- One-time migration plans remain until every external/manual step is complete, then move to an archive only if the active documentation index becomes noisy.
- Third-party notices and embedded-runtime READMEs stay beside the artifacts they govern and must not be consolidated away.

Before adding another Markdown file, first check whether the information belongs in an existing guide, reference, ADR, roadmap item, or dated eval. A new file should have one clear owner, audience, and lifecycle.
