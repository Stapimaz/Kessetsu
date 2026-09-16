# Release and Deployment Contract

## CLI artifact matrix

| Artifact | Runner/architecture | Simulator policy |
|---|---|---|
| `windows-x86_64.zip` | Windows Server x86-64 | Bundled, version-probed Ngspice 46 plus complete upstream notice inventory |
| `linux-x86_64.tar.gz` | Ubuntu x86-64 | Trusted `ngspice` on `PATH`, or explicit `KESSETSU_NGSPICE` full path |
| `macos-x86_64.tar.gz` | macOS Intel | Homebrew/system `ngspice`, or explicit override |
| `macos-aarch64.tar.gz` | macOS Apple Silicon | Homebrew/system `ngspice`, or explicit override |

Every archive contains `kess` (`kess.exe` on Windows), platform-specific `INSTALL.txt`, the public README/documentation, `.kess` examples and evaluator-owned `.kessreq` requirements, `LICENSE`, `NOTICE`, `COMMERCIAL_LICENSE.md`, `SUPPORTED_DOMAIN.md` and `release-manifest.json`; a sibling `.sha256` protects the archive. The manifest records target, Git commit, simulator policy and executable SHA-256. `scripts/smoke-release.ps1` extracts to a new temporary directory, verifies binary/manifest versions, checks the newcomer path with inline and hash-pinned external assertions plus SVG/KiCad export, then requires the real power-amplifier simulation to pass 12/12 assertions.

## Public documentation boundary

New packages copy only the reviewed files in `docs/public-documents.json`, not an arbitrary
recursive documentation directory. `scripts/audit-public-docs.ps1` checks list completeness,
paths, Markdown links and selected publication-sensitive patterns before packaging.
These focused checks are not a comprehensive secret scanner.

Detailed internal plans and personal/commercial operational notes must remain outside the
source checkout. Local agent planning configuration is not a distributable artifact.
The public roadmap describes product direction rather than private task execution.

This policy applies to future packages. Previously published `v1.0.0` archives and Git
history retain their original documentation; current-tree cleanup does not erase those copies.
Do not silently replace immutable downloads, checksums or tags.

## Web production

The guided `/install/` page and root `install.ps1`/`install.sh` bootstrappers are website distribution tools, independent of CLI binary releases. They resolve the latest published stable CLI (or an explicitly requested stable version) from the official GitHub repository; shipping a website-only installation improvement does not rebuild or retag an unchanged CLI. Product/runtime releases still use `VERSION` and immutable SemVer tags. Record distribution changes under `[Unreleased]` until the next versioned product release.

Windows installs complete bundles under `%LOCALAPPDATA%\Kessetsu\releases` and switches a `bin\kess.cmd` launcher; only user/process PATH is changed. POSIX installs under `~/.local/share/kessetsu/releases`, switches `~/.local/bin/kess`, and idempotently appends its PATH line to bash/zsh/profile configuration (fish uses `fish_add_path`). Existing unowned paths are refused. Downloads, archive structure, executable hashes, platform/version manifests, and CLI launch are checked before switching the launcher; old bundles are retained. The scripts have no telemetry, elevation, permanent execution-policy changes, or automatic privileged Ngspice installation.

For isolated developer testing, use PowerShell `-InstallDir <dedicated-directory> -NoPath` or POSIX `--install-dir <absolute-dedicated-directory> --no-path`. Windows contracts load only installer functions and mock PATH writes; never run a default installer on the developer's account just to test it. `scripts/test-installer.ps1 -Archive <Windows-archive>` covers local Windows contracts; `node --test scripts/installer.test.mjs` runs synthetic POSIX contracts on Linux/macOS and intentionally skips Windows. The separate **Installer smoke** workflow exercises all four real platforms using published `1.0.0` artifacts, including default PATH integration and repeated updates, without rebuilding the circuit engine.

After a website installation update, run only its live browser checks with `npm run test:production -- tests/e2e/install.spec.ts` from `webapp/`, then compare the two downloaded installer files and RC example to the verified source bytes. The full production suite remains available for wider release changes.

The production workflow builds the pinned Rust/WASM/Node dependency graph and deploys `webapp/dist` through GitHub Pages. Vite content-hashes JavaScript, CSS, WASM and Worker assets; GitHub controls transport/cache headers, while mutable `index.html` selects the current hashes. GitHub Pages supplies `.wasm` and module JavaScript MIME types. A restrictive meta CSP permits only same-origin code/data plus the minimum WebAssembly, inline Monaco style and Worker/blob capabilities. The build includes the project license, commercial-license notice and third-party inventories; the interactive header links to the exact public Corresponding Source.

The first release sends no analytics or error telemetry. Failures remain in local UI/console state. This avoids silently collecting circuit source; server telemetry may be introduced only with a documented privacy boundary and opt-in/necessity review.

