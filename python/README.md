# Kessetsu for Python

This package is a thin, local adapter for the Kessetsu CLI. It launches `kess` without a
shell, validates versioned JSON contracts and exposes simulation, study and research-data
artifacts as typed Python objects and optional pandas DataFrames.

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
`KessetsuClient`. See `examples/notebooks/research-data-workflow.ipynb` and the public
Python/Jupyter guide for the complete measured/reference comparison workflow.
