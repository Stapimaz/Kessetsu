"""Versioned Kessetsu artifact readers and tabular views.

This module deliberately performs presentation-only transformations. Electrical calculations,
simulation projection, interpolation and residual metrics stay in Kessetsu Core.
"""

from __future__ import annotations

from dataclasses import dataclass
from hashlib import sha256
import json
import math
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence

CLI_SCHEMA = "kessetsu.cli.v1"
SIMULATION_SCHEMA = "kessetsu.simulation.v1"
DATA_SCHEMA = "kessetsu.research-data.v1"
COMPARISON_SCHEMA = "kessetsu.data-comparison.v1"
EXPERIMENT_RESULTS_SCHEMA = "kessetsu.experiment-results.v1"
FIT_RESULT_SCHEMA = "kessetsu.fit-result.v1"


class KessetsuError(RuntimeError):
    """Base exception for adapter, process and contract failures."""


class KessetsuNotFoundError(KessetsuError):
    """The local Kessetsu CLI could not be found or executed."""


class KessetsuTimeoutError(KessetsuError):
    """The local Kessetsu CLI exceeded the caller's process timeout."""


class KessetsuContractError(KessetsuError):
    """A CLI or artifact schema is missing, malformed or unsupported."""


class KessetsuCommandError(KessetsuError):
    """Kessetsu completed with a non-zero documented exit code."""

    def __init__(
        self,
        message: str,
        *,
        exit_code: int,
        report: Mapping[str, Any] | None,
        stderr: str,
    ) -> None:
        super().__init__(message)
        self.exit_code = exit_code
        self.report = report
        self.stderr = stderr


def _mapping(value: Any, label: str) -> Mapping[str, Any]:
    if not isinstance(value, dict):
        raise KessetsuContractError(f"{label} must be a JSON object")
    return value


def _sequence(value: Any, label: str) -> Sequence[Any]:
    if not isinstance(value, list):
        raise KessetsuContractError(f"{label} must be a JSON array")
    return value