The canonical public origin is `https://kessetsu.com/`. The production bundle includes its `CNAME`, canonical/Open Graph/Twitter metadata, `robots.txt`, sitemap, and current workspace preview. `scripts/audit-performance.mjs` records and gates three gzip transfer stages in `performance-budget.json`: the initial landing shell (90 KiB), editor activation including Monaco/Core WASM (2,800 KiB), and the browser simulator loaded only on first Run (5,700 KiB). These are regression ceilings, not claims about network latency; release review uses the generated measured values.

Rollback is a normal workflow dispatch: choose a previously verified tag/commit in the `ref` input. The workflow rebuilds that immutable source and atomically replaces the Pages deployment. A release tag is never moved.

The `github-pages` environment permits deployments from branch `main` and tags matching `v*`. Preserve both narrow rules: tag-triggered releases otherwise build successfully but are rejected at deployment. Deployment policy does not change repository write permissions. If deployment is rejected by a protection rule, stop for owner approval rather than weakening or bypassing that rule.

## Static public content maintenance

`npm run build:web` pre-renders the landing, installation and circuit-tool React components,
then publishes the reviewed documentation index, guides and references as static `/docs/`
pages. Markdown remains the source of truth; contributor/evaluation documents stay linked
on GitHub. The build generates the sitemap from those same pages. Documentation pages
load no application JavaScript; the editor remains lazily loaded and requires JavaScript.

For content/metadata changes, use the Web build, deployment audit and focused
`discovery.spec.ts` / installation checks rather than repeating the full engine suite.
Preserve the canonical domain, CSP, approved wordmark and binary-release identities.
After deployment, verify the initial HTML and relevant public routes. Crawlable content
and metadata do not prove indexing or search ranking; account ownership and sitemap
submission belong to the owner's Search Console workflow.

## First-public-release transaction

The prepared repository state does not itself publish anything. Execute the external transaction in this order:

1. Confirm the final candidate has passed local canonical verification and remote CI, then change repository visibility to public.
2. Configure GitHub Pages for the Actions workflow and set `kessetsu.com` as the custom domain. Apply the DNS records required by GitHub, wait for its DNS check, and enable HTTPS before announcing the URL. The built site already contains the matching root-domain `CNAME` and `/` asset base.
3. Manually dispatch **Web Hub production** for the exact candidate commit. Verify the real origin, content/MIME/cache headers, CSP, Web compile/simulation/export path, and a dispatch-based redeploy before tagging.
4. Manually dispatch **Release matrix** and require all four clean package smokes. Create the immutable annotated `v1.0.0` tag only after both production and package evidence pass; the tag publishes the selected changelog section and rebuilds the same Web source.
5. Verify the GitHub Release assets/checksums and `https://kessetsu.com/` once more. If any gate fails, do not move the tag; redeploy a known verified ref and publish a new patch version after correction.

The repeatable live-product smoke is `npm run test:production` from `webapp/`, after building the matching native release CLI. It uses real Chromium against `https://kessetsu.com/`, with no local preview server, and checks desktop/mobile entry, native/browser benchmark parity, Worker simulation/cancellation, all seven exports, and named sharing. Run it after the initial deployment, rollback/redeploy, and final release deployment. It does not upload circuit projects and is separate from canonical CI because the public network is an external dependency.

## Release gates

Use the full canonical gate for a release candidate, not for every subsequent documentation or deployment-setting edit. Reuse its evidence while the verified source remains unchanged; after publication, validate the public download checksums/manifests and a focused live-site smoke. Do not repeat the full local suite or await documentation-only CI solely to record release evidence.

1. Canonical `scripts/verify.ps1` passes without changing tracked files.
2. RustSec, npm production vulnerability, project/Rust/npm license metadata and generated-artifact audits pass; the root and Cargo-package AGPL texts match exactly, and informational risk acceptances are recorded in [security audit](security-audit.md).
3. Four clean-runner CLI packages pass real simulation smoke tests.
4. KiCad/LTspice round-trip evidence and browser/native benchmark parity pass.
5. Web production build, CSP, runtime integrity, browser E2E and Pages deployment pass.
6. Changelog and version-scoped migration notes, checksums, notices, screenshots and support boundaries are present.
7. Project source is released under `AGPL-3.0-only`; a separate commercial license is available only by signed agreement. Repository visibility changes only after the technical gates pass.

## Release commands

`VERSION` is the sole product-version authority. Cargo, npm, packaging and release automation must match it through `scripts/verify-version.ps1`; machine-contract versions such as `kessetsu.cli.v1` evolve independently. The normal path is a signed/annotated `v1.0.0` tag after all gates. The tag triggers the package matrix, GitHub Release publication and Web deployment. A manual workflow dispatch tests the matrix without publishing a GitHub Release. GitHub release notes are extracted only from the matching version section in `CHANGELOG.md`, never from the entire history.
