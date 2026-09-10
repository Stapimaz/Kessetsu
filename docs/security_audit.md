# Release Security Audit

Audit date: 2026-08-13. This is an engineering dependency review, not a legal opinion or proof that the software has no vulnerabilities.

## Automated gates

2026-09-10 maintenance note: full npm audit identified `GHSA-82fw-gwwq-j7x9` in the development-only Vitest/mocker 4.1.10 dependency chain. The lockfile now resolves Vitest 4.1.11 and the lockfile audit reports zero vulnerabilities. The Windows RustSec bootstrap now retries extraction from its checksum-verified archive if a previous extraction left directories without `cargo-audit.exe`; the restored audit passes with the same two informational notices below.

- `npm audit --omit=dev`: zero known production vulnerabilities.
- RustSec `cargo-audit 0.22.2`: zero vulnerability advisories affecting the locked 128-crate graph.
- Rust license metadata: all 127 third-party crates declare a license; no GPL/AGPL/SSPL identifier detected.
- npm license metadata: all 81 installed packages declare a license; no GPL/AGPL/SSPL identifier detected.
- Generated artifact audit: only intentional golden/simulation fixtures are tracked.
- Runtime integrity: exact npm lock/integrity plus SHA-256 for the browser simulator entry; complete Ngspice and font notices bundled.

## Informational RustSec notices

RustSec reports `RUSTSEC-2026-0206` (`rustybuzz 0.20.1`) and `RUSTSEC-2026-0192` (`ttf-parser 0.25.1`) as **unmaintained**, not as known vulnerabilities. They are transitive dependencies of the current `usvg/resvg/svg2pdf` visual export stack. The upstream resvg project still documents these libraries as part of its rendering stack and provides the deterministic native/WASM behavior Kessetsu requires.

Risk controls for 0.1.0:

- Kessetsu does not parse arbitrary user-supplied SVG or font files. It renders its own canonical Schematic IR projection with one repository-pinned OFL font.
- Export scale and source size are bounded, and unverified circuit connectivity fails before rendering.
- The notices remain visible on every RustSec run; they are not placed on an ignore list.
- Upgrade or replacement is required when the resvg/svg2pdf ecosystem exposes a maintained compatible path, or immediately if a vulnerability/unsoundness advisory changes the risk.

The release gate fails on vulnerability advisories. Informational unmaintained notices are accepted only with the bounded-input rationale above.
