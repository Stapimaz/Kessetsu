# ADR 0002 — Browser Simulation Runtime

- Status: Accepted
- Date: 2026-08-13
- First-release decision: Ngspice WASM inside a Web Worker

## Options considered

| Criterion | `eecircuit-engine` 1.7.0 | Self-built Ngspice/Emscripten | Service-backed native Ngspice |
|---|---|---|---|
| Engine | Ngspice WASM wrapper | Ngspice WASM | Native Ngspice |
| Local artifact | npm package: 40,693,211 bytes unpacked; main ESM bundle approximately 20.4 MB | Depends on build options | Small browser artifact; requires server/container |
| Startup | Initial worker/WASM parse cost; measured as a gate in the 4.2 parity test | Similar, plus ownership of the build chain | Network plus cold start |
| Cancellation | Hard boundary through worker termination/restart | Hard boundary through worker termination/restart | HTTP abort alone does not stop server work; server-side cancellation is required |
| Model support | Ngspice netlist/model semantics, to be verified with the Kessetsu security/parity corpus | Depends on build flags | Broadest support and current parity with native |
| Offline/privacy | Yes; the circuit never leaves the browser | Yes | No; source/netlist is sent to a server |
| Deployment | Static asset plus worker MIME/cache | Static asset plus worker MIME/cache, and a reproducible Emscripten build | Stateful/isolated execution service, queue, and abuse controls |
| Supply chain | Exact npm version/integrity plus upstream source/license | We own the source commit, toolchain, and build recipe completely | OS/container Ngspice provenance |

## Decision and rationale

The first public Web Hub will run the exact `eecircuit-engine` **1.7.0** package only inside a Web Worker. The package is used for real browser simulation by the active EEcircuit project, and its wrapper is MIT-licensed. Most Ngspice code uses a modified BSD license; transitive license notices and an exact hash are retained separately for the distributed artifact.

This choice is not a permanent domain dependency. The worker adapter accepts only canonical SPICE and a typed request, then converts raw engine output to `kessetsu.simulation.v1`. Measurement/assertion evaluation runs in WASM Core over that same typed dataset. The UI never sees `eecircuit-engine` types. A later move to a self-built runtime or service adapter remains behind this boundary.

The service-backed path was rejected for the first release. Although it can meet the zero-friction goal, it sends circuit data over the network and creates a separate secure-process service and operational surface. If native parity fails, there will be no silent fallback; Web will show a structured unsupported/runtime diagnostic.

Wokwi's `ngspice-wasm` build recipe demonstrates technical feasibility, but its repository has one commit, offers no release artifact/API contract, and last received a source push in 2022. It was not selected as the first-release runtime dependency. If long-term supply-chain control becomes necessary, it remains a reproducible self-build candidate behind the same adapter.

## Mandatory acceptance gates

1. The worker does not block the main thread; timeout/cancel terminates it and starts a clean instance.
2. Simulator provenance is visible from runtime initialization output; package version and asset SHA-256 are recorded.
3. RC, gain-stage, and power-amplifier canonical netlists produce the same native/Web PASS/FAIL decisions within declared tolerances.
4. Raw-log debugging is opt-in; large datasets are not copied twice into UI state.
5. Runtime/model license notices are distributed with the public artifact.

## Artifact update and cache policy

- The runtime dependency is pinned to exact `eecircuit-engine@1.7.0`, not a floating range. Updates require joint review of package integrity, ESM SHA-256, browser parity, and license inventory.
- `runtime-manifest.json` records both package integrity and the SHA-256 of the distributed 20,424,332-byte ESM source. A hash change without a version change is supply-chain drift and stops the build.
- In production, content-hashed JS/WASM/Worker assets use `public,max-age=31536000,immutable`; HTML, the runtime manifest, and notice files use `no-cache`. A new runtime never replaces an old hashed asset in place.
- The Vite release build copies the EEcircuit MIT text and complete Ngspice licensing inventory into `dist/licenses/`.

## Sources

- [Ngspice FAQ and shared-library/license information](https://ngspice.sourceforge.io/faq.html)
- [Ngspice developer and license information](https://ngspice.sourceforge.io/devel.html)
- [EEcircuit browser application](https://github.com/eelab-dev/EEcircuit)
- [`eecircuit-engine` source repository](https://github.com/eelab-dev/EEcircuit-engine)
- [Wokwi Ngspice WASM build recipe](https://github.com/wokwi/ngspice-wasm)
