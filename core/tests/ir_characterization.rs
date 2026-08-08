use netlang_core::ast::Statement;
use netlang_core::ir::{
    ComponentKind, ComponentParams, Waveform, ast_to_ir, parse_si_value, parse_waveform,
    resolve_model,
};
use netlang_core::parse_program;

fn assert_approx_eq(actual: f64, expected: f64) {
    let tolerance = f64::max(expected.abs() * 1e-12, 1e-15);
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn supported_si_values_are_characterized() {
    for (source, expected) in [
        ("10k", 10_000.0),
        ("2.2k", 2_200.0),
        ("100uF", 100e-6),
        ("1MHz", 1e6),
        ("-2mA", -2e-3),
    ] {
        assert_approx_eq(
            parse_si_value(source).expect("value should parse"),
            expected,
        );
    }
}

#[test]
fn scientific_notation_is_currently_rejected_instead_of_misparsed() {
    assert!(parse_si_value("1e-3").is_err());
}

#[test]
fn sine_waveform_parameters_preserve_si_scaling() {
    let waveform = parse_waveform("sine(0mA, 5mA, 10kHz)").expect("sine should parse");
    match waveform {
        Waveform::Sine {
            offset,
            amplitude,
            frequency,
        } => {
            assert_approx_eq(offset, 0.0);
            assert_approx_eq(amplitude, 5e-3);
            assert_approx_eq(frequency, 10e3);
        }
        other => panic!("expected sine waveform, got {other:?}"),
    }
}

#[test]
fn builtin_model_resolution_covers_supported_polarities() {
    for (name, expected_kind) in [
        (
            "2N3904",
            ComponentKind::BJT(netlang_core::ir::BJTPolarity::NPN),
        ),
        (
            "2N3906",
            ComponentKind::BJT(netlang_core::ir::BJTPolarity::PNP),
        ),
        (
            "IRF540",
            ComponentKind::MOSFET(netlang_core::ir::FETPolarity::NMOS),
        ),
        ("1N4148", ComponentKind::Diode),
    ] {
        let model = resolve_model(name).unwrap_or_else(|| panic!("missing model {name}"));
        assert_eq!(model.kind, expected_kind);
    }
    assert!(resolve_model("NOT_A_MODEL").is_none());
}

#[test]
fn source_and_assertion_values_reach_typed_ir() {
    let source =
        "source V1 10V\ncurrent_source I1 sine(0mA, 5mA, 10kHz)\nassert peak(I(V1)) < 100mA\n";
    let program = parse_program(source).expect("source should parse");
    assert!(
        program
            .statements
            .iter()
            .any(|statement| matches!(statement, Statement::Decl(_)))
    );
    let circuit = ast_to_ir(&program).expect("source should convert to IR");

    assert!(matches!(
        circuit.components[0].parameters,
        ComponentParams::DCSource { voltage } if voltage == 10.0
    ));
    assert!(matches!(
        circuit.components[1].parameters,
        ComponentParams::ACSource {
            waveform: Waveform::Sine { .. }
        }
    ));
    assert_approx_eq(circuit.assertions[0].threshold, 0.1);
}

#[test]
fn malformed_si_number_shapes_are_rejected() {
    for value in ["not_a_number", "1.2.3V", "--1A"] {
        assert!(
            parse_si_value(value).is_err(),
            "{value} unexpectedly parsed"
        );
    }
}

#[test]
fn every_assertion_comparator_reaches_typed_ir() {
    use netlang_core::ast::Cmp;

    let cases = [
        ("<", Cmp::Lt),
        (">", Cmp::Gt),
        ("==", Cmp::Eq),
        ("<=", Cmp::Le),
        (">=", Cmp::Ge),
    ];

    for (operator, expected) in cases {
        let source = format!("assert max(V(out)) {operator} 5V\n");
        let program = parse_program(&source).expect("assertion should parse");
        let circuit = ast_to_ir(&program).expect("assertion should convert to IR");
        assert_eq!(circuit.assertions[0].cmp, expected);
        assert_approx_eq(circuit.assertions[0].threshold, 5.0);
    }
}
