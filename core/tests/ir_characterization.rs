use kessetsu_core::ast::Statement;
use kessetsu_core::ir::{
    AcScale, Analysis, BJTPolarity, ComponentKind, ComponentParams, SIUnit, SourceValue, Waveform,
    ast_to_ir, parse_quantity, parse_si_value, parse_waveform, resolve_model,
};
use kessetsu_core::parse_program;

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
fn decimal_negative_and_scientific_notation_are_supported() {
    for (source, expected) in [(".5", 0.5), ("-0.25", -0.25), ("1e-3", 1e-3), ("2E+3", 2e3)] {
        assert_approx_eq(
            parse_si_value(source).expect("value should parse"),
            expected,
        );
    }
}

#[test]
fn sine_waveform_parameters_preserve_si_scaling() {
    let waveform = parse_waveform("sine(0mA, 5mA, 10kHz)", SIUnit::Ampere)
        .expect("sine should be valid")
        .expect("sine should parse");
    match waveform {
        Waveform::Sine {
            offset,
            amplitude,
            frequency,
        } => {
            assert_eq!(offset.unit, SIUnit::Ampere);
            assert_eq!(amplitude.unit, SIUnit::Ampere);
            assert_eq!(frequency.unit, SIUnit::Hertz);
            assert_approx_eq(offset.value, 0.0);
            assert_approx_eq(amplitude.value, 5e-3);
            assert_approx_eq(frequency.value, 10e3);
        }
        other => panic!("expected sine waveform, got {other:?}"),
    }
}

#[test]
fn builtin_model_resolution_covers_supported_polarities() {
    for (name, expected_kind) in [
        (
            "2N3904",
            ComponentKind::BJT(kessetsu_core::ir::BJTPolarity::NPN),
        ),
        (
            "2N3906",
            ComponentKind::BJT(kessetsu_core::ir::BJTPolarity::PNP),
        ),
        (
            "IRF540",
            ComponentKind::MOSFET(kessetsu_core::ir::FETPolarity::NMOS),
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
        &circuit.components[0].parameters,
        ComponentParams::VoltageSource {
            value: SourceValue::Dc(value)
        } if value.value == 10.0 && value.unit == SIUnit::Volt
    ));
    assert!(matches!(
        &circuit.components[1].parameters,
        ComponentParams::CurrentSource {
            value: SourceValue::Waveform(Waveform::Sine { .. })
        }
    ));
    assert_eq!(circuit.assertions[0].threshold.unit, SIUnit::Ampere);
    assert_approx_eq(circuit.assertions[0].threshold.value, 0.1);
}

#[test]
fn malformed_numbers_and_trailing_text_are_rejected() {
    for value in ["not_a_number", "1.2.3V", "--1A", "10kgarbage", "1eV"] {
        assert!(
            parse_si_value(value).is_err(),
            "{value} unexpectedly parsed"
        );
    }
}

#[test]
fn every_assertion_comparator_reaches_typed_ir() {
    use kessetsu_core::ast::Cmp;

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
        assert_eq!(circuit.assertions[0].threshold.unit, SIUnit::Volt);
        assert_approx_eq(circuit.assertions[0].threshold.value, 5.0);
    }
}

#[test]
fn quantity_parser_rejects_wrong_physical_dimensions() {
    assert!(parse_quantity("10V", SIUnit::Ohm).is_err());
    assert!(parse_quantity("1ms", SIUnit::Second).is_ok());
    assert!(parse_quantity("1mA", SIUnit::Ampere).is_ok());
}

#[test]
fn invalid_or_missing_component_values_fail_ir_conversion() {
    for source in [
        "resistor R1\n",
        "resistor R1 definitely_not_a_value\n",
        "resistor R1 5V\n",
        "source V1 10A\n",
        "current_source I1 10V\n",
    ] {
        let program = parse_program(source).expect("syntax should parse");
        let diagnostic = ast_to_ir(&program).expect_err("invalid value reached typed IR");
        assert_eq!(
            diagnostic.code, "KES-C001",
            "unexpected error for {source:?}"
        );
    }
}

