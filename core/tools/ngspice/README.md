# Kessetsu Ngspice runtime

This directory contains the deliberately minimized Ngspice runtime package used
for native Kessetsu simulation on Windows. It is not an Ngspice source or test
distribution.

## Version and source

- Version: **ngspice-46**, Windows x86-64 console build
- Build date reported by the binary: March 29, 2026
- Official download page: <https://ngspice.sourceforge.io/download.html>
- Official documentation: <https://ngspice.sourceforge.io/docs.html>
- Official development and license overview: <https://ngspice.sourceforge.io/devel.html>

The version is verified through `bin/ngspice_con.exe --version`. Kessetsu first
tries the explicit executable path in `KESSETSU_NGSPICE`, then repository/release
Windows sidecar locations, and finally the platform-appropriate system fallback
of `ngspice_con.exe` or `ngspice`. First-release Linux/macOS packages use a
version-probed `ngspice` installed by the system package manager; the exact
matrix and smoke contract are in the [release document](../../../docs/release.md).

## Tracked runtime profile

| File | Purpose |
| --- | --- |
| `bin/ngspice_con.exe` | Console simulator invoked by Kessetsu in batch mode |
| `bin/libomp140.x86_64.dll` | OpenMP runtime for this Windows binary |
| `share/ngspice/scripts/spinit` | Deterministic minimal startup settings |
| `docs/COPYING` | Upstream license texts and exceptions |
| `docs/AUTHORS` | Upstream attribution record |
| `docs/README` | Upstream project and source information |

The GUI executable, upstream example/test tree, PDF manual, development notes,
XSPICE `.cm` code models, and OpenVAF/OSDI model libraries are not part of
Kessetsu's current analog runtime profile. `spinit` is explicitly configured not
to load those absent optional libraries. If any such feature enters product
scope, it requires a separate runtime profile backed by fixture and distribution
review.

## Verification scope

The following was verified on August 8, 2026:

1. In a clean temporary directory containing only the runtime files above,
   `ngspice_con.exe --version` reported version 46.
2. The SPICE netlist generated from `examples/test_features.kess` ran in batch
   mode with exit code 0.
3. Measurement output produced `max_v_my_signal = 6.20001e-08` with no missing
   initialization, code-model, or OSDI-file errors.

This smoke verification does not guarantee every supported Ngspice feature.
Canonical integration fixtures additionally lock OP, transient, AC, and DC
behavior, while RC, gain-stage, and power-amplifier benchmarks use real
simulation.

## License and distribution gate

`docs/COPYING` contains both the upstream package's base Modified BSD license and
exceptions for components such as KLU, OSDI, and XSPICE. The distributed binary
reports in `--version` output that it was built with the KLU solver, so a
distribution decision must not rely only on the main project license. License
and attribution records must remain with the binary.

This document is not legal advice. The release package preserves this directory's
complete notice inventory, binary provenance, and SHA-256 records; the release
gate also runs a dependency/license audit.

## Upgrade procedure

1. Obtain the binary only from the official Ngspice download page.
2. Record `--version` output and target architecture.
3. Refresh `docs/COPYING`, `docs/AUTHORS`, and the upstream README from the same distribution.
4. Verify the minimal file set in a clean temporary directory.
5. Run the Phase 3 simulator-integration fixtures.
6. Update this document and the roadmap evidence.
