use netlang_core::ir::{Analysis, CircuitIR, SIUnit, ast_to_ir};
use netlang_core::parse_program;
use netlang_core::sim_result::{
    ASSERTION_SCHEMA_VERSION, AssertionStatus, TolerancePolicy, evaluate_assertions,
    evaluate_assertions_with_policy, format_quantity,
};
use netlang_core::simulation::{
    AnalysisDataset, Dataset, RealSeriesDataset, SeriesAxis, SimulationResult, SimulationStatus,
    SimulatorInfo, SimulatorLog, SimulatorProcessStatus,
};
use std::collections::BTreeMap;

fn circuit(source: &str) -> CircuitIR {
    let program = parse_program(source).expect("assertion fixture should parse");
    ast_to_ir(&program).expect("assertion fixture should reach IR")
}

fn simulation(status: SimulationStatus, data: Dataset) -> SimulationResult {
    SimulationResult {
        schema_version: "netlang.simulation.v1".to_string(),
        status,
        analyses: vec![Analysis::OperatingPoint],
        simulator: SimulatorInfo {
            executable: "fixture".to_string(),
            version: "fixture-1".to_string(),
        },
        process: SimulatorProcessStatus {
            exit_code: Some(0),
            success: status == SimulationStatus::Succeeded,
        },
        measurements: BTreeMap::new(),
        datasets: vec![AnalysisDataset {
            index: 0,
            analysis: Analysis::OperatingPoint,
            data,
        }],
        diagnostics: vec![],
        warnings: vec![],
        errors: vec![],
        raw_log: SimulatorLog {
            stdout: String::new(),
            stderr: String::new(),
        },
        artifacts: vec![],
    }
}

fn transient(signals: BTreeMap<String, Vec<f64>>) -> Dataset {
    Dataset::Transient(RealSeriesDataset {
        axis: SeriesAxis {
            name: "time".to_string(),
            values: vec![0.0, 1.0, 2.0],
        },
        signals,
    })
}

#[test]
fn assertion_report_has_deterministic_codes_statuses_and_summary() {
    let circuit =
        circuit("assert peak(V(out)) < 4V\nassert rms(V(out)) > 3V\nassert max(V(missing)) < 1V\n");
    let simulation = simulation(
        SimulationStatus::Succeeded,
        transient(BTreeMap::from([("out".to_string(), vec![-5.0, 1.0, 3.0])])),
    );

    let report = evaluate_assertions(&circuit, &simulation);
    assert_eq!(report.schema_version, ASSERTION_SCHEMA_VERSION);
    assert_eq!(
        report
            .assertions
            .iter()
            .map(|result| result.code.as_str())
            .collect::<Vec<_>>(),
        ["NL-T001", "NL-T002", "NL-T003"]
    );
    assert_eq!(report.assertions[0].status, AssertionStatus::Fail);
    assert_eq!(report.assertions[0].actual, Some(5.0));
    assert_eq!(report.assertions[1].status, AssertionStatus::Pass);
    assert_eq!(report.assertions[2].status, AssertionStatus::Error);
    assert_eq!(report.assertions[2].actual, None);
    assert!(
        report.assertions[2]
            .message
            .as_deref()
            .is_some_and(|message| message.contains("was not produced"))
    );
    assert_eq!(report.summary.total, 3);
    assert_eq!(report.summary.passed, 1);
    assert_eq!(report.summary.failed, 1);
    assert_eq!(report.summary.errors, 1);
    assert_eq!(report.summary.skipped, 0);
    assert!(!report.all_passed());
}

#[test]
fn failed_simulation_skips_assertions_instead_of_fabricating_failures() {
    let circuit = circuit("assert max(V(out)) < 5V\n");
    let simulation = simulation(
        SimulationStatus::Failed,
        Dataset::OperatingPoint {
            values: BTreeMap::new(),
        },
    );
    let report = evaluate_assertions(&circuit, &simulation);
    assert_eq!(report.assertions[0].status, AssertionStatus::Skipped);
    assert_eq!(report.summary.skipped, 1);
    assert_eq!(report.summary.failed, 0);
}

#[test]
fn equality_and_inclusive_comparators_use_explicit_absolute_relative_tolerance() {
    let circuit = circuit(
        "assert value(V(out)) == 5V\nassert value(V(out)) <= 5V\nassert value(V(out)) >= 5V\n",
    );
    let simulation = simulation(
        SimulationStatus::Succeeded,
        Dataset::OperatingPoint {
            values: BTreeMap::from([("out".to_string(), 5.000004)]),
        },
    );
    let policy = TolerancePolicy {
        absolute: 1e-9,
        relative: 1e-6,
    };
    let report = evaluate_assertions_with_policy(&circuit, &simulation, policy);
    assert!(
        report
            .assertions
            .iter()
            .all(|result| result.status == AssertionStatus::Pass)
    );
}

#[test]
fn peak_is_absolute_while_operating_point_current_preserves_ngspice_sign() {
    let circuit = circuit("source V1 5V\nassert peak(I(V1)) > 2mA\nassert max(I(V1)) < 0A\n");
    let simulation = simulation(
        SimulationStatus::Succeeded,
        Dataset::OperatingPoint {
            values: BTreeMap::from([("v_v1#branch".to_string(), -0.005)]),
        },
    );
    let report = evaluate_assertions(&circuit, &simulation);
    assert_eq!(report.assertions[0].actual, Some(0.005));
    assert_eq!(report.assertions[1].actual, Some(-0.005));
    assert!(report.all_passed());
}

#[test]
fn unsupported_metric_and_complex_ac_reduction_are_typed_errors() {
    let unsupported = circuit("assert settle(V(out)) < 1V\n");
    let simulation = simulation(
        SimulationStatus::Succeeded,
        Dataset::OperatingPoint {
            values: BTreeMap::from([("out".to_string(), 0.0)]),
        },
    );
    assert_eq!(
        evaluate_assertions(&unsupported, &simulation).assertions[0].status,
        AssertionStatus::Error
    );
}

#[test]
fn human_quantity_formatter_keeps_units_and_engineering_prefixes() {
    assert_eq!(format_quantity(0.00321, SIUnit::Volt), "3.210000mV");
    assert_eq!(format_quantity(2.5e-6, SIUnit::Ampere), "2.500000µA");
    assert_eq!(format_quantity(10_000.0, SIUnit::Hertz), "10.000000kHz");
}
