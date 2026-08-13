# Release and Deployment Contract

## CLI artifact matrix

| Artifact | Runner/architecture | Simulator policy |
|---|---|---|
| `windows-x86_64.zip` | Windows Server x86-64 | Bundled, version-probed Ngspice 46 plus complete upstream notice inventory |
| `linux-x86_64.tar.gz` | Ubuntu x86-64 | Trusted `ngspice` on `PATH`, or explicit `KESSETSU_NGSPICE` full path |
| `macos-x86_64.tar.gz` | macOS Intel | Homebrew/system `ngspice`, or explicit override |
| `macos-aarch64.tar.gz` | macOS Apple Silicon | Homebrew/system `ngspice`, or explicit override |

Every archive contains `kess` (`kess.exe` on Windows), `INSTALL.txt`, `README.md`, `LICENSE`, `NOTICE`, `COMMERCIAL_LICENSE.md`, `SUPPORTED_DOMAIN.md` and `release-manifest.json`; a sibling `.sha256` protects the archive. The manifest records target, Git commit, simulator policy and executable SHA-256. `scripts/smoke-release.ps1` extracts to a new temporary directory, verifies the binary, probes its version, performs a real power-amplifier simulation and requires 12/12 assertions.

## Web production

The production workflow builds the pinned Rust/WASM/Node dependency graph and deploys `webapp/dist` through GitHub Pages. Vite content-hashes JavaScript, CSS, WASM and Worker assets; GitHub controls transport/cache headers, while mutable `index.html` selects the current hashes. GitHub Pages supplies `.wasm` and module JavaScript MIME types. A restrictive meta CSP permits only same-origin code/data plus the minimum WebAssembly, inline Monaco style and Worker/blob capabilities. The build includes the project license, commercial-license notice and third-party inventories; the interactive header links to the exact public Corresponding Source.

The first release sends no analytics or error telemetry. Failures remain in local UI/console state. This avoids silently collecting circuit source; server telemetry may be introduced only with a documented privacy boundary and opt-in/necessity review.

Rollback is a normal workflow dispatch: choose a previously verified tag/commit in the `ref` input. The workflow rebuilds that immutable source and atomically replaces the Pages deployment. A release tag is never moved.

## Release gates

1. Canonical `scripts/verify.ps1` passes without changing tracked files.
2. RustSec, npm production vulnerability, project/Rust/npm license metadata and generated-artifact audits pass; the root and Cargo-package AGPL texts match exactly, and informational risk acceptances are recorded in [security audit](security_audit.md).
3. Four clean-runner CLI packages pass real simulation smoke tests.
4. KiCad/LTspice round-trip evidence and browser/native benchmark parity pass.
5. Web production build, CSP, runtime integrity, browser E2E and Pages deployment pass.
6. Changelog/migration notes, checksums, notices, screenshots and support boundaries are present.
7. Project source is released under `AGPL-3.0-only`; a separate commercial license is available only by signed agreement. Repository visibility changes only after the technical gates pass.

## Release commands

The normal path is a signed/annotated `v0.1.0` tag after all gates. The tag triggers the package matrix, GitHub Release publication and Web deployment. A manual workflow dispatch tests the matrix without publishing a GitHub Release.
