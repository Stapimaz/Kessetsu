# Research Data Import and Comparison

This guide covers working CSV import and scalar-data comparison, including a current
Web simulation as reference. Kessetsu also provides a thin local Python/Jupyter
adapter over these same contracts and a separate, evidence-preserving
[finite fitting workflow](model-fitting.md) over completed parameter studies.

Import local CSV with explicit columns and units, preserve its original text and compare
it with a separately imported reference. These commands do not launch a simulator, upload
files, change circuit source or declare that a model is physically valid.

## Complete example

Download the [synthetic scope-style CSV](../../examples/research/scope-style.csv),
[its mapping](../../examples/research/scope-style.kessimport.json),
[analytical reference CSV](../../examples/research/rc-reference.csv),
[reference mapping](../../examples/research/rc-reference.kessimport.json) and
[comparison mapping](../../examples/research/rc-comparison.kesscompare.json).
Their locations below assume a source checkout or extracted release package.
The scope-style numbers are illustrative, **not laboratory measurements**.

```sh
kess data preview examples/research/scope-style.csv --format json
kess data import examples/research/scope-style.csv --mapping examples/research/scope-style.kessimport.json --output capture.kessdata.json
kess data import examples/research/rc-reference.csv --mapping examples/research/rc-reference.kessimport.json --output reference.kessdata.json
kess data compare capture.kessdata.json --reference reference.kessdata.json --mapping examples/research/rc-comparison.kesscompare.json --output residuals.json --format json
```

The comparison reports six matched points and differences around 4–6 mV. Full residual
points are in `residuals.json`; stdout is a compact `kessetsu.cli.v1` envelope. Use
`--include datasets` to request normalized arrays/residual points in JSON stdout.
Exit `0` means the data operation completed, **not that a circuit passed requirements**.
Invalid input, identity, coverage or I/O returns `2` with `KES-R001`; malformed command
arguments and unsupported CLI schema also fail before file writes. Global `--param`
does not apply to data commands. Existing output needs `--force`, which cannot authorize
overwriting any input CSV, mapping, dataset or reference.

## Web workflow

In the Web editor, open **Analyze → Compare research data…**. The three explicit steps are:

1. Choose the observed CSV, review its dialect/preview, map axis and signal columns, units,
   calibration, missing-value behavior and origin, then import it.
2. Repeat for an independent reference CSV, or select an analysis and vectors from the current
   successful Web simulation. Transient/DC vectors retain real values; AC uses linear complex
   magnitude. The derived dataset records simulator identity, result hash, analysis and vectors.
3. Pair logical signals, select coverage/interpolation and any explicit shift/window/gap,
   then compare. The result shows an overlay and complete residual metrics and downloads as
   `.kesscompare.json`; either normalized source dataset can download as `.kessdata.json`.

The browser enforces the same Core schemas and limits as CLI. Files stay in the current tab,
are not uploaded or placed in browser storage, and do not change the open circuit. Closing and
reopening the dialog during the same editor session retains the local working set; refreshing
the page deliberately clears it. The visible plot may decimate very large series for display,
while the downloaded evidence retains every point. A projected simulation has explicit
`simulation` origin; comparing against it does not make the simulator model physically valid.

## Preview and import mapping

CSV is UTF-8, with optional BOM. Preview shows header columns, sample records and counts.
It shows at most eight records and 256 characters per field/caption, recording truncated indices.
This display limit never truncates the stored raw evidence.
column indices in mappings are **zero-based**. Duplicate instrument header captions do
not confuse indexed mappings; assigned logical names must be unique ignoring ASCII case.

```sh
kess data preview capture.csv --delimiter semicolon --decimal comma --skip-records 2 --format json
```

Preview defaults to comma separation, decimal dot and one header. `--no-header` treats
the first nonblank record as data. `--skip-records` explicitly excludes logical CSV
records before the header, not physical lines inside quoted fields.
The import mapping declares the same settings; preview does not save or infer them:

```json
{
  "schema_version": "kessetsu.data-import.v1",
  "name": "Device A, run 7",
  "file_name": "capture.csv",
  "dialect": {"delimiter": "semicolon", "decimal": "comma", "header": true, "preamble_records": 2},
  "metadata": {"origin": "measured", "device": "Device A", "sample": "run 7"},
  "axis": {"column": 0, "name": "time", "unit": "Second", "source_unit": "ms"},
  "signals": [{"column": 1, "name": "out", "unit": "Volt", "source_unit": "mV", "gain": 1, "offset": 0}],
  "missing": "error",
  "missing_tokens": ["", "NA"]
}
```

