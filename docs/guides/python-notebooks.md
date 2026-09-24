# Python and Jupyter

Available in development source, not published 1.2.0 packages. The optional Python package
provides typed, local access to Kessetsu simulations, parameter studies and research-data
evidence. It calls the installed `kess` CLI; it is not another simulator or circuit language.

## Install

Install the CLI first and confirm `kess --version`. From a source checkout or extracted
release bundle, install the lightweight adapter:

```sh
python -m pip install ./python
```

For pandas, plots and JupyterLab, open the
[complete workflow notebook](../../examples/notebooks/research-data-workflow.ipynb):

```sh
python -m pip install "./python[notebook]"
jupyter lab examples/notebooks/research-data-workflow.ipynb
```

The adapter finds `kess` on `PATH`. Set `KESSETSU_CLI` to a trusted full executable path,
or pass `KessetsuClient(executable=...)`, when it is installed elsewhere. `KESSETSU_CLI`
selects the Kessetsu executable; `KESSETSU_NGSPICE` independently selects the simulator.
Neither value is executed through a shell.

## Inspect a simulation

```python
from kessetsu import KessetsuClient

client = KessetsuClient()
run = client.simulate_file("examples/rc_low_pass.kess")
table = run.simulation.table(0)
frame = table.to_pandas()

print(frame.head())
print(frame.attrs["units"])
print(frame.attrs["provenance"])
```

Transient and DC datasets become real-valued tables. AC signals remain Python complex
numbers; the adapter does not silently choose magnitude, phase or decibels. Operating point
results become a one-row table. Units and simulator/analysis provenance are stored separately
from column names and copied to `DataFrame.attrs`.

The client writes temporary SPICE/lock artifacts outside the circuit directory and removes
them after the call. Source-relative external model files still resolve from the `.kess`
file. Failed commands raise `KessetsuCommandError` with `exit_code`, parsed `report` and
`stderr`, so diagnostics and failed assertions are not lost.

## Compare CSV data with a simulation

The complete notebook uses these four Core-owned operations:

1. Run `examples/research/rc-step.kess`.
2. Import the explicitly synthetic scope-style CSV with its unit mapping.
3. Convert the typed simulation into `kessetsu.research-data.v1`.
4. Compare both datasets with the ordinary Kessetsu interpolation/residual contract.

```python
import json
from pathlib import Path
from kessetsu import KessetsuClient

root = Path("examples/research")
read = lambda path: json.loads(path.read_text(encoding="utf-8"))
client = KessetsuClient()

run = client.simulate_file(root / "rc-step.kess")
observed = client.import_csv(
    root / "scope-style.csv",
    read(root / "scope-style.kessimport.json"),
)
reference = client.simulation_to_research_data(
    run,
    read(root / "rc-step.kesssim.json"),
)
comparison = client.compare(
    observed,
    reference,
    read(root / "rc-simulation.kesscompare.json"),
)

print(comparison.metrics("out"))
residuals = comparison.table("out").to_pandas()
```

Python does not calculate the normalized dataset, interpolation or error metrics in this
workflow. The adapter passes explicit mappings to CLI commands backed by the shared Rust
Core and then reads their versioned artifacts. Provide `output=` to retain `.kessdata.json`
and `.kesscompare.json`; existing evidence is protected unless `force=True` is explicit.

Origins such as `measured`, `published_simulation`, `simulation` and `synthetic` are user
metadata, not attestations. The example intentionally says `synthetic`; replace it with a
real capture and truthful device/sample/citation fields before making measurement claims.

## Inspect a parameter study

Run or load the same full result used by the CLI and Web study workflow:

```python
from kessetsu import KessetsuClient, load_study_results

client = KessetsuClient()
results = client.run_study(
    "examples/studies/loaded-filter.kessstudy.json",
    "filter-results.json",
)
# Or: results = load_study_results("filter-results.json")

cases = results.measurements_table().to_pandas()
print(cases[["case_id", "status", "measurement.cutoff"]])
```

The table retains every case status, raw parameter literal, measurement value and measurement
error. Failed/cancelled cases do not disappear. Use `results.simulation(case_id).table(index)`
to inspect the full typed dataset retained for a completed case.

## Contracts and privacy

- Exact known schema names are required; future/unknown contracts fail rather than being
  guessed compatible.
- The core package has no third-party Python dependency. pandas/Jupyter are optional extras.
- CLI calls use argument arrays with `shell=False`, bounded process timeouts and isolated
  temporary output directories.
- The adapter performs structural checks and verifies retained raw-CSV SHA-256. Core remains
  authoritative for full dataset identity, unit mapping and comparison validation.
- Files stay local. A notebook can still contain private source/model paths or captured data;
  review executed notebooks and `.kessdata.json` before sharing them.
- Simulation and fitted-model evidence does not by itself establish physical hardware validity.

The Python package version follows the Kessetsu product version. Use the CLI report's
`domain_versions` and each artifact's `schema_version` for machine compatibility, rather
than parsing display text or assuming every product release changes every contract.
