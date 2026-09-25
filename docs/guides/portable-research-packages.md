# Portable Research Packages

A completed parameter study is useful only if another person can identify its inputs, inspect
the complete evidence and rerun it. `kess study package` creates a new self-contained folder
for that purpose without altering the original specification or result.

## Create a package

Run the study first, then name the analysis signal that should appear in the publication plot:

```sh
kess study run examples/memristor_pulse_protocol.kessstudy.json \
  --output memristor-results.json
kess study package examples/memristor_pulse_protocol.kessstudy.json \
  --results memristor-results.json \
  --output memristor-package \
  --signal "V(TOP)"
```

The output directory must not already exist. Kessetsu builds a sibling temporary directory
and publishes it only after every file is written. It never merges into or overwrites an
existing research folder.

The plot automatically uses the first twelve completed cases that contain the requested
signal. Use repeatable `--case CASE_ID` options to select up to twelve exact cases. Select
another dataset with `--analysis N`. Complete results and numeric CSV always retain every
case; plot selection changes only `plot.svg`.

## Folder contents

Every package contains:

| File | Purpose |
|---|---|
| `manifest.json` | Versioned package identity, file hashes, model dependencies, solver identity, plotted cases and rerun commands |
| `study.kessstudy.json` | Authoritative experiment specification, including revision sources |
| `circuit.kess` | Convenient editable copy of the root source |
| `requirements.kessreq` | Exact evaluator-owned requirements, when present |
| `results.json` | Complete result, including failures and retained simulation arrays |
| `summary.csv` | Case/measurement table with units and errors |
| `datasets.csv` | Full numeric case/analysis/axis/signal data |
| `plot.svg` | Deterministic labeled overlay for the requested signal |
| `report.html` | Self-contained printable summary and provenance |
| `README.txt` | Human-readable dependencies, limitations and exact rerun commands |

The manifest uses `kessetsu.research-package.v1`. Every payload file has its byte count,
media type, role and SHA-256. These hashes detect accidental corruption; they are not a
signature or proof that the package owner is independent.

## Model licensing and dependencies

Kessetsu re-plans the original study against the model files beside the supplied specification
and rejects any mismatch with the completed result. An external resource declared
`redistribution=permitted` is copied at its exact source-relative path. Its entry, version,
license, citation/source, simulator compatibility and hash remain in the manifest.

A `redistribution=prohibited` model body is never copied. The manifest and README instead
record the exact path and identity that the recipient must lawfully acquire. After placing the
file there, the same rerun command applies. Declaring redistribution permission remains the
model provider/package author's responsibility; retain all notices required by that license.

## Clean rerun

Copy the complete folder to a machine with a compatible `kess` and Ngspice installation,
open a terminal in that folder, and run the command printed in `README.txt`:

```sh
kess study run study.kessstudy.json --output rerun-results.json
```

Compare identities, case status and numerical values with appropriate tolerances. The stored
solver executable/version/fingerprint explain the original environment; a clean machine is
not expected to have the same path or binary fingerprint. Platform/solver differences should
be measured and reported, not erased by replacing the original evidence.

The package is a reproducibility aid, not an archival standard, cryptographic attestation,
physical-device validation or guarantee that a model is meaningful outside its declared
conditions.
