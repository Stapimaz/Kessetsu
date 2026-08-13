# Troubleshooting

## `KES-P...`: parse error

Check the reported line/column against [language reference](language_reference.md). Kessetsu is line-oriented; commas belong between connection pins and `to` is required.

## `KES-C...` or `KES-E...`: compile/ERC error

Verify names, pin identifiers, physical units, one unambiguous `GND`, and that every required component pin is connected. Raw SPICE directives and unversioned model packages are intentionally rejected.

## Simulator not found or exit 3

Run the CLI with the release-provided simulator instructions, or set `KESSETSU_NGSPICE` to the full Ngspice executable path. Kessetsu probes the executable version before use; a directory or incompatible binary fails closed. Use `--include raw-log --format json` only when diagnosing simulator output.

## Assertion returns `ERROR`

The required analysis or signal is missing or incompatible. AC metrics need an AC source; transient metrics need enough samples, and THD needs at least two fundamental periods. The message on the assertion explains the exact missing condition. See [simulation and assertions](simulation_and_assertions.md).

## Output already exists

Generated files are never silently overwritten. Pick a new path or add `--force` after confirming the target. The source file itself cannot be an output target even with `--force`.

## A shared URL will not open

Kessetsu accepts only `kessetsu.share.v1`, limits compressed input and decompressed source size, and verifies the embedded Core schema and package versions. A truncated URL, older/newer schema or modified package manifest is rejected instead of partially loading. Ask the sender for a URL created by the same public release.

## Web simulation does not start

Confirm that JavaScript, WebAssembly and module Workers are allowed and reload once. The production build pins and integrity-checks the runtime. Browser simulation runs locally; the first release sends no telemetry and has no server-side fallback.

## KiCad/LTspice warning on open

Read the export capability/loss message. KiCad files embed portable Kessetsu symbols and may report a symbol-library-table warning; LTspice uses its standard symbol library. Connectivity is release-smoke-tested, but Kessetsu assertions remain in the `.kess` source and are not editable-EDA assertions.
