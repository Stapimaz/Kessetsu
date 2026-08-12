use netlang_core::ir::{Analysis, CircuitIR, SIUnit, ast_to_ir};
use netlang_core::measurement::MEASUREMENT_SCHEMA_VERSION;
use netlang_core::parse_program;
use netlang_core::sim_result::{AssertionStatus, evaluate_assertions, format_quantity};
use netlang_core::simulation::{
    AnalysisDataset, ComplexSeries, ComplexSeriesDataset, Dataset, RealSeriesDataset, SeriesAxis,
    SimulationResult, SimulationStatus, SimulatorInfo, SimulatorLog, SimulatorProcessStatus,
};
use std::collections::BTreeMap;

fn circuit(assertions: &str) -> CircuitIR {
    let source = format!(
        "net GND\nnet VDD\nnet out\nnet in\nnet bias\nnet qout\n\
         source VS 10V\nsource VIN 1V\nresistor RL 8\ntransistor Q1 npn 2N3904\n\
         connect VS.plus to VDD\nconnect VS.minus to GND\nconnect VIN.plus to in\nconnect VIN.minus to GND\n\
         connect RL.p1 to out\nconnect RL.p2 to GND\nconnect Q1.c to qout\nconnect Q1.b to bias\nconnect Q1.e to GND\n{assertions}"
    );
    ast_to_ir(&parse_program(&source).expect("measurement source should parse"))
        .expect("measurement source should reach IR")
}

fn simulation(analysis: Analysis, data: Dataset) -> SimulationResult {
    SimulationResult {
        schema_version: "netlang.simulation.v1".to_string(),
        status: SimulationStatus::Succeeded,
        analyses: vec![analysis.clone()],
        simulator: SimulatorInfo {
            executable: "fixture".to_string(),
            version: "fixture-1".to_string(),
        },
        process: SimulatorProcessStatus {
            exit_code: Some(0),
            success: true,
        },
        measurements: BTreeMap::new(),
        datasets: vec![AnalysisDataset {
            index: 0,
            analysis,
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

#[test]
fn voltage_current_power_and_derived_transient_metrics_are_typed() {
    let assertions = "assert gain(V(out),V(in)) > 3.99\n\
assert frequency(V(out)) == 1kHz\n\
assert output_power(V(out),RL) == 1W\n\
assert efficiency(V(out),RL,V(VDD),I(VS)) == 50%\n\
assert thd(V(out)) < 0.001%\n\
assert average(P(RL)) == 1W\n\
assert peak(P(RL)) == 2W\n\
assert dissipation(RL) == 1W\n\
assert rms(V(out),1ms,2ms) > 2.80V\n\
assert average(I(Q1)) == 100mA\n";
    let circuit = circuit(assertions);
    let sample_count = 192;
    let axis = (0..sample_count)
        .map(|index| index as f64 / 64_000.0)
        .collect::<Vec<_>>();
    let output = axis
        .iter()
        .map(|time| 4.0 * (std::f64::consts::TAU * 1_000.0 * time).sin())
        .collect::<Vec<_>>();
    let input = output.iter().map(|value| value / 4.0).collect::<Vec<_>>();
    let data = Dataset::Transient(RealSeriesDataset {
        axis: SeriesAxis {
            name: "time".to_string(),
            values: axis,
        },
        signals: BTreeMap::from([
            ("out".to_string(), output),
            ("in".to_string(), input),
            ("vdd".to_string(), vec![10.0; sample_count]),
            ("v_vs#branch".to_string(), vec![-0.2; sample_count]),
            ("@q_q1[ic]".to_string(), vec![0.1; sample_count]),
        ]),
    });
    let mut mixed_analysis = simulation(
        Analysis::Transient {
            step: netlang_core::ir::parse_quantity("1us", SIUnit::Second).unwrap(),
            stop: netlang_core::ir::parse_quantity("3ms", SIUnit::Second).unwrap(),
        },
        data,
    );
    mixed_analysis.analyses.insert(0, Analysis::OperatingPoint);
    mixed_analysis.datasets[0].index = 1;
    mixed_analysis.datasets.insert(
        0,
        AnalysisDataset {
            index: 0,
            analysis: Analysis::OperatingPoint,
            data: Dataset::OperatingPoint {
                values: BTreeMap::from([("out".to_string(), 0.0)]),
            },
        },
    );
    let report = evaluate_assertions(&circuit, &mixed_analysis);
    assert!(
        report
            .assertions
            .iter()
            .all(|assertion| assertion.status == AssertionStatus::Pass),
        "assertions: {:?}",
        report.assertions
    );
    assert_eq!(report.assertions[2].unit, SIUnit::Watt);
    assert_eq!(report.assertions[3].unit, SIUnit::Percent);
    assert_eq!(report.assertions[1].unit, SIUnit::Hertz);
    assert_eq!(MEASUREMENT_SCHEMA_VERSION, "netlang.measurement.v1");
}

#[test]
fn ac_gain_bandwidth_and_phase_use_complex_frequency_data() {
    let circuit = circuit(
        "assert gain(V(out),V(in)) > 0.99\n\
         assert bandwidth(V(out),V(in)) > 990Hz\n\
         assert bandwidth(V(out),V(in)) < 1010Hz\n\
         assert phase(V(out),V(in),1kHz) == -45deg\n",
    );
    let frequencies = vec![10.0, 100.0, 1_000.0, 10_000.0];
    let output = ComplexSeries {
        real: vec![0.9999, 0.9901, 0.5, 0.0099],
        imaginary: vec![-0.01, -0.099, -0.5, -0.099],
    };
    let input = ComplexSeries {
        real: vec![1.0; frequencies.len()],
        imaginary: vec![0.0; frequencies.len()],
    };
    let data = Dataset::Ac(ComplexSeriesDataset {
        frequency_hz: frequencies,
        signals: BTreeMap::from([("out".to_string(), output), ("in".to_string(), input)]),
    });
    let report = evaluate_assertions(
        &circuit,
        &simulation(
            Analysis::Ac {
                scale: netlang_core::ir::AcScale::Decade,
                points: 10,
                start: netlang_core::ir::parse_quantity("10Hz", SIUnit::Hertz).unwrap(),
                stop: netlang_core::ir::parse_quantity("10kHz", SIUnit::Hertz).unwrap(),
            },
            data,
        ),
    );
    assert!(report.all_passed(), "assertions: {:?}", report.assertions);
    assert!((report.assertions[1].actual.unwrap() - 1_000.0).abs() < 1.0);
    assert_eq!(report.assertions[3].unit, SIUnit::Degree);
}

#[test]
fn missing_or_unsupported_engineering_data_is_a_typed_error() {
    let circuit = circuit("assert bandwidth(V(out),V(in)) > 1kHz\nassert dissipation(Q1) < 1W\n");
    let data = Dataset::OperatingPoint {
        values: BTreeMap::from([("out".to_string(), 1.0)]),
    };
    let report = evaluate_assertions(&circuit, &simulation(Analysis::OperatingPoint, data));
    assert!(
        report
            .assertions
            .iter()
            .all(|assertion| assertion.status == AssertionStatus::Error)
    );
    assert_eq!(report.summary.errors, 2);
}

#[test]
fn engineering_units_render_without_misleading_si_scaling() {
    assert_eq!(format_quantity(2.0, SIUnit::Watt), "2.000000W");
    assert_eq!(format_quantity(0.75, SIUnit::Ratio), "0.750000");
    assert_eq!(format_quantity(0.5, SIUnit::Percent), "0.500000%");
    assert_eq!(format_quantity(-45.0, SIUnit::Degree), "-45.000000deg");
}
