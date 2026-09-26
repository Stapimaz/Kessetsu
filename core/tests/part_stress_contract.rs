use kessetsu_core::compiler::{CompileOptions, compile_source};
use kessetsu_core::ir::{Analysis, PartRatingKind, SIUnit};
use kessetsu_core::simulation::{
    AnalysisDataset, Dataset, RealSeriesDataset, SeriesAxis, SimulationResult, SimulationStatus,
    SimulatorInfo, SimulatorLog, SimulatorProcessStatus,
};
use kessetsu_core::stress::{
    PART_STRESS_DISCLAIMER, PART_STRESS_SCHEMA_VERSION, PartStressStatus, evaluate_part_stress,
};
use std::collections::BTreeMap;

const SOURCE: &str = "net GND\nnet OUT\nsource V1 10V\nresistor R1 100Ohm\npart R1 manufacturer=\"Example\" mpn=\"R-100\" peak_voltage_limit=\"12V\" peak_voltage_conditions=\"Across p1-p2 at 25 C\" peak_current_limit=\"90mA\" peak_current_conditions=\"Continuous at 25 C\" average_dissipation_limit=\"1.2W\" average_dissipation_conditions=\"Free air at 25 C\" rating_source=\"User-supplied example record\"\nconnect V1.plus,R1.p1 to OUT\nconnect V1.minus,R1.p2 to GND\nsimulate tran 1ms 2ms\n";

fn simulation() -> SimulationResult {
    let analysis = Analysis::Transient {
        step: kessetsu_core::ir::Quantity {
            value: 1e-3,
            unit: SIUnit::Second,
        },
        stop: kessetsu_core::ir::Quantity {
            value: 2e-3,
            unit: SIUnit::Second,
        },
        use_initial_conditions: false,
    };
    SimulationResult {
        schema_version: "kessetsu.simulation.v1".into(),
        status: SimulationStatus::Succeeded,
        analyses: vec![analysis.clone()],
        simulator: SimulatorInfo {
            executable: "fixture".into(),
            version: "fixture-1".into(),
        },
        process: SimulatorProcessStatus {
            exit_code: Some(0),
            success: true,
        },
        measurements: BTreeMap::new(),
        datasets: vec![AnalysisDataset {
            index: 0,
            analysis,
            data: Dataset::Transient(RealSeriesDataset {
                axis: SeriesAxis {
                    name: "time".into(),
                    values: vec![0.0, 1e-3, 2e-3],
                },
                signals: BTreeMap::from([("out".into(), vec![10.0, 10.0, 10.0])]),
            }),
        }],
        diagnostics: Vec::new(),
        warnings: Vec::new(),
        errors: Vec::new(),
        raw_log: SimulatorLog {
            stdout: String::new(),
            stderr: String::new(),
        },
        artifacts: Vec::new(),
    }
}

#[test]
fn typed_part_ratings_are_preserved_and_compared_without_becoming_assertions() {
    let report = compile_source(SOURCE, CompileOptions::default());
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let circuit = report.ir.unwrap();
    assert!(circuit.assertions.is_empty());
    let part = &circuit.physical_parts.assignments[0];
    assert_eq!(part.ratings.len(), 3);
    assert_eq!(part.ratings[0].kind, PartRatingKind::PeakVoltage);
    assert_eq!(part.ratings[0].limit.unit, SIUnit::Volt);
    assert_eq!(
        part.ratings[0].source.as_deref(),
        Some("User-supplied example record")
    );

    let stress = evaluate_part_stress(&circuit, &simulation());
    assert_eq!(stress.schema_version, PART_STRESS_SCHEMA_VERSION);
    assert_eq!(stress.disclaimer, PART_STRESS_DISCLAIMER);
    assert_eq!(stress.summary.total, 3);
    assert_eq!(stress.summary.within_provided_limit, 2);
    assert_eq!(stress.summary.exceeds_provided_limit, 1);
    assert_eq!(stress.summary.unavailable, 0);
    let current = stress
        .results
        .iter()
        .find(|result| result.rating == PartRatingKind::PeakCurrent)
        .unwrap();
    assert_eq!(current.status, PartStressStatus::ExceedsProvidedLimit);
    assert!((current.actual.unwrap() - 0.1).abs() < 1e-12);
    assert!((current.utilization_percent.unwrap() - 111.111_111).abs() < 1e-5);
}

#[test]
fn ratings_require_positive_typed_limits_and_explicit_conditions() {
    for (field, source) in [
        (
            "peak_voltage_conditions",
            "resistor R1 1k\npart R1 peak_voltage_limit=\"10V\"\n",
        ),
        (
            "peak_current_limit",
            "resistor R1 1k\npart R1 peak_current_limit=\"2V\" peak_current_conditions=\"25 C\"\n",
        ),
        (
            "peak_voltage_limit",
            "resistor R1 1k\npart R1 peak_voltage_limit=\"0V\" peak_voltage_conditions=\"25 C\"\n",
        ),
        (
            "rating_source",
            "resistor R1 1k\npart R1 rating_source=\"orphan citation\"\n",
        ),
    ] {
        let report = compile_source(source, CompileOptions::default());
        let diagnostic = report
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "KES-C024")
            .unwrap_or_else(|| panic!("missing rating diagnostic for {field}"));
        assert_eq!(diagnostic.field.as_deref(), Some(field));
    }
}
