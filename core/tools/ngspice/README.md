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
matrix and smoke contract are in the [release document](../../../docs/maintainers/release.md).

## Tracked runtime profile

| File | Purpose |
| --- | --- |
| `bin/ngspice_con.exe` | Console simulator invoked by Kessetsu in batch mode |
| `bin/libomp140.x86_64.dll` | OpenMP runtime for this Windows binary |
| `lib/ngspice/analog.cm` | XSPICE transfer-function models required by PSpice library translation |
| `lib/ngspice/xtradev.cm` | XSPICE analog-switch models required by PSpice `VSWITCH` translation |
| `share/ngspice/scripts/spinit` | Deterministic minimal startup settings |
| `docs/COPYING` | Upstream license texts and exceptions |
| `docs/AUTHORS` | Upstream attribution record |
| `docs/README` | Upstream project and source information |

The GUI executable, upstream example/test tree, PDF manual, development notes,
the other five XSPICE `.cm` modules, and OpenVAF/OSDI model libraries are not
part of Kessetsu's current runtime profile. `spinit` loads only the two modules
above and explicitly disables the absent OSDI family. Any broader feature set
requires a separate runtime-profile review and fixtures.

When this packaged layout is detected, the native runner sets `SPICE_LIB_DIR`
to the adjacent `share/ngspice` directory and starts Ngspice with `-n`. This
selects the tracked `scripts/spinit` deterministically without reading a user's
home-directory startup file. A declared `ngspice_ps` external subcircuit adds
only `-D ngbehavior=ps`; source-controlled arbitrary command-line arguments are
never accepted.

## Verification scope

The base profile was verified on August 8, 2026, and its manufacturer-model
extension on September 11, 2026:

1. In a clean temporary directory containing only the runtime files above,
   `ngspice_con.exe --version` reported version 46.
2. The SPICE netlist generated from `core/tests/fixtures/valid/feature_matrix.kess` ran in batch
   mode with exit code 0.
3. Measurement output produced `max_v_my_signal = 6.20001e-08` with no missing
   initialization, code-model, or OSDI-file errors.
4. The exact user-supplied TI OPAx197 Rev. D model ran in PSpice library mode
   without modifying or redistributing the model; evaluator-owned OP, AC, and
   transient checks passed.

The source archive was the official `ngspice-46_64.7z` from SourceForge
(`SHA-256 7ed713cd8d401db724ffe99087c3122bf05a9cfa99de02c6eeed44ee44785a33`).
The tracked module hashes are
`6173f1621c91b77c4a32c1573e55bfff97fc144d838eb7648e76e50f843a9202`
(`analog.cm`) and
`28899526c5024193160a4cdcc96b11723b4ced4494cc5d6ca4e95f573cd7773f`
(`xtradev.cm`).

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
