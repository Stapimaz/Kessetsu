use kessetsu_core::ir::{Analysis, CircuitIR, SIUnit, ast_to_ir};
use kessetsu_core::measurement::MEASUREMENT_SCHEMA_VERSION;
use kessetsu_core::parse_program;
use kessetsu_core::sim_result::{AssertionStatus, evaluate_assertions, format_quantity};
use kessetsu_core::simulation::{
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
        schema_version: "kessetsu.simulation.v1".to_string(),
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
assert output_power(V(out),RL,1ms,2ms) == 1W\n\
assert efficiency(V(out),RL,V(VDD),I(VS),0ms,2ms) == 50%\n\
assert thd(V(out),1kHz,0ms,2ms,hann) < 0.01%\n\
assert average(P(RL)) == 1W\n\
assert peak(P(RL)) == 2W\n\
assert dissipation(RL,0ms,2ms) == 1W\n\
assert rms(V(out),1ms,2ms) > 2.80V\n\
assert average(I(Q1)) == 100mA\n\
assert peak(V(Q1.c,Q1.e)) == 0V\n";
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
            ("qout".to_string(), vec![0.0; sample_count]),
            ("v_vs#branch".to_string(), vec![-0.2; sample_count]),
            ("@q_q1[ic]".to_string(), vec![0.1; sample_count]),
        ]),
    });
    let mut mixed_analysis = simulation(
        Analysis::Transient {
            step: kessetsu_core::ir::parse_quantity("1us", SIUnit::Second).unwrap(),
            stop: kessetsu_core::ir::parse_quantity("3ms", SIUnit::Second).unwrap(),
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
    assert_eq!(MEASUREMENT_SCHEMA_VERSION, "kessetsu.measurement.v2");
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
                scale: kessetsu_core::ir::AcScale::Decade,
                points: 10,
                start: kessetsu_core::ir::parse_quantity("10Hz", SIUnit::Hertz).unwrap(),
                stop: kessetsu_core::ir::parse_quantity("10kHz", SIUnit::Hertz).unwrap(),
            },
            data,
        ),
    );
    assert!(report.all_passed(), "assertions: {:?}", report.assertions);
    assert!((report.assertions[1].actual.unwrap() - 1_000.0).abs() < 1.0);
    assert_eq!(report.assertions[3].unit, SIUnit::Degree);
}

