# Troubleshooting

## `KES-P...`: parse error

Check the reported line/column against [language reference](../reference/language.md). Kessetsu is line-oriented; commas belong between connection pins and `to` is required.

## `KES-C...` or `KES-E...`: compile/ERC error

Verify names, pin identifiers, physical units, one unambiguous `GND`, and that every required component pin is connected. Raw SPICE directives and unversioned model packages are intentionally rejected.

For external subcircuits, `KES-C014` means the typed metadata, path, or compatibility value is invalid; `KES-C015` means the declared resource bytes were not supplied; `KES-C016` means their SHA-256 differs from the declaration; and `KES-C017` means the file is not valid UTF-8 or does not contain exactly one matching `.SUBCKT` entry with the declared terminal count. Native CLI commands resolve resources relative to a file-based `.kess` source; stdin cannot bind these files. On Web, select the exact local file in **View → Circuit details…**. Browser-incompatible models need the CLI. No generic model is substituted. See the [model catalog](../reference/model-catalog.md).

## Installed on Windows, but the agent cannot find `kess`

The installer updates User PATH and the PowerShell session executing it. An already-running
IDE/agent or its parent process may still hold the previous PATH. Restart that application,
or give your agent the full launcher path printed by the installer. This is not necessarily
an installation failure. In ordinary PowerShell you can also run:

```powershell
& "$env:LOCALAPPDATA\Kessetsu\bin\kess.cmd" --version
```

For a custom installation directory, use its printed command path instead.

## Simulator not found or exit 3

Run the CLI with the release-provided simulator instructions, or set `KESSETSU_NGSPICE` to the full Ngspice executable path. Kessetsu probes the executable version before use; a directory or incompatible binary fails closed. Use `--include raw-log --format json` only when diagnosing simulator output.

## Assertion returns `ERROR`

The required analysis or signal is missing or incompatible. AC metrics need an AC source; transient metrics need enough samples, and THD needs at least two fundamental periods. The message on the assertion explains the exact missing condition. See [simulation and assertions](../reference/simulation-and-assertions.md).

## Output already exists

Generated files are never silently overwritten. Pick a new path or add `--force` after confirming the target. The source file itself cannot be an output target even with `--force`.

`KES-I007` means a SPICE or LTspice export depends on an external model file but the requested output directory differs from the source directory. Write the export beside the `.kess` source so its validated relative include remains usable. Kessetsu does not silently copy or redistribute the model.

`KES-I008` means the external `.kessreq` file could not be read. Check the filename and permissions; Kessetsu does not fall back to inline assertions when an external requirement path was requested.

## A shared URL will not open

Kessetsu accepts only `kessetsu.share.v1`, limits compressed input and decompressed source size, and verifies the embedded Core schema and package versions. A truncated URL, older/newer schema or modified package manifest is rejected instead of partially loading. Ask the sender for a URL created by the same public release.

## Web simulation does not start

Confirm that JavaScript, WebAssembly and module Workers are allowed and reload once. The production build pins and integrity-checks the runtime. Browser simulation runs locally; the Web Hub sends no telemetry and has no server-side fallback.

## KiCad/LTspice warning on open

Read the export capability/loss message. KiCad files embed portable Kessetsu symbols and may report a symbol-library-table warning; LTspice uses its standard symbol library. Connectivity is release-smoke-tested, but Kessetsu assertions remain in the `.kess` source and are not editable-EDA assertions.