#[test]
fn waveform_arity_and_units_are_validated() {
    for source in [
        "source V1 sine(0V, 1V)\n",
        "source V1 ac(1A)\n",
        "source V1 sine_ac(0V, 1V, 1kHz)\n",
        "source V1 sine_ac(0V, 1V, 1ms, 1V)\n",
        "source V1 sine(0V, 1A, 1kHz)\n",
        "source V1 sine(0V, 1V, 1ms)\n",
        "current_source I1 pulse(0A, 1A, 1ms, 1us, 1us, 5V, 10ms)\n",
        "source V1 pwl(0s, 0V, 1ms)\n",
        "source V1 pwl(0s, 0V, 1ms, 1A)\n",
        "source V1 pwl(1ms, 0V, 500us, 1V)\n",
    ] {
        let program = parse_program(source).expect("syntax should parse");
        let diagnostic = ast_to_ir(&program).expect_err("invalid waveform reached typed IR");
        assert_eq!(
            diagnostic.code, "KES-C002",
            "unexpected error for {source:?}"
        );
    }

    let source = "source V1 pulse(0V, 5V, 1ms, 1us, 2us, 3ms, 10ms)\n";
    let program = parse_program(source).expect("pulse should parse");
    let circuit = ast_to_ir(&program).expect("pulse should reach typed IR");
    assert!(matches!(
        &circuit.components[0].parameters,
        ComponentParams::VoltageSource {
            value: SourceValue::Waveform(Waveform::Pulse { period, .. })
        } if period.unit == SIUnit::Second && period.value == 0.01
    ));

    let source = "source V1 sine_ac(0V, 2V, 1kHz, 1V)\ncurrent_source I1 ac(1mA)\n";
    let circuit = ast_to_ir(&parse_program(source).expect("AC sources should parse"))
        .expect("AC sources should reach typed IR");
    assert!(matches!(
        &circuit.components[0].parameters,
        ComponentParams::VoltageSource {
            value: SourceValue::Waveform(Waveform::SineAc { ac_amplitude, .. })
        } if ac_amplitude.unit == SIUnit::Volt && ac_amplitude.value == 1.0
    ));
    assert!(matches!(
        &circuit.components[1].parameters,
        ComponentParams::CurrentSource {
            value: SourceValue::Waveform(Waveform::Ac { amplitude })
        } if amplitude.unit == SIUnit::Ampere && amplitude.value == 0.001
    ));

    let source = "source V1 pwl(0s, 0V, 1ms, 5V, 2ms, 0V)\n";
    let circuit = ast_to_ir(&parse_program(source).expect("PWL source should parse"))
        .expect("PWL source should reach typed IR");
    assert!(matches!(
        &circuit.components[0].parameters,
        ComponentParams::VoltageSource {
            value: SourceValue::Waveform(Waveform::PWL { points })
        } if points.len() == 3
            && points[1].0.unit == SIUnit::Second
            && points[1].0.value == 0.001
            && points[1].1.unit == SIUnit::Volt
            && points[1].1.value == 5.0
    ));
}

#[test]
fn assertion_threshold_dimension_matches_signal_dimension() {
    for source in [
        "assert max(V(out)) < 2A\n",
        "assert peak(I(V1)) < 5V\n",
        "assert max(R(out)) < 2V\n",
        "assert max(Voltage(out)) < 2V\n",
    ] {
        let program = parse_program(source).expect("syntax should parse");
        let diagnostic = ast_to_ir(&program).expect_err("invalid assertion reached typed IR");
        assert_eq!(
            diagnostic.code, "KES-C006",
            "unexpected error for {source:?}"
        );
    }

    let lowercase = parse_program("assert max(v(out)) < 2V\n").expect("signal should parse");
    let circuit = ast_to_ir(&lowercase).expect("signal function should be case insensitive");
    assert_eq!(circuit.assertions[0].threshold.unit, SIUnit::Volt);
}

#[test]
fn transistor_polarity_is_case_insensitive_at_the_parser_boundary() {
    let program = parse_program("transistor Q1 NPN\n").expect("uppercase polarity should parse");
    let circuit = ast_to_ir(&program).expect("transistor should reach typed IR");
    assert_eq!(
        circuit.components[0].kind,
        ComponentKind::BJT(BJTPolarity::NPN)
    );
    assert_eq!(
        circuit.components[0]
            .model
            .as_ref()
            .expect("default model should be assigned")
            .name,
        "2N3904"
    );
}

#[test]
fn builtin_model_defaults_are_explicit_in_typed_ir() {
    for (source, expected_model) in [
        ("transistor Q1\n", "2N3904"),
        ("transistor Q1 pnp\n", "2N3906"),
        ("mosfet M1\n", "IRF540"),
        ("diode D1\n", "1N4148"),
        ("opamp U1\n", "KESSETSU_OPAMP_V1"),
    ] {
        let program = parse_program(source).expect("component should parse");
        let circuit = ast_to_ir(&program).expect("builtin default should resolve");
        let model = circuit.components[0]
            .model
            .as_ref()
            .expect("model must be explicit in IR");
        assert_eq!(model.name, expected_model);
    }
}

#[test]
fn unsupported_and_incompatible_models_fail_closed_with_codes() {
    for (source, expected_code) in [
        ("transistor Q1 NOT_A_MODEL\n", "KES-C003"),
        ("mosfet M1 NOT_A_MODEL\n", "KES-C003"),
        ("diode D1 NOT_A_MODEL\n", "KES-C003"),
        ("opamp U1 LM358\n", "KES-C003"),
        ("transistor Q1 1N4148\n", "KES-C004"),
        ("transistor Q1 pnp 2N3904\n", "KES-C004"),
        ("diode D1 2N3904\n", "KES-C004"),
        ("mosfet M1 1N4148\n", "KES-C004"),
    ] {
        let program = parse_program(source).expect("component syntax should parse");
        let diagnostic = ast_to_ir(&program).expect_err("invalid model reached typed IR");
        assert_eq!(
            diagnostic.code, expected_code,
            "unexpected error for {source:?}"
        );
        assert_eq!(diagnostic.field.as_deref(), Some("model"));
    }
}

