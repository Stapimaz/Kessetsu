"""Safe subprocess adapter for the local Kessetsu CLI."""

from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
from typing import Any, Mapping, Sequence

from .contracts import (
    CLI_SCHEMA,
    DataComparison,
    FitResults,
    KessetsuCommandError,
    KessetsuContractError,
    KessetsuNotFoundError,
    KessetsuTimeoutError,
    ResearchData,
    RunReport,
    SimulationResult,
    StudyResults,
    dump_json,
    load_comparison,
    load_fit_results,
    load_research_data,
    load_study_results,
)


class KessetsuClient:
    """Launch a local `kess` executable and validate its versioned JSON output."""

    def __init__(self, executable: str | os.PathLike[str] | None = None, *, timeout: float = 120.0) -> None:
        requested = os.fspath(executable) if executable is not None else os.environ.get("KESSETSU_CLI", "kess")
        candidate = Path(requested).expanduser()
        if candidate.parent != Path(".") or candidate.is_absolute():
            if not candidate.is_file():
                raise KessetsuNotFoundError(f"Kessetsu CLI was not found at '{candidate}'")
            resolved = str(candidate.resolve())
        else:
            found = shutil.which(requested)
            if found is None:
                raise KessetsuNotFoundError(
                    "Kessetsu CLI was not found; install it, set KESSETSU_CLI, or pass executable="
                )
            resolved = found
        if timeout <= 0:
            raise ValueError("timeout must be positive")
        self.executable = resolved
        self.timeout = float(timeout)

    def version(self) -> str:
        try:
            completed = subprocess.run(
                [self.executable, "--version"],
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=self.timeout,
                check=False,
                shell=False,
            )
        except OSError as error:
            raise KessetsuNotFoundError(f"Could not launch Kessetsu CLI: {error}") from error
        except subprocess.TimeoutExpired as error:
            raise KessetsuTimeoutError("Kessetsu version probe timed out") from error
        output = completed.stdout.strip()
        if completed.returncode != 0 or not output.startswith("kess "):
            raise KessetsuContractError("Executable did not return a Kessetsu version")
        return output.removeprefix("kess ")

    def check_file(self, source: str | os.PathLike[str], *, parameters: Mapping[str, str] | None = None) -> RunReport:
        report = self._run([*self._parameter_args(parameters), "check", str(Path(source).resolve())], "check")
        return RunReport.from_mapping(report, "check")

    def simulate_file(
        self,
        source: str | os.PathLike[str],
        *,
        parameters: Mapping[str, str] | None = None,
    ) -> RunReport:
        with tempfile.TemporaryDirectory(prefix="kessetsu-python-") as directory:
            output = Path(directory) / "simulation.spice"
            report = self._run(
                [
                    "--include",
                    "simulation",
                    *self._parameter_args(parameters),
                    "simulate",
                    str(Path(source).resolve()),
                    "--output",
                    str(output),
                ],
                "simulate",
            )
        parsed = RunReport.from_mapping(report, "simulate")
        if parsed.simulation is None:
            raise KessetsuContractError("Simulation report omitted debug.simulation")
        return parsed

    def test_file(
        self,
        source: str | os.PathLike[str],
        *,
        requirements: str | os.PathLike[str] | None = None,
        parameters: Mapping[str, str] | None = None,
    ) -> RunReport:
        with tempfile.TemporaryDirectory(prefix="kessetsu-python-") as directory:
            args = ["--include", "simulation", *self._parameter_args(parameters), "test", str(Path(source).resolve())]
            if requirements is not None:
                args.extend(["--requirements", str(Path(requirements).resolve())])
            args.extend(["--output", str(Path(directory) / "test.spice")])
            report = self._run(args, "test")
        return RunReport.from_mapping(report, "test")

    def plan_study(self, specification: str | os.PathLike[str]) -> Mapping[str, Any]:
        report = self._run(["study", "plan", str(Path(specification).resolve())], "study")
        study = report.get("study")
        if not isinstance(study, dict):
            raise KessetsuContractError("Study plan report omitted the study object")
        return study

    def run_study(
        self,
        specification: str | os.PathLike[str],
        output: str | os.PathLike[str],
        *,
        resume: bool = False,
    ) -> StudyResults:
        target = Path(output).resolve()
        args = ["study", "run", str(Path(specification).resolve()), "--output", str(target)]
        if resume:
            args.append("--resume")
        self._run(args, "study")
        return load_study_results(target)

    def import_csv(
        self,
        csv_file: str | os.PathLike[str],
        mapping: Mapping[str, Any],
        *,
        output: str | os.PathLike[str] | None = None,
        force: bool = False,
    ) -> ResearchData:
        with tempfile.TemporaryDirectory(prefix="kessetsu-python-") as directory:
            root = Path(directory)
            mapping_path = root / "mapping.json"
            dump_json(mapping_path, mapping)
            target = Path(output).resolve() if output is not None else root / "dataset.json"
            args = ["data", "import", str(Path(csv_file).resolve()), "--mapping", str(mapping_path), "--output", str(target)]
            if force:
                args.append("--force")
            self._run(args, "data")
            result = load_research_data(target)
        return result

    def simulation_to_research_data(
        self,
        simulation: SimulationResult | RunReport,
        mapping: Mapping[str, Any],
        *,
        output: str | os.PathLike[str] | None = None,
        force: bool = False,
    ) -> ResearchData:
        result = simulation.simulation if isinstance(simulation, RunReport) else simulation
        if result is None:
            raise KessetsuContractError("Run report has no typed simulation result")
        with tempfile.TemporaryDirectory(prefix="kessetsu-python-") as directory:
            root = Path(directory)
            simulation_path = root / "simulation.json"
            mapping_path = root / "mapping.json"
            dump_json(simulation_path, result.raw)
            dump_json(mapping_path, mapping)
            target = Path(output).resolve() if output is not None else root / "dataset.json"
            args = [
                "data",
                "from-simulation",
                str(simulation_path),
                "--mapping",
                str(mapping_path),
                "--output",
                str(target),
            ]
            if force:
                args.append("--force")
            self._run(args, "data")
            projected = load_research_data(target)
        return projected

    def compare(
        self,
        observed: ResearchData,
        reference: ResearchData,
        mapping: Mapping[str, Any],
        *,
        output: str | os.PathLike[str] | None = None,
        force: bool = False,
    ) -> DataComparison:
        with tempfile.TemporaryDirectory(prefix="kessetsu-python-") as directory:
            root = Path(directory)
            observed_path = root / "observed.json"
            reference_path = root / "reference.json"
            mapping_path = root / "mapping.json"
            dump_json(observed_path, observed.raw)
            dump_json(reference_path, reference.raw)
            dump_json(mapping_path, mapping)
            target = Path(output).resolve() if output is not None else root / "comparison.json"
            args = [
                "data",
                "compare",
                str(observed_path),
                "--reference",
                str(reference_path),
                "--mapping",
                str(mapping_path),
                "--output",
                str(target),
            ]
            if force:
                args.append("--force")
            self._run(args, "data")
            comparison = load_comparison(target)
        return comparison

    def evaluate_fit(
        self,
        study: StudyResults | str | os.PathLike[str],
        specification: Mapping[str, Any],
        datasets: Mapping[str, ResearchData | str | os.PathLike[str]],
        *,
        output: str | os.PathLike[str] | None = None,
        force: bool = False,
    ) -> FitResults:
        """Evaluate a finite Core study against explicit calibration/validation datasets."""
        if not datasets:
            raise ValueError("datasets must contain at least one named research dataset")
        with tempfile.TemporaryDirectory(prefix="kessetsu-python-") as directory:
            root = Path(directory)
            if isinstance(study, StudyResults):
                study_path = root / "study-results.json"
                dump_json(study_path, study.raw)
            else:
                study_path = Path(study).resolve()
            specification_path = root / "fit-spec.json"
            dump_json(specification_path, specification)
            bindings: list[str] = []
            for index, (name, dataset) in enumerate(datasets.items()):
                if not name or "=" in name:
                    raise ValueError("dataset names must be non-empty and cannot contain '='")
                if isinstance(dataset, ResearchData):
                    path = root / f"dataset-{index + 1}.json"
                    dump_json(path, dataset.raw)
                else:
                    path = Path(dataset).resolve()
                bindings.extend(["--data", f"{name}={path}"])
            target = Path(output).resolve() if output is not None else root / "fit-result.json"
            args = [
                "fit",
                "evaluate",
                str(study_path),
                "--spec",
                str(specification_path),
                *bindings,
                "--output",
                str(target),
            ]
            if force:
                args.append("--force")
            self._run(args, "fit")
            result = load_fit_results(target)
        return result

    @staticmethod
    def _parameter_args(parameters: Mapping[str, str] | None) -> list[str]:
        args: list[str] = []
        for name, value in (parameters or {}).items():
            if not name or "=" in name or not isinstance(value, str) or not value:
                raise ValueError("parameters must contain non-empty NAME: VALUE strings")
            args.extend(["--param", f"{name}={value}"])
        return args

    def _run(self, arguments: Sequence[str], expected_command: str) -> Mapping[str, Any]:
        command = [self.executable, "--format", "json", *arguments]
        try:
            completed = subprocess.run(
                command,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=self.timeout,
                check=False,
                shell=False,
            )
        except OSError as error:
            raise KessetsuNotFoundError(f"Could not launch Kessetsu CLI: {error}") from error
        except subprocess.TimeoutExpired as error:
            raise KessetsuTimeoutError(
                f"Kessetsu command exceeded the {self.timeout:g} second process timeout"
            ) from error
        report: Mapping[str, Any] | None
        try:
            parsed = json.loads(completed.stdout)
            report = parsed if isinstance(parsed, dict) else None
        except json.JSONDecodeError:
            report = None
        if report is not None:
            if report.get("schema_version") != CLI_SCHEMA:
                raise KessetsuContractError(
                    f"CLI returned {report.get('schema_version')!r}; expected {CLI_SCHEMA!r}"
                )
            if report.get("command") != expected_command:
                raise KessetsuContractError(
                    f"CLI returned command {report.get('command')!r}; expected {expected_command!r}"
                )
        if completed.returncode != 0:
            diagnostics = report.get("diagnostics", []) if report is not None else []
            message = next(
                (
                    item.get("message")
                    for item in diagnostics
                    if isinstance(item, dict) and isinstance(item.get("message"), str)
                ),
                f"Kessetsu exited with code {completed.returncode}",
            )
            raise KessetsuCommandError(
                message,
                exit_code=completed.returncode,
                report=report,
                stderr=completed.stderr,
            )
        if report is None:
            raise KessetsuContractError("Kessetsu stdout was not one JSON object")
        return report
