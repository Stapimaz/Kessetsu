# Kessetsu Documentation

This page is the entry point for Kessetsu documentation. Files are grouped by audience so the root stays limited to the index and the two project-wide contracts.

```text
docs/
├── README.md          documentation index
├── architecture.md   normative architecture
├── ROADMAP.md         status and ordered work
├── guides/            task-oriented user documentation
├── reference/         language, CLI, simulation, and export contracts
├── maintainers/       release and security operations
├── decisions/         architecture decision records
├── evals/             reproducible plans and dated evidence
└── assets/            current public screenshots
```

## Start here

- [Project overview](../README.md): product vision, current capabilities, Web Hub, and local setup.
- [Tutorial](guides/tutorial.md): build, verify, simulate, and export a first circuit.
- [Why Kessetsu?](guides/why-kessetsu.md): product scope and measurable differences from adjacent tools.
- [Cookbook](guides/cookbook.md): short recipes for common circuit-engineering tasks.
- [Troubleshooting](guides/troubleshooting.md): common compiler, simulator, assertion, sharing, and export failures.

## User and integration reference

- [Language reference](reference/language.md): source syntax, components, connections, analyses, assertions, modules, and models.
- [CLI reference](reference/cli.md): commands, output schemas, exit codes, and agent-oriented usage.
- [Simulation and assertions](reference/simulation-and-assertions.md): analysis execution and assertion states.
- [Engineering measurement contract](reference/measurements.md): formulas, units, sign conventions, and failure behavior.
- [Supported domain](reference/supported-domain.md): the explicit electrical and physical boundary of the first release.
- [Export formats](reference/exports.md): visual, machine-readable, SPICE, and editable EDA outputs.

## Maintainer contracts

- [Architecture](architecture.md): normative pipeline, ownership boundaries, and invariants.
- [Roadmap](ROADMAP.md): ordered work, acceptance gates, and completion evidence.
- [Release contract](maintainers/release.md): packaging, deployment, rollback, provenance, and telemetry policy.
- [Release security audit](maintainers/security-audit.md): dependency, license, and runtime risk review.

Repository-level governance is documented in [AGENTS.md](../AGENTS.md), [CONTRIBUTING.md](../CONTRIBUTING.md), [SECURITY.md](../SECURITY.md), the [changelog](../CHANGELOG.md), and the licensing files at the repository root. Package-specific operational notes stay beside the package they govern, such as the [Web Hub README](../webapp/README.md), [Windows Ngspice runtime README](../core/tools/ngspice/README.md), and bundled [Web third-party notices](../webapp/public/THIRD_PARTY_NOTICES.md).

## Decisions and quality evidence

Architecture Decision Records explain decisions that should remain understandable after the implementation changes:

- [ADR 0001: Phase 4 schematic and Web architecture](decisions/0001-phase-4-schematic-and-web-architecture.md)
- [ADR 0002: browser simulation runtime](decisions/0002-browser-simulation-runtime.md)
- [ADR 0003: external subcircuit references](decisions/0003-external-subcircuit-references.md)
- [ADR 0004: evaluator-owned requirement sets](decisions/0004-evaluator-owned-requirements.md)

The [schematic quality plan](evals/schematic-quality-plan.md) defines the current visual acceptance method. Its dated evidence is retained as a sequence:

- [Rejected baseline — 2026-08-13](evals/schematic-quality-baseline-2026-08-13.md)
- [Rejected/superseded candidate — 2026-08-13](evals/schematic-quality-candidate-2026-08-13.md)
- [Accepted visual golden — candidate 2026-08-14, accepted 2026-08-20](evals/schematic-quality-candidate-2026-08-14.md)

The unseen-design gate has its own frozen protocol and claim boundaries:

- [Unseen design evaluation v1](evals/unseen-design-v1.md)
- [Agent comparison harness v1](evals/agent-comparison-harness-v1.md)
- [Evaluation verification-claim audit v1](evals/verification-claim-audit-v1.md)
- [OPA197 manufacturer-model probe](evals/opa197-model-probe-2026-09-11.md)
- [Six-task unseen-design decision summary](evals/unseen-design-summary-2026-09-13.md)
- Per-task comparisons: [U1](evals/unseen-design-u1-comparison-2026-09-12.md), [U2](evals/unseen-design-u2-comparison-2026-09-12.md), [U3](evals/unseen-design-u3-comparison-2026-09-12.md), [U4](evals/unseen-design-u4-comparison-2026-09-13.md), [U5](evals/unseen-design-u5-comparison-2026-09-13.md), and [U6](evals/unseen-design-u6-comparison-2026-09-13.md)
- [U6 external-model follow-up specification](evals/unseen-design-u6-followup-v1.md) and [result](evals/unseen-design-u6-followup-2026-09-14.md)

Files under [`evals/`](evals/) are evidence, not general user documentation. Rejected candidates are intentionally retained so that a later layout regression cannot be mistaken for progress.

## Language policy

English is the primary language for source comments, UI text, diagnostics, and documentation. New documents and newly written sections must use English. Exact identifiers, commands, schema fields, and quoted external text should retain their original spelling.

The authoritative architecture, CLI reference, and roadmap were migrated to English on 2026-08-20. If older Turkish prose is found elsewhere, translate the containing section as a coherent unit; avoid mixing languages sentence by sentence or changing technical meaning merely to finish a translation.

## Document lifecycle

- User guides and references describe the current released behavior and must be updated with the implementation.
- Architecture and release contracts are normative. Conflicting code or prose must not be silently accepted; resolve the conflict explicitly.
- ADRs are append-only decision history. Supersede an old decision with a new ADR instead of rewriting the old context.
- Dated evals and audits are immutable evidence except for factual corrections clearly marked in the document.
- Completed one-time migration plans are removed from the active tree; Git history remains the audit trail.
- Third-party notices and embedded-runtime READMEs stay beside the artifacts they govern and must not be consolidated away.

Before adding another Markdown file, first check whether the information belongs in an existing guide, reference, ADR, roadmap item, or dated eval. A new file should have one clear owner, audience, and lifecycle.