def _text(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise KessetsuContractError(f"{label} must be a non-empty string")
    return value


def _number(value: Any, label: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise KessetsuContractError(f"{label} must be numeric")
    result = float(value)
    if not math.isfinite(result):
        raise KessetsuContractError(f"{label} must be finite")
    return result


def _numbers(value: Any, label: str) -> tuple[float, ...]:
    return tuple(_number(item, f"{label}[{index}]") for index, item in enumerate(_sequence(value, label)))


def _load(path: str | Path) -> Mapping[str, Any]:
    artifact = Path(path)
    try:
        value = json.loads(artifact.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise KessetsuContractError(f"Could not read JSON artifact '{artifact}': {error}") from error
    return _mapping(value, str(artifact))


def _schema(value: Mapping[str, Any], expected: str, label: str) -> None:
    actual = value.get("schema_version")
    if actual != expected:
        raise KessetsuContractError(f"{label} uses {actual!r}; expected {expected!r}")


def _vector_unit(name: str) -> str:
    upper = name.strip().upper()
    if upper.startswith("I(") or upper.endswith("#BRANCH"):
        return "Ampere"
    return "Volt"


@dataclass(frozen=True)
class Table:
    """Dependency-free table with optional pandas conversion and explicit unit metadata."""

    columns: tuple[str, ...]
    rows: tuple[tuple[Any, ...], ...]
    units: Mapping[str, str]
    provenance: Mapping[str, Any]

    def records(self) -> list[dict[str, Any]]:
        return [dict(zip(self.columns, row, strict=True)) for row in self.rows]

    def to_pandas(self):
        """Return a DataFrame, keeping units and provenance in ``DataFrame.attrs``."""
        try:
            import pandas as pd
        except ImportError as error:
            raise KessetsuError(
                "pandas is optional; install Kessetsu with the 'tables' or 'notebook' extra"
            ) from error
        frame = pd.DataFrame.from_records(self.rows, columns=self.columns)
        frame.attrs["units"] = dict(self.units)
        frame.attrs["provenance"] = dict(self.provenance)
        return frame


@dataclass(frozen=True)
class SimulationResult:
    raw: Mapping[str, Any]

    @classmethod
    def from_mapping(cls, value: Mapping[str, Any]) -> "SimulationResult":
        _schema(value, SIMULATION_SCHEMA, "Simulation result")
        if value.get("status") != "succeeded":
            raise KessetsuContractError("Only a successful simulation result has usable datasets")
        _mapping(value.get("simulator"), "simulation.simulator")
        _sequence(value.get("datasets"), "simulation.datasets")
        return cls(value)

    @property
    def simulator(self) -> Mapping[str, Any]:
        return _mapping(self.raw["simulator"], "simulation.simulator")

    @property
    def analyses(self) -> tuple[int, ...]:
        return tuple(int(_mapping(item, "dataset").get("index")) for item in self.raw["datasets"])

    def table(self, analysis_index: int = 0) -> Table:
        selected = next(
            (
                _mapping(item, "simulation dataset")
                for item in self.raw["datasets"]
                if _mapping(item, "simulation dataset").get("index") == analysis_index
            ),
            None,
        )
        if selected is None:
            raise KessetsuContractError(f"Simulation analysis index {analysis_index} was not found")
        analysis = _mapping(selected.get("analysis"), "dataset.analysis")
        data = _mapping(selected.get("data"), "dataset.data")
        kind = _text(data.get("kind"), "dataset.data.kind")
        provenance = {
            "schema_version": SIMULATION_SCHEMA,
            "analysis_index": analysis_index,
            "analysis": dict(analysis),
            "simulator": dict(self.simulator),
        }
        if kind == "operating_point":
            values = _mapping(data.get("values"), "operating-point values")
            columns = tuple(sorted(values))
            return Table(
                columns,
                (tuple(_number(values[name], name) for name in columns),),
                {name: _vector_unit(name) for name in columns},
                provenance,
            )
        if kind in {"transient", "dc_sweep"}:
            axis = _mapping(data.get("axis"), "series axis")
            axis_name = _text(axis.get("name"), "series axis name")
            axis_values = _numbers(axis.get("values"), "series axis values")
            signals = _mapping(data.get("signals"), "series signals")
            names = tuple(sorted(signals))
            vectors = {name: _numbers(signals[name], f"signal {name}") for name in names}
            if any(len(vector) != len(axis_values) for vector in vectors.values()):
                raise KessetsuContractError("Simulation axis and signal lengths do not agree")
            axis_unit = "Second" if kind == "transient" else _text(
                _mapping(analysis.get("start"), "DC start quantity").get("unit"),
                "DC axis unit",
            )
            columns = (axis_name, *names)
            rows = tuple((axis_values[index], *(vectors[name][index] for name in names)) for index in range(len(axis_values)))
            return Table(
                columns,
                rows,
                {axis_name: axis_unit, **{name: _vector_unit(name) for name in names}},
                provenance,
            )
        if kind == "ac":
            axis_values = _numbers(data.get("frequency_hz"), "AC frequency")
            signals = _mapping(data.get("signals"), "AC signals")
            names = tuple(sorted(signals))
            vectors: dict[str, tuple[complex, ...]] = {}
            for name in names:
                vector = _mapping(signals[name], f"AC signal {name}")
                real = _numbers(vector.get("real"), f"AC signal {name}.real")
                imaginary = _numbers(vector.get("imaginary"), f"AC signal {name}.imaginary")
                if len(real) != len(axis_values) or len(imaginary) != len(axis_values):
                    raise KessetsuContractError("AC frequency and signal lengths do not agree")
                vectors[name] = tuple(complex(a, b) for a, b in zip(real, imaginary, strict=True))
            columns = ("frequency", *names)
            rows = tuple((axis_values[index], *(vectors[name][index] for name in names)) for index in range(len(axis_values)))
            return Table(
                columns,
                rows,
                {"frequency": "Hertz", **{name: _vector_unit(name) for name in names}},
                {**provenance, "complex_representation": "python_complex"},
            )
        raise KessetsuContractError(f"Unsupported simulation dataset kind {kind!r}")


@dataclass(frozen=True)
class RunReport:
    raw: Mapping[str, Any]
    simulation: SimulationResult | None

    @classmethod
    def from_mapping(cls, value: Mapping[str, Any], expected_command: str | None = None) -> "RunReport":
        _schema(value, CLI_SCHEMA, "CLI report")
        if expected_command is not None and value.get("command") != expected_command:
            raise KessetsuContractError(
                f"CLI returned command {value.get('command')!r}; expected {expected_command!r}"
            )
        debug = value.get("debug")
        simulation = None
        if isinstance(debug, dict) and isinstance(debug.get("simulation"), dict):
            simulation = SimulationResult.from_mapping(debug["simulation"])
        return cls(value, simulation)


@dataclass(frozen=True)
class ResearchData:
    raw: Mapping[str, Any]

    @classmethod
    def from_mapping(cls, value: Mapping[str, Any]) -> "ResearchData":
        _schema(value, DATA_SCHEMA, "Research dataset")
        _text(value.get("identity"), "research-data identity")
        raw_csv = _text(value.get("raw_csv"), "research-data raw_csv")
        expected_hash = _text(value.get("raw_sha256"), "research-data raw_sha256")
        actual_hash = f"sha256:{sha256(raw_csv.encode('utf-8')).hexdigest()}"
        if expected_hash != actual_hash:
            raise KessetsuContractError("Research-data raw CSV SHA-256 does not match")
        axis = _mapping(value.get("axis"), "research-data axis")
        axis_values = _numbers(axis.get("values"), "research-data axis values")
        signals = _sequence(value.get("signals"), "research-data signals")
        if not signals:
            raise KessetsuContractError("Research dataset must contain at least one signal")
        for item in signals:
            signal = _mapping(item, "research-data signal")
            if len(_numbers(signal.get("values"), "research-data signal values")) != len(axis_values):
                raise KessetsuContractError("Research-data axis and signal lengths do not agree")
        return cls(value)

    @property
    def identity(self) -> str:
        return str(self.raw["identity"])

    def table(self) -> Table:
        axis = _mapping(self.raw["axis"], "research-data axis")
        axis_name = _text(axis.get("name"), "research-data axis name")
        axis_values = _numbers(axis.get("values"), "research-data axis values")
        signals = tuple(_mapping(item, "research-data signal") for item in self.raw["signals"])
        names = tuple(_text(signal.get("name"), "research-data signal name") for signal in signals)
        vectors = tuple(_numbers(signal.get("values"), f"signal {name}") for name, signal in zip(names, signals, strict=True))
        rows = tuple((axis_values[index], *(vector[index] for vector in vectors)) for index in range(len(axis_values)))
        spec = _mapping(self.raw.get("spec"), "research-data spec")
        metadata = _mapping(spec.get("metadata"), "research-data metadata")
        return Table(
            (axis_name, *names),
            rows,
            {
                axis_name: _text(axis.get("unit"), "research-data axis unit"),
                **{
                    name: _text(signal.get("unit"), f"signal {name} unit")
                    for name, signal in zip(names, signals, strict=True)
                },
            },
            {
                "schema_version": DATA_SCHEMA,
                "identity": self.identity,
                "raw_sha256": self.raw["raw_sha256"],
                "origin": metadata.get("origin", "unspecified"),
                "metadata": dict(metadata),
            },
        )


@dataclass(frozen=True)
class DataComparison:
    raw: Mapping[str, Any]

    @classmethod
    def from_mapping(cls, value: Mapping[str, Any]) -> "DataComparison":
        _schema(value, COMPARISON_SCHEMA, "Data comparison")
        _text(value.get("identity"), "comparison identity")
        signals = _sequence(value.get("signals"), "comparison signals")
        if not signals:
            raise KessetsuContractError("Data comparison must contain at least one signal")
        return cls(value)

    def signal_names(self) -> tuple[str, ...]:
        names = []
        for item in self.raw["signals"]:
            mapping = _mapping(_mapping(item, "comparison signal").get("mapping"), "signal mapping")
            names.append(_text(mapping.get("data_signal"), "data signal name"))
        return tuple(names)

    def metrics(self, signal: str | None = None) -> Mapping[str, Any]:
        selected = self._signal(signal)
        return _mapping(selected.get("metrics"), "comparison metrics")

    def table(self, signal: str | None = None) -> Table:
        selected = self._signal(signal)
        mapping = _mapping(selected.get("mapping"), "comparison signal mapping")
        name = _text(mapping.get("data_signal"), "comparison data signal")
        points = _sequence(selected.get("points"), "comparison points")
        rows = []
        for item in points:
            point = _mapping(item, "comparison point")
            rows.append(
                (
                    int(point.get("source_record")),
                    _number(point.get("axis"), "comparison axis"),
                    _number(point.get("observed"), "comparison observed"),
                    None if point.get("predicted") is None else _number(point["predicted"], "comparison predicted"),
                    None if point.get("residual") is None else _number(point["residual"], "comparison residual"),
                    _text(point.get("status"), "comparison point status"),
                )
            )
        axis_unit = _text(self.raw.get("axis_unit"), "comparison axis unit")
        signal_unit = _text(selected.get("unit"), "comparison signal unit")
        return Table(
            ("source_record", "axis", "observed", "predicted", "residual", "status"),
            tuple(rows),
            {"axis": axis_unit, "observed": signal_unit, "predicted": signal_unit, "residual": signal_unit},
            {
                "schema_version": COMPARISON_SCHEMA,
                "identity": self.raw["identity"],
                "data_identity": self.raw.get("data_identity"),
                "reference_identity": self.raw.get("reference_identity"),
                "signal": name,
                "mapping": dict(mapping),
                "metrics": dict(self.metrics(name)),
            },
        )

    def _signal(self, name: str | None) -> Mapping[str, Any]:
        signals = tuple(_mapping(item, "comparison signal") for item in self.raw["signals"])
        if name is None:
            return signals[0]
        for signal in signals:
            mapping = _mapping(signal.get("mapping"), "comparison signal mapping")
            if mapping.get("data_signal") == name:
                return signal
        raise KessetsuContractError(f"Comparison signal {name!r} was not found")


@dataclass(frozen=True)
class StudyResults:
    raw: Mapping[str, Any]

    @classmethod
    def from_mapping(cls, value: Mapping[str, Any]) -> "StudyResults":
        _schema(value, EXPERIMENT_RESULTS_SCHEMA, "Study results")
        _mapping(value.get("plan"), "study plan")
        _sequence(value.get("cases"), "study case results")
        _mapping(value.get("summary"), "study summary")
        return cls(value)

    def measurements_table(self) -> Table:
        plan = _mapping(self.raw["plan"], "study plan")
        planned = {
            _text(_mapping(item, "planned case").get("id"), "planned case id"): _mapping(item, "planned case")
            for item in _sequence(plan.get("cases"), "planned cases")
        }
        results = tuple(_mapping(item, "study case result") for item in self.raw["cases"])
        parameter_names = sorted(
            {
                key
                for case in planned.values()
                for key in _mapping(case.get("parameters"), "case parameters")
            }
        )
        measurement_names = sorted(
            {
                key
                for result in results
                for key in _mapping(result.get("measurements"), "case measurements")
            }
        )
        columns = (
            "case_id",
            "status",
            "revision",
            "case_name",
            "temperature_c",
            *(f"parameter.{name}" for name in parameter_names),
            *(f"measurement.{name}" for name in measurement_names),
            *(f"measurement.{name}.error" for name in measurement_names),
        )
        units: dict[str, str] = {"temperature_c": "DegreeCelsius"}
        rows = []
        for result in results:
            case_id = _text(result.get("case_id"), "case result id")
            case = planned.get(case_id)
            if case is None:
                raise KessetsuContractError(f"Study result references unknown case {case_id!r}")
            parameters = _mapping(case.get("parameters"), "case parameters")
            measurements = _mapping(result.get("measurements"), "case measurements")
            values: list[Any] = [
                case_id,
                _text(result.get("status"), "case status"),
                int(case.get("revision")),
                _text(case.get("name"), "case name"),
                _number(case.get("temperature_c"), "case temperature"),
            ]
            values.extend(parameters.get(name) for name in parameter_names)
            for name in measurement_names:
                measurement = measurements.get(name)
                if measurement is None:
                    values.append(None)
                    continue
                item = _mapping(measurement, f"measurement {name}")
                value = item.get("value")
                values.append(None if value is None else _number(value, f"measurement {name} value"))
                unit = item.get("unit")
                if isinstance(unit, str):
                    units[f"measurement.{name}"] = unit
            for name in measurement_names:
                measurement = measurements.get(name)
                values.append(None if measurement is None else _mapping(measurement, f"measurement {name}").get("error"))
            rows.append(tuple(values))
        return Table(
            columns,
            tuple(rows),
            units,
            {
                "schema_version": EXPERIMENT_RESULTS_SCHEMA,
                "identity": self.raw.get("identity"),
                "solver_fingerprint": self.raw.get("solver_fingerprint"),
                "summary": dict(_mapping(self.raw["summary"], "study summary")),
            },
        )

    def simulation(self, case_id: str) -> SimulationResult:
        for item in self.raw["cases"]:
            case = _mapping(item, "study case result")
            if case.get("case_id") == case_id:
                simulation = case.get("simulation")
                if not isinstance(simulation, dict):
                    raise KessetsuContractError(f"Study case {case_id!r} has no simulation result")
                return SimulationResult.from_mapping(simulation)
        raise KessetsuContractError(f"Study case {case_id!r} was not found")


@dataclass(frozen=True)
class FitResults:
    raw: Mapping[str, Any]

    @classmethod
    def from_mapping(cls, value: Mapping[str, Any]) -> "FitResults":
        _schema(value, FIT_RESULT_SCHEMA, "Fit results")
        _text(value.get("identity"), "fit identity")
        _text(value.get("experiment_identity"), "fit experiment identity")
        _mapping(value.get("spec"), "fit specification")
        _mapping(value.get("data_identities"), "fit data identities")
        candidates = _sequence(value.get("candidates"), "fit candidates")
        if not candidates:
            raise KessetsuContractError("Fit results must retain at least one candidate")
        for item in candidates:
            candidate = _mapping(item, "fit candidate")
            _text(candidate.get("id"), "fit candidate id")
            _mapping(candidate.get("parameters"), "fit candidate parameters")
            _sequence(candidate.get("observations"), "fit candidate observations")
        return cls(value)

    @property
    def selected_candidate(self) -> str | None:
        value = self.raw.get("selected_candidate")
        if value is None:
            return None
        return _text(value, "selected candidate")

    def candidates_table(self) -> Table:
        candidates = tuple(_mapping(item, "fit candidate") for item in self.raw["candidates"])
        parameter_names = sorted(
            {
                name
                for candidate in candidates
                for name in _mapping(candidate.get("parameters"), "fit candidate parameters")
            }
        )
        columns = (
            "candidate_id",
            "selected",
            "eligible",
            "calibration_score",
            "validation_score",
            "boundary_hits",
            *(f"parameter.{name}" for name in parameter_names),
        )
        rows = []
        for candidate in candidates:
            candidate_id = _text(candidate.get("id"), "fit candidate id")
            parameters = _mapping(candidate.get("parameters"), "fit candidate parameters")
            calibration = candidate.get("calibration_score")
            validation = candidate.get("validation_score")
            rows.append(
                (
                    candidate_id,
                    candidate_id == self.selected_candidate,
                    bool(candidate.get("eligible")),
                    None if calibration is None else _number(calibration, "calibration score"),
                    None if validation is None else _number(validation, "validation score"),
                    ", ".join(str(item) for item in _sequence(candidate.get("boundary_hits"), "boundary hits")),
                    *(parameters.get(name) for name in parameter_names),
                )
            )
        return Table(
            columns,
            tuple(rows),
            {"calibration_score": "normalized_rms", "validation_score": "normalized_rms"},
            {
                "schema_version": FIT_RESULT_SCHEMA,
                "identity": self.raw["identity"],
                "experiment_identity": self.raw["experiment_identity"],
                "selected_candidate": self.selected_candidate,
                "warnings": tuple(self.raw.get("warnings", [])),
            },
        )

    def observations_table(self) -> Table:
        rows = []
        for candidate_value in self.raw["candidates"]:
            candidate = _mapping(candidate_value, "fit candidate")
            candidate_id = _text(candidate.get("id"), "fit candidate id")
            for observation_value in _sequence(candidate.get("observations"), "fit observations"):
                observation = _mapping(observation_value, "fit observation")
                score = observation.get("score")
                comparison = observation.get("comparison")
                comparison_identity = (
                    _mapping(comparison, "fit comparison").get("identity")
                    if comparison is not None
                    else None
                )
                rows.append(
                    (
                        candidate_id,
                        _text(observation.get("name"), "fit observation name"),
                        _text(observation.get("role"), "fit observation role"),
                        observation.get("case_id"),
                        None if score is None else _number(score, "fit observation score"),
                        int(observation.get("scored_points", 0)),
                        observation.get("error"),
                        comparison_identity,
                    )
                )
        return Table(
            (
                "candidate_id",
                "observation",
                "role",
                "case_id",
                "score",
                "scored_points",
                "error",
                "comparison_identity",
            ),
            tuple(rows),
            {"score": "normalized_rms"},
            {"schema_version": FIT_RESULT_SCHEMA, "identity": self.raw["identity"]},
        )

    def comparison(self, candidate_id: str, observation_name: str) -> DataComparison:
        for candidate_value in self.raw["candidates"]:
            candidate = _mapping(candidate_value, "fit candidate")
            if candidate.get("id") != candidate_id:
                continue
            for observation_value in _sequence(candidate.get("observations"), "fit observations"):
                observation = _mapping(observation_value, "fit observation")
                if observation.get("name") == observation_name:
                    comparison = observation.get("comparison")
                    if not isinstance(comparison, dict):
                        raise KessetsuContractError(
                            f"Observation {observation_name!r} has no successful comparison"
                        )
                    return DataComparison.from_mapping(comparison)
            raise KessetsuContractError(f"Fit observation {observation_name!r} was not found")
        raise KessetsuContractError(f"Fit candidate {candidate_id!r} was not found")


def load_research_data(path: str | Path) -> ResearchData:
    return ResearchData.from_mapping(_load(path))


def load_comparison(path: str | Path) -> DataComparison:
    return DataComparison.from_mapping(_load(path))


def load_study_results(path: str | Path) -> StudyResults:
    return StudyResults.from_mapping(_load(path))


def load_fit_results(path: str | Path) -> FitResults:
    return FitResults.from_mapping(_load(path))


def dump_json(path: str | Path, value: Mapping[str, Any]) -> None:
    Path(path).write_text(
        json.dumps(value, indent=2, ensure_ascii=False, allow_nan=False) + "\n",
        encoding="utf-8",
    )
