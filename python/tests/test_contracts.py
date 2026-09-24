from __future__ import annotations

from hashlib import sha256
import json
import tempfile
import unittest
from pathlib import Path

from kessetsu import (
    DataComparison,
    KessetsuContractError,
    ResearchData,
    SimulationResult,
    StudyResults,
)


class ContractTests(unittest.TestCase):
    def test_transient_and_ac_tables_preserve_units_and_complex_values(self) -> None:
        transient = simulation(
            {
                "kind": "transient",
                "axis": {"name": "time", "values": [0.0, 1.0]},
                "signals": {"V(out)": [0.0, 1.0], "v_vin#branch": [0.0, -0.001]},
            },
            {"kind": "transient", "step": {"value": 1.0, "unit": "Second"}, "stop": {"value": 1.0, "unit": "Second"}, "use_initial_conditions": False},
        )
        table = SimulationResult.from_mapping(transient).table(0)
        self.assertEqual(table.columns, ("time", "V(out)", "v_vin#branch"))
        self.assertEqual(table.units["time"], "Second")
        self.assertEqual(table.units["V(out)"], "Volt")
        self.assertEqual(table.units["v_vin#branch"], "Ampere")

        ac = simulation(
            {
                "kind": "ac",
                "frequency_hz": [10.0, 100.0],
                "signals": {"out": {"real": [1.0, 0.0], "imaginary": [0.0, -1.0]}},
            },
            {"kind": "ac", "scale": "decade", "points": 1, "start": {"value": 10.0, "unit": "Hertz"}, "stop": {"value": 100.0, "unit": "Hertz"}},
        )
        ac_table = SimulationResult.from_mapping(ac).table(0)
        self.assertEqual(ac_table.rows[1][1], -1j)
        self.assertEqual(ac_table.provenance["complex_representation"], "python_complex")

    def test_research_data_checks_raw_hash_and_exposes_provenance(self) -> None:
        raw_csv = "time,out\n0,0\n1,1\n"
        value = {
            "schema_version": "kessetsu.research-data.v1",
            "identity": "sha256:dataset",
            "raw_sha256": f"sha256:{sha256(raw_csv.encode()).hexdigest()}",
            "raw_csv": raw_csv,
            "spec": {"metadata": {"origin": "measured", "device": "sample-a"}},
            "axis": {"name": "time", "unit": "Second", "values": [0.0, 1.0]},
            "signals": [{"name": "out", "unit": "Volt", "values": [0.0, 1.0]}],
            "source_records": [2, 3],
            "skipped": [],
            "records_seen": 2,
        }
        dataset = ResearchData.from_mapping(value)
        self.assertEqual(dataset.table().provenance["origin"], "measured")
        damaged = dict(value)
        damaged["raw_csv"] = raw_csv + "2,2\n"
        with self.assertRaisesRegex(KessetsuContractError, "SHA-256"):
            ResearchData.from_mapping(damaged)

    def test_comparison_keeps_unmatched_points_and_metrics(self) -> None:
        value = {
            "schema_version": "kessetsu.data-comparison.v1",
            "identity": "sha256:comparison",
            "data_identity": "sha256:data",
            "reference_identity": "sha256:reference",
            "data_origin": "measured",
            "reference_origin": "simulation",
            "axis_unit": "Second",
            "spec": {},
            "signals": [
                {
                    "mapping": {"data_signal": "out", "reference_signal": "out"},
                    "unit": "Volt",
                    "metrics": {"total": 2, "matched": 1, "excluded_by_window": 0, "unmatched": 1, "bias": 0.1, "mae": 0.1, "rmse": 0.1, "max_absolute": 0.1},
                    "points": [
                        {"source_record": 2, "axis": 0.0, "observed": 0.0, "predicted": 0.1, "residual": 0.1, "status": "matched"},
                        {"source_record": 3, "axis": 1.0, "observed": 1.0, "predicted": None, "residual": None, "status": "outside_reference"},
                    ],
                }
            ],
        }
        report = DataComparison.from_mapping(value)
        table = report.table("out")
        self.assertEqual(len(table.rows), 2)
        self.assertIsNone(table.rows[1][3])
        self.assertEqual(report.metrics("out")["unmatched"], 1)

    def test_study_table_retains_failed_measurements_and_case_parameters(self) -> None:
        value = {
            "schema_version": "kessetsu.experiment-results.v1",
            "identity": "sha256:study",
            "plan": {
                "cases": [
                    {"id": "case-a", "revision": 0, "name": "nominal", "parameters": {"r": "1kOhm"}, "temperature_c": 27.0}
                ]
            },
            "simulator": {"executable": "ngspice", "version": "46"},
            "solver_fingerprint": "sha256:solver",
            "cases": [
                {
                    "case_id": "case-a",
                    "status": "error",
                    "measurements": {"gain": {"value": None, "unit": "Ratio", "error": "missing vector"}},
                    "assertions": None,
                    "simulation": None,
                    "errors": ["missing vector"],
                    "content_sha256": "sha256:case",
                }
            ],
            "summary": {"total": 1, "pending": 0, "completed": 0, "passed": 0, "failed": 0, "errors": 1, "cancelled": 0, "best_case": None},
        }
        table = StudyResults.from_mapping(value).measurements_table()
        record = table.records()[0]
        self.assertEqual(record["parameter.r"], "1kOhm")
        self.assertIsNone(record["measurement.gain"])
        self.assertEqual(record["measurement.gain.error"], "missing vector")


def simulation(data: dict, analysis: dict) -> dict:
    return {
        "schema_version": "kessetsu.simulation.v1",
        "status": "succeeded",
        "analyses": [analysis],
        "simulator": {"executable": "ngspice", "version": "46"},
        "process": {"exit_code": 0, "success": True},
        "measurements": {},
        "datasets": [{"index": 0, "analysis": analysis, "data": data}],
        "diagnostics": [],
        "warnings": [],
        "errors": [],
        "raw_log": {"stdout": "", "stderr": ""},
        "artifacts": [],
    }


if __name__ == "__main__":
    unittest.main()