#[test]
fn flattened_module_ports_use_an_explicit_parameter_variant() {
    let source = include_str!("fixtures/valid/module.kess");
    let program = parse_program(source)
        .expect("module fixture should parse")
        .flatten()
        .expect("module fixture should flatten");
    let circuit = ast_to_ir(&program).expect("module fixture should reach typed IR");
    assert!(matches!(
        &circuit.components[1].parameters,
        ComponentParams::ModulePort { module_name } if module_name == "Divider"
    ));
}

#[test]
fn semantic_diagnostics_are_serializable_for_cli_and_wasm() {
    let program = parse_program("diode D1 UNKNOWN\n").expect("syntax should parse");
    let diagnostic = ast_to_ir(&program).expect_err("unknown model should fail");
    let json = serde_json::to_value(&diagnostic).expect("diagnostic should serialize");
    assert_eq!(json["code"], "KES-C003");
    assert_eq!(json["component"], "D1");
    assert_eq!(json["field"], "model");
}

#[test]
fn simulation_analyses_are_typed_and_unit_checked_in_ir() {
    let source = "source V1 5V\ncurrent_source I1 1mA\nsimulate op\nsimulate tran 1us 10ms\nsimulate ac dec 20 10Hz 1MHz\nsimulate dc V1 -1V 5V 100mV\nsimulate dc I1 2mA -2mA -100uA\n";
    let program = parse_program(source).expect("analyses should parse");
    let circuit = ast_to_ir(&program).expect("analyses should reach typed IR");

    assert_eq!(circuit.analyses[0], Analysis::OperatingPoint);
    assert!(matches!(
        &circuit.analyses[1],
        Analysis::Transient { step, stop }
            if step.unit == SIUnit::Second && step.value == 1e-6 && stop.value == 1e-2
    ));
    assert!(matches!(
        &circuit.analyses[2],
        Analysis::Ac { scale: AcScale::Decade, points: 20, start, stop }
            if start.unit == SIUnit::Hertz && start.value == 10.0 && stop.value == 1e6
    ));
    assert!(matches!(
        &circuit.analyses[3],
        Analysis::DcSweep { source, start, stop, step }
            if source == "V1" && start.unit == SIUnit::Volt && start.value == -1.0
                && stop.value == 5.0 && step.value == 0.1
    ));
    assert!(matches!(
        &circuit.analyses[4],
        Analysis::DcSweep { source, start, stop, step }
            if source == "I1" && start.unit == SIUnit::Ampere
                && (start.value - 2e-3).abs() < 1e-15
                && (stop.value + 2e-3).abs() < 1e-15
                && (step.value + 100e-6).abs() < 1e-15
    ));
}

#[test]
fn unsupported_or_malformed_analyses_fail_closed() {
    for source in [
        "simulate noise\n",
        "simulate op extra\n",
        "simulate tran 1V 10ms\n",
        "simulate tran 10ms 1ms\n",
        "simulate ac log 10 1Hz 1kHz\n",
        "simulate ac dec zero 1Hz 1kHz\n",
        "simulate ac dec 10 1kHz 1Hz\n",
        "simulate dc V1 0V 5V 1V\n",
        "resistor R1 1k\nsimulate dc R1 0V 5V 1V\n",
        "source V1 5V\nsimulate dc V1 0V 5V -1V\n",
    ] {
        let program = parse_program(source).expect("analysis syntax should parse");
        let diagnostic = ast_to_ir(&program).expect_err("invalid analysis reached typed IR");
        assert_eq!(
            diagnostic.code, "KES-C009",
            "unexpected error for {source:?}"
        );
        assert_eq!(diagnostic.field.as_deref(), Some("analysis"));
    }
}

#[test]
fn duplicate_analysis_kinds_fail_closed_until_selectors_exist() {
    for source in [
        "simulate op\nsimulate op\n",
        "simulate tran 1us 1ms\nsimulate tran 10us 2ms\n",
        "simulate ac dec 10 1Hz 1kHz\nsimulate ac lin 20 10Hz 2kHz\n",
        "source V1 5V\nsimulate dc V1 0V 5V 1V\nsimulate dc V1 0V 3V 500mV\n",
    ] {
        let program = parse_program(source).expect("duplicate analysis syntax should parse");
        let diagnostic = ast_to_ir(&program).expect_err("duplicate analysis kind reached typed IR");
        assert_eq!(
            diagnostic.code, "KES-C009",
            "unexpected error for {source:?}"
        );
        assert_eq!(diagnostic.field.as_deref(), Some("analysis"));
        assert!(diagnostic.message.contains("duplicate"));
    }
}
