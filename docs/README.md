# Kessetsu Documentation

English is the primary documentation language. These documents describe public behavior,
implementation contracts and reproducible engineering evidence.

## Getting started

- [Project overview](../README.md)
- [Tutorial](guides/tutorial.md)
- [Why Kessetsu?](guides/why-kessetsu.md)
- [Cookbook](guides/cookbook.md)
- [Troubleshooting](guides/troubleshooting.md)

## Reference

- [Language](reference/language.md)
- [CLI](reference/cli.md)
- [Simulation and assertions](reference/simulation-and-assertions.md)
- [Engineering measurements](reference/measurements.md)
- [Supported domain](reference/supported-domain.md)
- [Export formats](reference/exports.md)

## Project and contributor documentation

- [Public roadmap](ROADMAP.md) and [changelog](../CHANGELOG.md)
- [Architecture](architecture.md)
- [Release and distribution contract](maintainers/release.md)
- [Dependency security review](maintainers/security-audit.md)
- [Contributing](../CONTRIBUTING.md) and [security reporting](../SECURITY.md)
- Architecture decisions: [schematics](decisions/0001-phase-4-schematic-and-web-architecture.md),
  [browser runtime](decisions/0002-browser-simulation-runtime.md),
  [external models](decisions/0003-external-subcircuit-references.md),
  [evaluator-owned requirements](decisions/0004-evaluator-owned-requirements.md).

## Quality evidence

[Six-task comparison results](evals/unseen-design-summary-2026-09-13.md) summarize the bounded
agent/toolchain evaluation. Protocols, per-task results, verification-claim boundaries and
dated schematic reviews remain under [evals/](evals/). Negative results and superseded visual
candidates are retained; they are not claims about current universal capability.

The accepted [schematic baseline](evals/schematic-quality-candidate-2026-08-14.md)
records the visual regression method and acceptance boundaries.

## Document maintenance

User guides and references must match supported behavior. Architecture and release contracts
are normative. Supersede architectural decisions with new ADRs; do not silently rewrite
historical reasoning. Preserve dated evaluation outcomes and disclose corrections.

Personal notes, account/access details, commercial strategy and internal execution plans do
not belong in this repository or release packages. Contributor-facing policies and licensing
terms remain public. Release documentation is explicitly listed in
[public-documents.json](public-documents.json); new public documents must be reviewed and added
there.

Before adding a document, check whether it belongs in an existing guide, reference, decision
or evaluation record. Every document should have a clear audience and purpose.
