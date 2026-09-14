# Release and Deployment Contract

## CLI artifact matrix

| Artifact | Runner/architecture | Simulator policy |
|---|---|---|
| `windows-x86_64.zip` | Windows Server x86-64 | Bundled, version-probed Ngspice 46 plus complete upstream notice inventory |
| `linux-x86_64.tar.gz` | Ubuntu x86-64 | Trusted `ngspice` on `PATH`, or explicit `KESSETSU_NGSPICE` full path |
| `macos-x86_64.tar.gz` | macOS Intel | Homebrew/system `ngspice`, or explicit override |
| `macos-aarch64.tar.gz` | macOS Apple Silicon | Homebrew/system `ngspice`, or explicit override |

Every archive contains `kess` (`kess.exe` on Windows), platform-specific `INSTALL.txt`, the public README/documentation and `.kess` examples, `LICENSE`, `NOTICE`, `COMMERCIAL_LICENSE.md`, `SUPPORTED_DOMAIN.md` and `release-manifest.json`; a sibling `.sha256` protects the archive. The manifest records target, Git commit, simulator policy and executable SHA-256. `scripts/smoke-release.ps1` extracts to a new temporary directory, verifies that the binary and manifest versions agree, performs a real power-amplifier simulation and requires 12/12 assertions.

## Web production

The production workflow builds the pinned Rust/WASM/Node dependency graph and deploys `webapp/dist` through GitHub Pages. Vite content-hashes JavaScript, CSS, WASM and Worker assets; GitHub controls transport/cache headers, while mutable `index.html` selects the current hashes. GitHub Pages supplies `.wasm` and module JavaScript MIME types. A restrictive meta CSP permits only same-origin code/data plus the minimum WebAssembly, inline Monaco style and Worker/blob capabilities. The build includes the project license, commercial-license notice and third-party inventories; the interactive header links to the exact public Corresponding Source.

The first release sends no analytics or error telemetry. Failures remain in local UI/console state. This avoids silently collecting circuit source; server telemetry may be introduced only with a documented privacy boundary and opt-in/necessity review.

The canonical public origin is `https://kessetsu.com/`. The production bundle includes its `CNAME`, canonical/Open Graph/Twitter metadata, `robots.txt`, sitemap, and current workspace preview. `scripts/audit-performance.mjs` records and gates three gzip transfer stages in `performance-budget.json`: the initial landing shell (90 KiB), editor activation including Monaco/Core WASM (2,800 KiB), and the browser simulator loaded only on first Run (5,700 KiB). These are regression ceilings, not claims about network latency; release review uses the generated measured values.

Rollback is a normal workflow dispatch: choose a previously verified tag/commit in the `ref` input. The workflow rebuilds that immutable source and atomically replaces the Pages deployment. A release tag is never moved.

## First-public-release transaction

The prepared repository state does not itself publish anything. Execute the external transaction in this order:

1. Confirm the final candidate has passed local canonical verification and remote CI, then change repository visibility to public.
2. Configure GitHub Pages for the Actions workflow and set `kessetsu.com` as the custom domain. Apply the DNS records required by GitHub, wait for its DNS check, and enable HTTPS before announcing the URL. The built site already contains the matching root-domain `CNAME` and `/` asset base.
3. Manually dispatch **Web Hub production** for the exact candidate commit. Verify the real origin, content/MIME/cache headers, CSP, Web compile/simulation/export path, and a dispatch-based redeploy before tagging.
4. Manually dispatch **Release matrix** and require all four clean package smokes. Create the immutable annotated `v1.0.0` tag only after both production and package evidence pass; the tag publishes the selected changelog section and rebuilds the same Web source.
5. Verify the GitHub Release assets/checksums and `https://kessetsu.com/` once more. If any gate fails, do not move the tag; redeploy a known verified ref and publish a new patch version after correction.

## Release gates

1. Canonical `scripts/verify.ps1` passes without changing tracked files.
2. RustSec, npm production vulnerability, project/Rust/npm license metadata and generated-artifact audits pass; the root and Cargo-package AGPL texts match exactly, and informational risk acceptances are recorded in [security audit](security_audit.md).
3. Four clean-runner CLI packages pass real simulation smoke tests.
4. KiCad/LTspice round-trip evidence and browser/native benchmark parity pass.
5. Web production build, CSP, runtime integrity, browser E2E and Pages deployment pass.
6. Changelog/migration notes, checksums, notices, screenshots and support boundaries are present.
7. Project source is released under `AGPL-3.0-only`; a separate commercial license is available only by signed agreement. Repository visibility changes only after the technical gates pass.

## Release commands

`VERSION` is the sole product-version authority. Cargo, npm, packaging and release automation must match it through `scripts/verify-version.ps1`; machine-contract versions such as `kessetsu.cli.v1` evolve independently. The normal path is a signed/annotated `v1.0.0` tag after all gates. The tag triggers the package matrix, GitHub Release publication and Web deployment. A manual workflow dispatch tests the matrix without publishing a GitHub Release. GitHub release notes are extracted only from the matching version section in `CHANGELOG.md`, never from the entire history.
