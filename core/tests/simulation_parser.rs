mod common;

use common::read_fixture;
use kessetsu_core::ir::{AcScale, Analysis, Quantity, SIUnit};
use kessetsu_core::simulation::{Dataset, SimulatorDiagnosticKind, SimulatorDiagnosticSeverity};
use kessetsu_core::simulation_parser::{classify_simulator_log, parse_measurements, parse_wrdata};

fn seconds(value: f64) -> Quantity {
    Quantity {
        value,
        unit: SIUnit::Second,
    }
}

fn hertz(value: f64) -> Quantity {
    Quantity {
        value,
        unit: SIUnit::Hertz,
    }
}

#[test]
fn measurement_parser_handles_exponents_locale_and_line_endings() {
    let fixture = read_fixture("simulation/output/measurements.txt");
    let expected = parse_measurements(&fixture).expect("LF fixture should parse");
    let crlf = fixture.replace('\n', "\r\n");
    let actual = parse_measurements(&crlf).expect("CRLF fixture should parse identically");

    assert_eq!(actual, expected);
    assert_eq!(actual["max_v_out"], 0.321);
    assert_eq!(actual["min_v_out"], -0.00125);
    assert_eq!(actual["rms_i_v1"], 6.2e-8);
}

#[test]
fn measurement_parser_rejects_duplicates_malformed_and_non_finite_values() {
    for fixture in [
        "gain = 2\ngain = 3\n",
        "gain = not-a-number\n",
        "gain = NaN\n",
        "gain = inf\n",
    ] {
        let errors = parse_measurements(fixture).expect_err("ambiguous value must fail closed");
        assert!(!errors.is_empty());
    }
}

#[test]
fn operating_point_fixture_becomes_a_sorted_map() {
    let data = parse_wrdata(
        &Analysis::OperatingPoint,
        &read_fixture("simulation/output/op.data"),
    )
    .expect("OP fixture should parse");
    let Dataset::OperatingPoint { values } = data else {
        panic!("expected operating-point dataset");
    };
    assert_eq!(
        values.keys().cloned().collect::<Vec<_>>(),
        ["in", "out", "v_input#branch"]
    );
    assert_eq!(values["out"], 2.5);
    assert_eq!(values["v_input#branch"], -0.0025);
}

#[test]
fn transient_fixture_becomes_a_real_time_series() {
    let analysis = Analysis::Transient {
        step: seconds(1e-6),
        stop: seconds(2e-6),
    };
    let data = parse_wrdata(&analysis, &read_fixture("simulation/output/tran.data"))
        .expect("transient fixture should parse");
    let Dataset::Transient(series) = data else {
        panic!("expected transient dataset");
    };
    assert_eq!(series.axis.name, "time");
    assert_eq!(series.axis.values, [0.0, 1e-6, 2e-6]);
    assert!(!series.signals.contains_key("time"));
    assert_eq!(series.signals["out"].len(), series.axis.values.len());
    assert_eq!(series.signals["in"][1], 5.0);
}

#[test]
fn ac_fixture_preserves_real_and_imaginary_components() {
    let analysis = Analysis::Ac {
        scale: AcScale::Decade,
        points: 10,
        start: hertz(10.0),
        stop: hertz(100_000.0),
    };
    let data = parse_wrdata(&analysis, &read_fixture("simulation/output/ac.data"))
        .expect("AC fixture should parse");
    let Dataset::Ac(series) = data else {
        panic!("expected AC dataset");
    };
    assert_eq!(series.frequency_hz.len(), 3);
    assert!(!series.signals.contains_key("frequency"));
    assert_eq!(series.signals["out"].real[0], 0.996067682);
    assert_eq!(series.signals["out"].imaginary[0], -0.0625847783);
}

#[test]
fn wrdata_parser_rejects_column_drift_duplicates_and_invalid_axes() {
    let tran = Analysis::Transient {
        step: seconds(1.0),
        stop: seconds(2.0),
    };
    for fixture in [
        "time out\n0 1\n1\n",
        "time out out\n0 1 2\n1 3 4\n",
        "time out\n1 1\n0 2\n",
    ] {
        assert!(parse_wrdata(&tran, fixture).is_err());
    }

    let ac = Analysis::Ac {
        scale: AcScale::Linear,
        points: 2,
        start: hertz(1.0),
        stop: hertz(2.0),
    };
    assert!(parse_wrdata(&ac, "frequency out other\n1 1 0\n2 2 0\n").is_err());

    let dc = Analysis::DcSweep {
        source: "V1".to_string(),
        start: Quantity {
            value: 1.0,
            unit: SIUnit::Volt,
        },
        stop: Quantity {
            value: -1.0,
            unit: SIUnit::Volt,
        },
        step: Quantity {
            value: -1.0,
            unit: SIUnit::Volt,
        },
    };
    let data = parse_wrdata(&dc, "v-sweep out\n1 2\n0 1\n-1 0\n")
        .expect("descending DC sweep should be monotonic and valid");
    assert!(matches!(data, Dataset::DcSweep(_)));
}

#[test]
fn simulator_log_classifier_separates_warning_convergence_and_fatal() {
    let diagnostics = classify_simulator_log(
        "Warning: model parameter defaulted\nWarning: singular matrix\nNo errors found\n",
        "Fatal error: simulation aborted\n",
    );

    assert_eq!(diagnostics.len(), 3);
    assert_eq!(diagnostics[0].kind, SimulatorDiagnosticKind::Warning);
    assert_eq!(
        diagnostics[0].severity,
        SimulatorDiagnosticSeverity::Warning
    );
    assert_eq!(diagnostics[1].kind, SimulatorDiagnosticKind::Convergence);
    assert_eq!(diagnostics[1].severity, SimulatorDiagnosticSeverity::Error);
    assert_eq!(diagnostics[2].kind, SimulatorDiagnosticKind::Fatal);
    assert_eq!(diagnostics[2].severity, SimulatorDiagnosticSeverity::Error);
}