Quoted delimiters, escaped quotes, quoted line breaks, CRLF/LF and scientific `e/E/d/D`
exponents are supported. Decimal comma requires semicolon or tab separation. Mixed decimal
conventions, thousands separators, ragged records, malformed quoting and non-finite or
out-of-range values fail; numerical cell contents must be numbers, not `1k` quantities.

`unit` uses Core enums: `Volt`, `Ampere`, `Second`, `Hertz`, `Ohm`, `Farad`, `Henry`,
`Watt`, `Joule`, `Ratio`, `Percent`, `Degree`. `source_unit` is an explicit compatible
suffix such as `mV`, `uA`, `ms` or `kHz`; dimensionless `Ratio` uses `""` or `"1"`.
Percent is stored in percent units, consistent with Core, not silently converted to Ratio.
Conversion is `normalized = numeric × source-unit factor × gain + offset`. Offset is
in normalized units. Gain/offset are explicit calibration, included in identity.

Time and frequency must increase strictly; frequency must be positive. Voltage/current/
resistance/ratio sweeps can explicitly set `axis_order: "decreasing"`. Data is never
automatically sorted. For non-monotonic I–V trajectories choose acquisition time as the
axis and voltage/current as signals; do not sort a hysteresis loop by voltage.

Missing mapped values fail by default. Explicit `missing: "skip_row"` skips the entire
record and records its number/line/reason; arrays cannot become misaligned. Blank records
are skipped with a recorded reason. Missing/unselected annotation columns do not substitute
zeros into mapped signals. No interpolation occurs during import.

## Comparison semantics

The comparison mapping identifies signal names from each imported dataset, not CSV captions.
Both axes and paired signals must have identical normalized units. Values use scalar real
arithmetic: this is not complex-vector comparison or circular phase unwrapping.

Reference values are interpolated at each data-axis position. `linear` interpolates along
the physical axis. `log_axis` is explicit and requires positive frequency; it interpolates
the selected scalar values along log frequency, not complex phase or decibel transforms.
`reference_axis_shift` explicitly shifts the reference in axis units; default zero.
There is no automatic alignment or fitted delay. Exact reference samples are unchanged.

Optional `window: [start, stop]` includes its endpoints and records every excluded point.
Optional positive `max_reference_gap` forbids interpolating across larger axis gaps, in
axis units. Without it, interpolation assumes continuity between retained samples,
including around explicitly skipped reference rows. Use a gap bound for interrupted captures.

`coverage: "require_full"` is the default: every in-window data point needs reference
coverage. No extrapolation occurs. Explicit `overlap_only` retains uncovered points with
null prediction/residual and a reason; metrics use matched points only and show all counts.
No overlap is an error, not a zero-error result.

Residual is **reference/prediction minus observation**. Bias is mean signed residual;
MAE is mean absolute residual; RMSE is root mean squared residual; maximum absolute error
is the largest magnitude. All use the signal unit, not percent of near-zero data.
These are sample-weighted summaries, not time-integrated errors or statistical confidence
intervals. Every original retained data point and CSV record number stays in the full report.

## Provenance, privacy and bounds

`kessetsu.research-data.v1` stores original CSV text, exact UTF-8 SHA-256, import mapping,
metadata, normalized values, source records and skipped rows. Validation reconstructs
values from raw text/mapping and rejects altered arrays/counts. Comparison records both
dataset identities and its complete mapping/settings. Keep both dataset JSON files and
comparison mapping with a report so another user can repeat it.

Origins are `measured`, `published_simulation`, `simulation`, `synthetic` or `unspecified`
(default). `simulation` is assigned by the typed current-simulation projection and may also
describe an explicitly imported simulator export; it is distinct from a published reference.
Origin/device/sample/citation are **user declarations**, not verified attestations. Hashes
detect changes, not deliberate rewriting by an owner. Exported dataset JSON contains raw
data and potentially private device/capture metadata; review before sharing it.

Limits: 8 MiB CSV; 64 columns; 64 KiB per field; 100,000 data records; one million
normalized values; 1–32 mapped signals; 128 explicit preamble records. Comparisons allow
1–16 signal pairs and one million residual points. Maps are capped at 64 KiB in CLI;
imported JSON at 64 MiB. Oversized or invalid inputs fail without a success artifact.
