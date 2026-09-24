# Kessetsu for Python

This package is a thin, local adapter for the Kessetsu CLI. It launches `kess` without a
shell, validates versioned JSON contracts and exposes simulation, study, research-data and
finite-fit artifacts as typed Python objects and optional pandas DataFrames.

It does not implement a second circuit language, simulator, measurement engine or fitting
engine. Circuit and comparison semantics stay in the shared Rust Core.

```bash
python -m pip install ./python
# Optional notebook stack:
python -m pip install "./python[notebook]"
```

```python
from kessetsu import KessetsuClient

client = KessetsuClient()
run = client.simulate_file("examples/rc_low_pass.kess")
frame = run.simulation.table(0).to_pandas()
print(frame.attrs["units"])
```

The `kess` executable must be on `PATH`, named by `KESSETSU_CLI`, or supplied explicitly to
`KessetsuClient`. See `examples/notebooks/research-data-workflow.ipynb` for the complete
measured/reference comparison workflow and `examples/notebooks/finite-parameter-fit.ipynb`
for calibration-only candidate selection with separate holdout validation. Python does not
recalculate fitting scores or selection; those remain in Rust Core.