#[test]
fn bandwidth_fails_closed_for_non_low_pass_response() {
    let circuit = circuit("assert bandwidth(V(out),V(in)) > 1kHz\n");
    let data = Dataset::Ac(ComplexSeriesDataset {
        frequency_hz: vec![10.0, 100.0, 1_000.0, 10_000.0],
        signals: BTreeMap::from([
            (
                "out".to_string(),
                ComplexSeries {
                    real: vec![0.1, 1.0, 0.7, 0.1],
                    imaginary: vec![0.0; 4],
                },
            ),
            (
                "in".to_string(),
                ComplexSeries {
                    real: vec![1.0; 4],
                    imaginary: vec![0.0; 4],
                },
            ),
        ]),
    });
    let report = evaluate_assertions(
        &circuit,
        &simulation(
            Analysis::Ac {
                scale: kessetsu_core::ir::AcScale::Decade,
                points: 10,
                start: kessetsu_core::ir::parse_quantity("10Hz", SIUnit::Hertz).unwrap(),
                stop: kessetsu_core::ir::parse_quantity("10kHz", SIUnit::Hertz).unwrap(),
            },
            data,
        ),
    );
    assert_eq!(report.assertions[0].status, AssertionStatus::Error);
    assert!(
        report.assertions[0]
            .message
            .as_deref()
            .is_some_and(|message| message.contains("low-pass"))
    );
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

fn ac_fixture(frequencies: Vec<f64>, gains: Vec<f64>) -> SimulationResult {
    let count = frequencies.len();
    simulation(
        Analysis::Ac {
            scale: kessetsu_core::ir::AcScale::Decade,
            points: 100,
            start: kessetsu_core::ir::parse_quantity("1Hz", SIUnit::Hertz).unwrap(),
            stop: kessetsu_core::ir::parse_quantity("1MHz", SIUnit::Hertz).unwrap(),
        },
        Dataset::Ac(ComplexSeriesDataset {
            frequency_hz: frequencies,
            signals: BTreeMap::from([
                (
                    "out".into(),
                    ComplexSeries {
                        real: gains,
                        imaginary: vec![0.0; count],
                    },
                ),
                (
                    "in".into(),
                    ComplexSeries {
                        real: vec![1.0; count],
                        imaginary: vec![0.0; count],
                    },
                ),
            ]),
        }),
    )
}

fn ac_metric(expression: &str, result: &SimulationResult) -> Result<f64, String> {
    let unit = if expression.starts_with("gain") {
        ""
    } else {
        "Hz"
    };
    let ir = circuit(&format!("assert {expression} > 0{unit}\n"));
    kessetsu_core::measurement::evaluate_assertion_metric(&ir.assertions[0], &ir, result)
}

fn near(actual: f64, expected: f64) {
    assert!(
        (actual / expected - 1.0).abs() < 0.001,
        "{actual} != {expected}"
    );
}

#[test]
fn gain_at_interpolates_magnitude_on_log_frequency_without_extrapolation() {
    let ac = ac_fixture(vec![10.0, 100.0, 1000.0], vec![2.0, 6.0, 10.0]);
    near(
        ac_metric("gain_at(V(out),V(in),31.6227766017Hz)", &ac).unwrap(),
        4.0,
    );
    for (frequency, expected) in [(10, 2.0), (100, 6.0), (1000, 10.0)] {
        near(
            ac_metric(&format!("gain_at(V(out),V(in),{frequency}Hz)"), &ac).unwrap(),
            expected,
        );
    }
    for frequency in [1, 1001] {
        assert!(
            ac_metric(&format!("gain_at(V(out),V(in),{frequency}Hz)"), &ac)
                .unwrap_err()
                .contains("outside")
        );
    }
    let report = evaluate_assertions(&circuit("assert gain_at(V(out),V(in),100Hz) == 6\n"), &ac);
    assert!(report.all_passed());
    assert_eq!(report.assertions[0].unit, SIUnit::Ratio);
    let mut complex = ac_fixture(vec![10.0], vec![3.0]);
    let Dataset::Ac(data) = &mut complex.datasets[0].data else {
        unreachable!()
    };
    data.signals.get_mut("out").unwrap().imaginary[0] = 4.0;
    near(
        ac_metric("gain_at(V(out),V(in),10Hz)", &complex).unwrap(),
        5.0,
    );
    assert!(
        ac_metric(
            "gain_at(V(out),V(in),10Hz)",
            &simulation(
                Analysis::OperatingPoint,
                Dataset::OperatingPoint {
                    values: BTreeMap::new()
                }
            )
        )
        .unwrap_err()
        .contains("AC")
    );
}

#[test]
fn lower_and_upper_edges_match_independent_filter_equations() {
    let frequencies = (0..=800)
        .map(|i| 10.0_f64.powf(i as f64 / 100.0 - 1.0))
        .collect::<Vec<_>>();
    let lp = ac_fixture(
        frequencies.clone(),
        frequencies
            .iter()
            .map(|f| 1.0 / (1.0 + (f / 1000.0).powi(2)).sqrt())
            .collect(),
    );
    let hp = ac_fixture(
        frequencies.clone(),
        frequencies
            .iter()
            .map(|f| 1.0 / (1.0 + (1000.0 / f).powi(2)).sqrt())
            .collect(),
    );
    near(
        ac_metric("upper_cutoff(V(out),V(in))", &lp).unwrap(),
        1000.0,
    );
    near(
        ac_metric("lower_cutoff(V(out),V(in))", &hp).unwrap(),
        1000.0,
    );
    assert!(
        ac_metric("lower_cutoff(V(out),V(in))", &lp)
            .unwrap_err()
            .contains("no observed")
    );
    assert!(
        ac_metric("upper_cutoff(V(out),V(in))", &hp)
            .unwrap_err()
            .contains("no observed")
    );
    let bp = ac_fixture(
        frequencies.clone(),
        frequencies
            .iter()
            .map(|f| 20.0 / ((1.0 + (100.0 / f).powi(2)) * (1.0 + (f / 10_000.0).powi(2))).sqrt())
            .collect(),
    );
    // Solve the half-power equation independently in y = f^2.
    let b: f64 = 10_000.0_f64.powi(2) + 4.0 * 100.0 * 10_000.0 + 100.0_f64.powi(2);
    let c = (100.0_f64 * 10_000.0).powi(2);
    let upper_squared = (b + (b * b - 4.0 * c).sqrt()) / 2.0;
    for suffix in ["", ",1kHz"] {
        near(
            ac_metric(&format!("lower_cutoff(V(out),V(in){suffix})"), &bp).unwrap(),
            (c / upper_squared).sqrt(),
        );
        near(
            ac_metric(&format!("upper_cutoff(V(out),V(in){suffix})"), &bp).unwrap(),
            upper_squared.sqrt(),
        );
    }
    let report = evaluate_assertions(
        &circuit(
            "assert lower_cutoff(V(out),V(in),1kHz) > 90Hz\nassert upper_cutoff(V(out),V(in),1kHz) < 11kHz\n",
        ),
        &bp,
    );
    assert!(report.all_passed());
    assert!(report.assertions.iter().all(|a| a.unit == SIUnit::Hertz));
}

#[test]
fn explicit_reference_selects_one_band_and_automatic_selection_rejects_ambiguity() {
    let ac = ac_fixture(
        vec![10.0, 100.0, 1000.0, 10_000.0, 100_000.0],
        vec![0.0, 1.0, 0.1, 1.0, 0.0],
    );
    for metric in ["lower_cutoff", "upper_cutoff"] {
        assert!(
            ac_metric(&format!("{metric}(V(out),V(in))"), &ac)
                .unwrap_err()
                .contains("reference_frequency")
        );
    }
    for (reference, low, high) in [(100, 10.0, 1000.0), (10_000, 1000.0, 100_000.0)] {
        let lower = ac_metric(&format!("lower_cutoff(V(out),V(in),{reference}Hz)"), &ac).unwrap();
        let upper = ac_metric(&format!("upper_cutoff(V(out),V(in),{reference}Hz)"), &ac).unwrap();
        assert!(low < lower && lower < reference as f64);
        assert!((reference as f64) < upper && upper < high);
    }
    // An unsampled reference is interpolated, not snapped to a nearby peak.
    let simple = ac_fixture(vec![10.0, 100.0, 1000.0], vec![0.0, 1.0, 0.0]);
    let reference = 10.0_f64.sqrt() * 10.0;
    let threshold = 0.5 / 2.0_f64.sqrt();
    near(
        ac_metric(
            &format!("lower_cutoff(V(out),V(in),{reference}Hz)"),
            &simple,
        )
        .unwrap(),
        10.0 * 10.0_f64.powf(threshold),
    );
    near(
        ac_metric(
            &format!("upper_cutoff(V(out),V(in),{reference}Hz)"),
            &simple,
        )
        .unwrap(),
        100.0 * 10.0_f64.powf(1.0 - threshold),
    );
    let zero = ac_fixture(vec![10.0, 100.0], vec![0.0, 0.0]);
    assert!(
        ac_metric("upper_cutoff(V(out),V(in),100Hz)", &zero)
            .unwrap_err()
            .contains("positive")
    );
    assert!(
        ac_metric("lower_cutoff(V(out),V(in),1Hz)", &ac)
            .unwrap_err()
            .contains("outside")
    );
    assert!(
        ac_metric("upper_cutoff(V(out),V(in))", &zero)
            .unwrap_err()
            .contains("positive")
    );
}

#[test]
fn threshold_touches_are_connected_and_observed_endpoint_edges_are_valid() {
    let threshold = 1.0 / 2.0_f64.sqrt();
    let ac = ac_fixture(
        vec![10.0, 100.0, 1000.0, 10_000.0, 100_000.0],
        vec![threshold, 1.0, threshold, 1.0, threshold],
    );
    near(ac_metric("lower_cutoff(V(out),V(in))", &ac).unwrap(), 10.0);
    near(
        ac_metric("upper_cutoff(V(out),V(in))", &ac).unwrap(),
        100_000.0,
    );
    let flat = ac_fixture(vec![10.0, 100.0], vec![1.0, 1.0]);
    for metric in ["lower_cutoff", "upper_cutoff"] {
        assert!(
            ac_metric(&format!("{metric}(V(out),V(in))"), &flat)
                .unwrap_err()
                .contains("no observed")
        );
    }
    let one = ac_fixture(vec![10.0], vec![2.0]);
    near(ac_metric("gain_at(V(out),V(in),10Hz)", &one).unwrap(), 2.0);
    assert!(ac_metric("upper_cutoff(V(out),V(in))", &one).is_err());
}

#[test]
fn new_ac_metrics_reject_malformed_data_and_accept_small_nonzero_inputs() {
    for axis in [
        vec![],
        vec![10.0, 10.0],
        vec![100.0, 10.0],
        vec![0.0, 10.0],
        vec![-1.0, 10.0],
        vec![10.0, f64::NAN],
        vec![10.0, f64::INFINITY],
    ] {
        let gains = vec![1.0; axis.len()];
        assert!(ac_metric("gain_at(V(out),V(in),10Hz)", &ac_fixture(axis, gains)).is_err());
    }
    for invalid in [0.0, f64::NAN, f64::INFINITY] {
        let mut result = ac_fixture(vec![10.0, 100.0], vec![2.0, 2.0]);
        let Dataset::Ac(data) = &mut result.datasets[0].data else {
            unreachable!()
        };
        data.signals.get_mut("in").unwrap().real[0] = invalid;
        assert!(ac_metric("gain_at(V(out),V(in),10Hz)", &result).is_err());
    }
    for imaginary in [false, true] {
        let mut result = ac_fixture(vec![10.0, 100.0], vec![2.0, 2.0]);
        let Dataset::Ac(data) = &mut result.datasets[0].data else {
            unreachable!()
        };
        let out = data.signals.get_mut("out").unwrap();
        if imaginary {
            out.imaginary.pop();
        } else {
            out.real.pop();
        }
        assert!(ac_metric("gain_at(V(out),V(in),10Hz)", &result).is_err());
    }
    let mut tiny = ac_fixture(vec![10.0, 100.0], vec![2e-200, 2e-200]);
    let Dataset::Ac(data) = &mut tiny.datasets[0].data else {
        unreachable!()
    };
    data.signals.get_mut("in").unwrap().real = vec![1e-200; 2];
    near(ac_metric("gain_at(V(out),V(in),10Hz)", &tiny).unwrap(), 2.0);
    let Dataset::Ac(data) = &mut tiny.datasets[0].data else {
        unreachable!()
    };
    data.signals.get_mut("out").unwrap().real[0] = f64::MAX;
    assert!(ac_metric("gain_at(V(out),V(in),10Hz)", &tiny).is_err());
}

#[test]
fn new_ac_metric_arguments_and_units_are_checked_before_simulation() {
    for expression in [
        "gain_at(V(out),V(in)) > 1",
        "gain_at(V(out),V(in),0Hz) > 1",
        "gain_at(V(out),V(in),-1Hz) > 1",
        "gain_at(V(out),V(in),1ms) > 1",
        "gain_at(I(VIN),V(in),1kHz) > 1",
        "gain_at(V(out),V(in),1kHz) > 1Hz",
        "lower_cutoff(V(out)) > 1Hz",
        "upper_cutoff(V(out),V(in),1kHz,2kHz) > 1Hz",
        "lower_cutoff(V(Q1.c,Q1.e),V(in)) > 1Hz",
        "upper_cutoff(V(),V(in)) > 1Hz",
        "lower_cutoff(V(out),V(in)) > 1V",
    ] {
        let source = format!(
            "net out\nnet in\nsource VIN 1V\ntransistor Q1 npn 2N3904\nassert {expression}\n"
        );
        let parsed = parse_program(&source);
        assert!(
            parsed.is_err() || ast_to_ir(&parsed.unwrap()).is_err(),
            "accepted {expression}"
        );
    }
}

#[test]
fn gain_at_uses_first_ac_dataset_and_never_transient_rms() {
    let mut mixed = ac_fixture(vec![10.0, 100.0], vec![10.0, 10.0]);
    mixed.datasets.insert(
        0,
        AnalysisDataset {
            index: 1,
            analysis: Analysis::Transient {
                step: kessetsu_core::ir::parse_quantity("1ms", SIUnit::Second).unwrap(),
                stop: kessetsu_core::ir::parse_quantity("2ms", SIUnit::Second).unwrap(),
            },
            data: Dataset::Transient(RealSeriesDataset {
                axis: SeriesAxis {
                    name: "time".into(),
                    values: vec![0.0, 0.001],
                },
                signals: BTreeMap::from([
                    ("out".into(), vec![4.0, -4.0]),
                    ("in".into(), vec![1.0, -1.0]),
                ]),
            }),
        },
    );
    mixed.datasets.push(
        ac_fixture(vec![100.0, 1000.0], vec![20.0, 20.0])
            .datasets
            .remove(0),
    );
    near(ac_metric("gain(V(out),V(in))", &mixed).unwrap(), 4.0);
    near(
        ac_metric("gain_at(V(out),V(in),100Hz)", &mixed).unwrap(),
        10.0,
    );
    assert!(
        ac_metric("gain_at(V(out),V(in),1000Hz)", &mixed)
            .unwrap_err()
            .contains("outside")
    );
}
