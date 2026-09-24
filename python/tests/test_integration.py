from __future__ import annotations

import json
import os
from pathlib import Path
import unittest

from kessetsu import KessetsuClient


@unittest.skipUnless(os.environ.get("KESSETSU_TEST_CLI"), "set KESSETSU_TEST_CLI for native integration")
class NativeIntegrationTests(unittest.TestCase):
    def test_simulation_import_and_comparison_stay_in_core(self) -> None:
        root = Path(__file__).resolve().parents[2]
        client = KessetsuClient(os.environ["KESSETSU_TEST_CLI"])
        run = client.simulate_file(root / "examples/research/rc-step.kess")
        reference = client.simulation_to_research_data(
            run,
            json.loads((root / "examples/research/rc-step.kesssim.json").read_text(encoding="utf-8")),
        )
        observed = client.import_csv(
            root / "examples/research/scope-style.csv",
            json.loads((root / "examples/research/scope-style.kessimport.json").read_text(encoding="utf-8")),
        )
        comparison = client.compare(
            observed,
            reference,
            json.loads((root / "examples/research/rc-simulation.kesscompare.json").read_text(encoding="utf-8")),
        )
        self.assertEqual(comparison.metrics("out")["matched"], 6)
        self.assertLess(comparison.metrics("out")["rmse"], 0.02)


if __name__ == "__main__":
    unittest.main()
