# Kessetsu Identity Migration

This document is the contract for a one-time clean migration from the unpublished project's temporary identity to Kessetsu. Other Phase 4 work waits until the migration is complete; a partial brand migration cannot be released.

## Final identity

| Surface | New identity |
|---|---|
| Product and repository name | `Kessetsu` |
| CLI executable and command | `kess` |
| Circuit source extension | `.kess` |
| Rust package | `kessetsu-core` |
| Rust crate/import | `kessetsu_core` |
| Rust CLI source | `core/src/bin/kess.rs` |
| Parser grammar | `core/src/kessetsu.pest` |
| Versioned schema namespace | `kessetsu.*` |
| Diagnostic prefix | `KES-*` |
| Built-in model prefix | `KESSETSU_*` |
| Environment prefix | `KESSETSU_*` |
| Model lockfile | `kessetsu.lock` |
| Website/domain | `kessetsu.com` |

## Migration policy

- Because the project is not yet public, no compatibility aliases remain for the former product name, source extension, schema identities, diagnostic codes, or environment variables.
- This is not a visible-text-only change. File paths, package and binary names, serialization contracts, fixtures, goldens, workflows, release artifacts, and documentation move together.
- Circuit IR, deterministic node naming, ERC, and exporter architecture remain unchanged; only project identity and dependent public-contract names move.
- Git history is not rewritten. The former identity in historical commits is not considered residue on an active product surface.
- The repository name and local root-folder name are changed as separate external steps after the content migration is verified.

## Implementation and acceptance checklist

- [x] Rename tracked source and fixture files to `.kess` and update every reference.
- [x] Migrate the CLI binary, Rust package/crate, and parser grammar identities.
- [x] Migrate schema, diagnostic, built-in model, lockfile, and environment identities.
- [x] Migrate Web Hub/WASM names, user-facing text, sharing, and export contracts.
- [x] Migrate README, architecture, references, license/notice text, and other documentation.
- [x] Migrate GitHub Actions, Pages, release artifacts, scripts, and repository URLs.
- [x] Regenerate generated/golden files through canonical producers.
- [x] Verify with a case-insensitive audit that the active tracked tree contains no former product name, crate name, environment/model prefix, or source extension.
- [x] Verify `kess --help`, representative `.kess` compile/simulate/render/export paths, and JSON schema/diagnostic paths.
- [x] Pass the canonical `scripts/verify.ps1` quality gate.
- [x] Close the roadmap and this document with verification evidence.
- [x] Rename the GitHub repository to `Kessetsu` and verify origin fetch/push URLs.
- [ ] Rename the local root folder to `Kessetsu` after the VS Code/Codex session is closed.

## Deliberately outside this migration

- Domain DNS, production Web Hub deployment, and public launch.
- Trademark registration or a legal clearance opinion.
- Removing old identity references from historical Git commits.
- Feature changes to circuit-language semantics, simulation behavior, or the schematic-layout algorithm.

## Verification evidence

Verification on 2026-08-14:

- A case-insensitive full-tree audit found no former product name, crate/model/environment/diagnostic prefix, executable identity, or source extension outside Git history and third-party dependency directories.
- `kess --help` verified the correct binary/product identity. An RC-focused smoke verified `kessetsu.cli.v1`, `kessetsu.compile.v3`, and `kessetsu.simulation.v1`, plus 5/5 assertion results. SVG and KiCad artifacts were generated from `.kess` source.
- The schematic corpus was regenerated through the canonical Kessetsu producer. Six deterministic SVG golden hashes affected by the schema change were updated from the new bytes, and connectivity/quality gates passed.
- After a clean cache, canonical `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/verify.ps1` passed completely: Rust formatting/Clippy/tests/release, WASM, Web lint plus 10 unit tests and production build, runtime/deployment audit, 11 Chromium E2E tests, dependency/license/security audit, replayable agent eval, real Ngspice 12/12 smoke using the packaged Windows `kess.exe`, and RC/gain/power KiCad plus LTspice smoke tests.
- Ignored release/WASM/Web-test artifacts with the former name were removed. The Rust build cache was rebuilt from scratch using only the new identity.
- Commit `79c7632` was pushed to the private `main` branch. The GitHub repository was renamed to `Stapimaz/Kessetsu`, and origin fetch/push URLs plus the remote `main` commit were verified.
- The only remaining local step is renaming the VS Code/Codex session root folder to `Kessetsu` after closing the session; it does not affect tracked product or release content.
