# Finite Parameter Fitting

Kessetsu 1.3.0 can evaluate a completed finite parameter study against explicit calibration
and holdout-validation data.
It selects only among candidates that were actually simulated; it does not run a continuous
optimizer or claim a global optimum.

This workflow is useful when a model exposes meaningful parameters such as resistance,
threshold voltage or a time constant and you have observed curves at more than one operating
condition. The fitting engine remains in Rust Core. CLI and Python are adapters over the same
versioned artifacts.

## Complete example

The repository includes a synthetic loaded-divider example with five resistance candidates,
a one-volt calibration condition and a separate two-volt validation condition:

```sh
kess study run examples/research/divider-fit.kessstudy.json \
  --output divider-study.json
kess data import examples/research/divider-calibration.csv \
  --mapping examples/research/divider-calibration.kessimport.json \
  --output divider-calibration.kessdata.json
kess data import examples/research/divider-validation.csv \
  --mapping examples/research/divider-validation.kessimport.json \
  --output divider-validation.kessdata.json
kess fit evaluate divider-study.json \
  --spec examples/research/divider-fit.kessfit.json \
  --data calibration=divider-calibration.kessdata.json \
  --data validation=divider-validation.kessdata.json \
  --output divider-fit-result.json --format json
```

The sample data is explicitly synthetic and demonstrates the workflow only. It is not
laboratory evidence. The complete executable walkthrough is also available as
[`finite-parameter-fit.ipynb`](../../examples/notebooks/finite-parameter-fit.ipynb).

## Fit specification

A `kessetsu.fit.v1` specification binds fitted parameters to existing study axes and each
observation to exactly one revision, temperature and optional non-fitted condition:

```json
{
  "schema_version": "kessetsu.fit.v1",
  "name": "Synthetic divider resistance fit",
  "parameters": [
    { "name": "resistance", "lower": "500Ohm", "upper": "2kOhm" }
  ],
  "observations": [
    {
      "name": "one-volt calibration",
      "role": "calibration",
      "revision": "Calibration",
      "temperature_c": 27.0,
      "data": "calibration",
      "simulation": {
        "schema_version": "kessetsu.simulation-data-import.v1",
        "name": "Calibration candidate",
        "analysis_index": 0,
        "signals": [{ "vector": "out", "name": "output" }]
      },
      "comparison": {
        "schema_version": "kessetsu.data-comparison.v1",
        "name": "Calibration residuals",
        "signals": [{ "data_signal": "output", "reference_signal": "output" }],
        "coverage": "require_full",
        "interpolation": "linear",
        "window": [0.0, 0.02]
      },
      "signals": [
        { "data_signal": "output", "uncertainty": 0.005, "weight": 1.0 }
      ]
    },
    {
      "name": "two-volt holdout",
      "role": "validation",
      "revision": "Validation",
      "temperature_c": 27.0,
      "data": "validation",
      "simulation": {
        "schema_version": "kessetsu.simulation-data-import.v1",
        "name": "Validation candidate",
        "analysis_index": 0,
        "signals": [{ "vector": "out", "name": "output" }]
      },
      "comparison": {
        "schema_version": "kessetsu.data-comparison.v1",
        "name": "Validation residuals",
        "signals": [{ "data_signal": "output", "reference_signal": "output" }],
        "coverage": "require_full",
        "interpolation": "linear",
        "window": [0.0, 0.02]
      },
      "signals": [
        { "data_signal": "output", "uncertainty": 0.005, "weight": 1.0 }
      ]
    }
  ],
  "near_equivalent_fraction": 0.01
}
```

At least one calibration and one validation observation are required. Each fitted parameter
must be an explicit study axis and have typed lower/upper bounds. Conditions may select other
study axes but cannot secretly constrain a fitted parameter. Every observation selector must
resolve to exactly one case per candidate.

`uncertainty` gives each residual a physically meaningful scale in the signal's normalized SI
unit. `weight` controls the relative contribution of a signal. Optional inclusive `masks`
exclude declared axis ranges from scoring while preserving those residual points in the result.
The ordinary research-data window, coverage and interpolation rules still apply.

## Selection and evidence

For every scored point, Core computes `residual / uncertainty`. A candidate's calibration
score is the weighted root-mean-square of those dimensionless values across all calibration
observations and signals. The lowest eligible calibration score wins; candidate identifiers
break exact ties deterministically.

Validation scores are calculated only after the same candidate results exist and never
influence selection. A failed calibration observation makes its candidate ineligible. A failed
validation observation makes that candidate's validation score unavailable but does not erase
it or retroactively change selection.

The `kessetsu.fit-result.v1` artifact retains:

- every evaluated candidate and resolved parameter value;
- separate calibration and validation scores;
- all successful point-by-point comparisons and declared score masks;
- failed observation messages and original case identifiers;
- experiment, dataset, simulator and solver identities;
- selected-boundary and near-equivalent-candidate warnings.

Default JSON stdout is compact. The output file always contains complete evidence; add
`--include datasets` only when complete residual arrays are also needed in stdout.
Existing outputs require `--force`, and no output may alias the study, specification or a
bound dataset. Invalid fitting input returns exit code `2` with `KES-R002`.

## Python and notebooks

```python
import json
from pathlib import Path
from kessetsu import KessetsuClient

root = Path("examples/research")
read = lambda path: json.loads(path.read_text(encoding="utf-8"))
client = KessetsuClient()

study = client.run_study(root / "divider-fit.kessstudy.json", "study.json")
calibration = client.import_csv(
    root / "divider-calibration.csv",
    read(root / "divider-calibration.kessimport.json"),
)
validation = client.import_csv(
    root / "divider-validation.csv",
    read(root / "divider-validation.kessimport.json"),
)
fit = client.evaluate_fit(
    study,
    read(root / "divider-fit.kessfit.json"),
    {"calibration": calibration, "validation": validation},
)

print(fit.candidates_table().rows)
selected = fit.selected_candidate
residuals = fit.comparison(selected, "one-volt calibration")
```

Python does not recalculate selection, interpolation or scores. It invokes the CLI, validates
the exact result schema and exposes candidate, observation and residual tables.

## External device parameters

Fitting is especially useful for exact local `.SUBCKT` models. A declaration can expose a
bounded, typed allowlist of parameters already present in the model header:

```kessetsu
external_subcircuit MEM threshold_memristor.lib THRESHOLD_MEMRISTOR
  pins="p,n" instance_parameters="Rinit:Ohm,Vt:V"

param initial_resistance: Ohm = 10k
param threshold: V = 1V
device XM MEM (Rinit={initial_resistance}, Vt={threshold})
```

Sweep the root parameters in a study, not arbitrary model text. Exact model hashes, resolved
instance values, circuit inputs and simulator identity then remain attached to the evidence.

## Limits and interpretation

- 1-8 fitted parameters, 2-32 observations and at most 32 bound datasets.
- The fit artifact retains at most two million residual points and 64 MiB.
- Only candidates already present in the completed, checksum-validated study are considered.
- There is no extrapolation, automatic time alignment, Bayesian inference, confidence interval,
  gradient search or continuous/global optimizer.
- Near-equivalent candidates indicate possible non-identifiability on the supplied observations;
  they are not silently collapsed into one answer.
- A boundary selection suggests that wider justified bounds or another model should be examined;
  it does not authorize automatic range expansion.
- Curve agreement does not prove a physical mechanism, parameter uniqueness, hardware safety or
  validity outside the explicit observations. Use independent measurements and engineering review.
