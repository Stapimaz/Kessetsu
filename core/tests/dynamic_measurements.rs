use kessetsu_core::compiler::{CompileOptions, compile_source};
use kessetsu_core::ir::{Analysis, SIUnit};
use kessetsu_core::measurement::evaluate_assertion_metric;
use kessetsu_core::simulation::*;
use std::collections::BTreeMap;

fn evaluate(metric: &str, threshold: &str, values: Vec<f64>, current: f64) -> Result<f64, String> {
    let source = format!(
        "net GND\nnet OUT\nsource VIN 1V\nresistor R1 1k\nconnect VIN.plus, R1.p1 to OUT\nconnect VIN.minus, R1.p2 to GND\nsimulate tran 1ms 4ms\nassert {metric} >= {threshold}\n"
    );
    let compiled = compile_source(&source, CompileOptions::default());
    assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);
    let circuit = compiled.ir.unwrap();
    let analysis = circuit.analyses[0].clone();
    let simulation = SimulationResult {
        schema_version: SIMULATION_SCHEMA_VERSION.into(),
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
                    values: vec![0.0, 0.001, 0.002, 0.003, 0.004],
                },
                signals: BTreeMap::from([
                    ("out".into(), values),
                    ("v_vin#branch".into(), vec![current; 5]),
                ]),
            }),
        }],
        diagnostics: vec![],
        warnings: vec![],
        errors: vec![],
        raw_log: SimulatorLog {
            stdout: String::new(),
            stderr: String::new(),
        },
        artifacts: vec![],
    };
    evaluate_assertion_metric(&circuit.assertions[0], &circuit, &simulation)
}
#[test]
fn directed_crossings_interpolate_without_extrapolation() {
    let a = evaluate(
        "rise_time(V(OUT),0.2V,0.8V,0ms,4ms)",
        "0s",
        vec![0., 0.5, 1., 1., 1.],
        -0.1,
    )
    .unwrap();
    assert!((a - 0.0012).abs() < 1e-12);
    let b = evaluate(
        "fall_time(V(OUT),0.2V,0.8V,0ms,4ms)",
        "0s",
        vec![1., 0.5, 0., 0., 0.],
        -0.1,
    )
    .unwrap();
    assert!((b - 0.0012).abs() < 1e-12);
    assert!(
        evaluate(
            "rise_time(V(OUT),0.2V,0.8V,0ms,4ms)",
            "0s",
            vec![1.; 5],
            -0.1
        )
        .is_err()
    );
    assert!(
        evaluate(
            "rise_time(V(OUT),0.2V,0.8V,0ms,5ms)",
            "0s",
            vec![0., 0.5, 1., 1., 1.],
            -0.1
        )
        .is_err()
    );
}
#[test]
fn settling_and_directional_overshoot_use_explicit_window() {
    let value = evaluate(
        "settling_time(V(OUT),1V,0.1V,0ms,4ms)",
        "0s",
        vec![0., 0.5, 1.2, 1.05, 1.],
        -0.1,
    )
    .unwrap();
    assert!((value - 0.002666666666666667).abs() < 1e-12);
    assert!(
        evaluate(
            "settling_time(V(OUT),1V,0.1V,0ms,4ms)",
            "0s",
            vec![0., 0.5, 1.2, 1.05, 1.2],
            -0.1
        )
        .is_err()
    );
    let positive = evaluate(
        "overshoot(V(OUT),0V,1V,0ms,4ms)",
        "0%",
        vec![0., 0.5, 1.2, 1.05, 1.],
        -0.1,
    )
    .unwrap();
    let negative = evaluate(
        "overshoot(V(OUT),1V,0V,0ms,4ms)",
        "0%",
        vec![1., 0.5, -0.2, -0.05, 0.],
        -0.1,
    )
    .unwrap();
    assert!((positive - 20.).abs() < 1e-12 && (negative - 20.).abs() < 1e-12);
}
#[test]
fn energy_integrates_signed_power_with_interpolated_boundaries() {
    let result = evaluate(
        "energy(V(OUT),I(VIN),0.5ms,3.5ms)",
        "0J",
        vec![0., 1., 2., 3., 4.],
        -0.1,
    )
    .unwrap();
    assert!((result + 0.0006).abs() < 1e-12);
    assert_eq!(
        kessetsu_core::ir::parse_quantity("1mJ", SIUnit::Joule)
            .unwrap()
            .value,
        0.001
    );
}
#[test]
fn invalid_windows_thresholds_and_dimensions_fail_compilation() {
    for call in [
        "rise_time(V(OUT),0.8V,0.2V,0ms,4ms)",
        "settling_time(V(OUT),1V,0V,0ms,4ms)",
        "overshoot(V(OUT),1V,1V,0ms,4ms)",
        "energy(V(OUT),I(VIN),4ms,0ms)",
        "energy(V(OUT),V(OUT),0ms,4ms)",
    ] {
        let source = format!(
            "net GND\nnet OUT\nsource VIN 1V\nresistor R1 1k\nconnect VIN.plus, R1.p1 to OUT\nconnect VIN.minus, R1.p2 to GND\nsimulate tran 1ms 4ms\nassert {call} >= 0\n"
        );
        assert!(
            compile_source(&source, CompileOptions::default()).has_errors(),
            "{call}"
        );
    }
    let _ = Analysis::OperatingPoint;
}

#[test]
fn real_rc_step_response_matches_time_constant_and_energy() {
    let source = "param tau: s = 1ms\nparam low: V = 0.1V\nparam high: V = 0.9V\nnet GND\nnet IN\nnet OUT\nsource VIN pulse(0V,1V,1ms,1us,1us,20ms,40ms)\nresistor R1 1kOhm\ncapacitor C1 {tau / 1kOhm}\nresistor RL 1GOhm\nconnect VIN.plus, R1.p1 to IN\nconnect VIN.minus, C1.p2, RL.p2 to GND\nconnect R1.p2, C1.p1, RL.p1 to OUT\nsimulate tran 2us 10ms\nassert rise_time(V(OUT),{low},{high},0ms,10ms) > 2.18ms\nassert rise_time(V(OUT),{low},{high},0ms,10ms) < 2.21ms\nassert settling_time(V(OUT),1V,20mV,1ms,10ms) > 3.8ms\nassert settling_time(V(OUT),1V,20mV,1ms,10ms) < 4ms\nassert overshoot(V(OUT),0V,1V,0ms,10ms) == 0%\nassert energy(V(OUT),I(RL),1ms,10ms) > 7.4pJ\nassert energy(V(OUT),I(RL),1ms,10ms) < 7.6pJ\n";
    let compiled = compile_source(source, CompileOptions::default());
    assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);
    let circuit = compiled.ir.unwrap();
    let simulation = NgspiceRunner::discover()
        .run(
            &SimulationRequest::new(compiled.spice_netlist.unwrap(), circuit.analyses.clone()),
            &CancellationToken::new(),
        )
        .unwrap();
    let assertions = kessetsu_core::sim_result::evaluate_assertions(&circuit, &simulation);
    assert!(assertions.all_passed(), "{assertions:?}");
}
