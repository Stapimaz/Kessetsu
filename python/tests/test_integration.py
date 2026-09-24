from __future__ import annotations

import json
import os
from pathlib import Path
import tempfile
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

    def test_finite_fit_keeps_calibration_selection_separate_from_holdout(self) -> None:
        root = Path(__file__).resolve().parents[2]
        examples = root / "examples" / "research"
        client = KessetsuClient(os.environ["KESSETSU_TEST_CLI"])
        with tempfile.TemporaryDirectory(prefix="kessetsu-fit-integration-") as directory:
            study = client.run_study(
                examples / "divider-fit.kessstudy.json",
                Path(directory) / "study-results.json",
            )
            calibration = client.import_csv(
                examples / "divider-calibration.csv",
                json.loads(
                    (examples / "divider-calibration.kessimport.json").read_text(encoding="utf-8")
                ),
            )
            validation = client.import_csv(
                examples / "divider-validation.csv",
                json.loads(
                    (examples / "divider-validation.kessimport.json").read_text(encoding="utf-8")
                ),
            )
            fit = client.evaluate_fit(
                study,
                json.loads((examples / "divider-fit.kessfit.json").read_text(encoding="utf-8")),
                {"calibration": calibration, "validation": validation},
            )
        selected = next(
            row for row in fit.candidates_table().records() if row["selected"]
        )
        self.assertEqual(selected["parameter.resistance"], "1000Ohm")
        self.assertIsNotNone(selected["validation_score"])


if __name__ == "__main__":
    unittest.main()
