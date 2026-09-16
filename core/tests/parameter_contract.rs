use kessetsu_core::expression::{MAX_EXPRESSION_DEPTH, parse_expression};
use kessetsu_core::ir::{ComponentParams, Quantity, SIUnit, SourceValue};
use kessetsu_core::{CompileOptions, compile_source, parse_program};
use std::collections::BTreeMap;

fn evaluate(text: &str, unit: SIUnit) -> Result<Quantity, String> {
    parse_expression(text)
        .map_err(|e| e.message)?
        .evaluate(&BTreeMap::new(), unit)
        .map_err(|e| e.message)
}

#[test]
fn operators_units_and_scientific_literals_have_unambiguous_semantics() {
    assert_eq!(evaluate("2 + 3 * 4", SIUnit::Ratio).unwrap().value, 14.0);
    assert_eq!(evaluate("8 / 2 / 2", SIUnit::Ratio).unwrap().value, 2.0);
    assert_eq!(evaluate("1e-3A * 1kOhm", SIUnit::Volt).unwrap().value, 1.0);
    assert_eq!(evaluate("-(12)", SIUnit::Volt).unwrap().value, -12.0);
    assert_eq!(evaluate("10k", SIUnit::Ohm).unwrap().value, 10_000.0);
    assert!(
        evaluate("2 * 5k", SIUnit::Ohm)
            .unwrap_err()
            .contains("ratio")
    );
    assert!(evaluate("1kOhm + 1000", SIUnit::Ohm).is_err());
    assert!(evaluate("1V", SIUnit::Ohm).is_err());
    let capacitance = evaluate("1 / (2 * pi * 1kOhm * 500Hz)", SIUnit::Farad).unwrap();
    assert!((capacitance.value - 318.3098861837907e-9).abs() < 1e-20);
}

#[test]
fn percentages_angles_and_range_checks_preserve_quantity_scales() {
    assert!(evaluate("1e-400", SIUnit::Ratio).is_err());
    assert!((evaluate("5% * 12V", SIUnit::Volt).unwrap().value - 0.6).abs() < 1e-14);
    assert_eq!(evaluate("5", SIUnit::Percent).unwrap().value, 5.0);
    assert_eq!(evaluate("0.05 * 1", SIUnit::Percent).unwrap().value, 5.0);
    assert!((evaluate("90deg * 2", SIUnit::Degree).unwrap().value - 180.0).abs() < 1e-12);
    assert!(evaluate("90deg", SIUnit::Ratio).is_err());
    for text in [
        "1 / 0",
        "1e308 * 10",
        "1e-300 * 1e-300",
        "1e308T",
        "1e-320p",
    ] {
        assert!(evaluate(text, SIUnit::Ratio).is_err(), "accepted {text}");
    }
}

#[test]
fn invalid_and_large_expressions_fail_without_recursive_parser_exhaustion() {
    for text in [
        "",
        "1\n+2",
        "1V 2V",
        "1V; .control",
        "sqrt(4)",
        "1e-",
        "2(3)",
        "{}",
        "\"1V\"",
        "1 +",
        "π",
    ] {
        assert!(parse_expression(text).is_err(), "accepted {text}");
    }
    let nested = format!(
        "{}1{}",
        "(".repeat(MAX_EXPRESSION_DEPTH + 1),
        ")".repeat(MAX_EXPRESSION_DEPTH + 1)
    );
    assert!(parse_expression(&nested).is_err());
    assert!(parse_expression(&format!("{}1", "-".repeat(MAX_EXPRESSION_DEPTH + 1))).is_err());
}

#[test]
fn parameters_resolve_forward_dependencies_and_record_sorted_provenance() {
    let source = "param rf: Ohm = (gain - 1) * rg\nparam rg: Ohm = 10k\nparam gain: ratio = 20\nresistor RF {rf}\n";
    let report = compile_source(source, CompileOptions::default());
    let ir = report.ir.unwrap(); // ERC's floating pin errors do not erase the semantic IR.
    assert_eq!(
        ir.parameter_manifest.schema_version,
        "kessetsu.parameters.v1"
    );
    let records = &ir.parameter_manifest.parameters;
    assert_eq!(
        records.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        ["gain", "rf", "rg"]
    );
    assert_eq!(records[1].resolved.value, 190_000.0);
    assert_eq!(records[1].dependencies, ["gain", "rg"]);
    assert_eq!(records[1].line, 1);
    assert_eq!(records[1].expression, "(gain - 1) * rg");
}

const PARAMETRIC: &str = "param supply: V = 12V\nparam r: Ohm = 10k\nparam gain: ratio = 20\nnet GND\nnet IN\nnet OUT\nsource V1 {(supply - 2V) / 2}\nresistor R1 {(gain - 1) * r}\nresistor R2 {r}\nconnect V1.plus, R1.p1 to IN\nconnect R1.p2, R2.p1 to OUT\nconnect V1.minus, R2.p2 to GND\nsimulate op\nassert value(V(OUT)) > 240mV\n";
const LITERAL: &str = "net GND\nnet IN\nnet OUT\nsource V1 5V\nresistor R1 190k\nresistor R2 10k\nconnect V1.plus, R1.p1 to IN\nconnect R1.p2, R2.p1 to OUT\nconnect V1.minus, R2.p2 to GND\nsimulate op\nassert value(V(OUT)) > 240mV\n";

#[test]
fn parameter_circuits_reach_the_same_ir_spice_and_drawing_as_literal_circuits() {
    let parametric = compile_source(PARAMETRIC, CompileOptions::all_outputs());
    let literal = compile_source(LITERAL, CompileOptions::all_outputs());
    assert!(!parametric.has_errors(), "{:?}", parametric.diagnostics);
    assert!(!literal.has_errors());
    let mut ir = parametric.ir.clone().unwrap();
    ir.parameter_manifest = Default::default();
    assert_eq!(ir, literal.ir.clone().unwrap());
    assert_eq!(parametric.spice_netlist, literal.spice_netlist);
    assert_eq!(parametric.schematic_svg, literal.schematic_svg);
    assert_eq!(parametric.schematic, literal.schematic);
    assert_eq!(parametric.kicad_sch, literal.kicad_sch);
    let json = serde_json::to_string(&literal.ir).unwrap();
    assert!(!json.contains("parameter_manifest"));
    assert!(
        !serde_json::to_string(&literal.ast)
            .unwrap()
            .contains("value_expression")
    );
}

#[test]
fn parameter_failures_have_actionable_diagnostics_and_no_backend_artifacts() {
    for (source, code, message) in [
        ("param a: V = missing\n", "KES-C021", "unknown parameter"),
        (
            "param a: V = b\nparam b: V = a\n",
            "KES-C023",
            "a -> b -> a",
        ),
        (
            "param a: V = 1V\nparam a: V = 2V\n",
            "KES-C020",
            "Duplicate",
        ),
        ("param pi: ratio = 3\n", "KES-C020", "reserved"),
        ("param a: Ohm = 12V\n", "KES-C022", "Expected Ohm"),
    ] {
        let report = compile_source(source, CompileOptions::all_outputs());
        assert!(report.has_errors());
        assert_eq!(report.diagnostics[0].code, code);
        assert!(report.diagnostics[0].message.contains(message));
        assert!(report.diagnostics[0].line.is_some());
        assert!(report.ir.is_none());
        assert!(report.spice_netlist.is_none());
        assert!(report.schematic.is_none());
    }
    assert!(parse_program("param x: bananas = 2\n").is_err());
    assert!(parse_program("param x: V = 1V +\nsource V1 2V\n").is_err());
}

#[test]
fn numeric_expressions_do_not_reinterpret_models_or_capture_globals_inside_modules() {
    let model = compile_source(
        "param x: V = 12V\ndiode D1 {x}\n",
        CompileOptions::default(),
    );
    assert!(model.has_errors());
    assert_eq!(model.diagnostics[0].code, "KES-C022");
    assert!(model.spice_netlist.is_none());
    let scoped = compile_source(
        "param r: Ohm = 1k\nmodule M(p1,p2) {\nresistor R {r}\n}\nuse M X\n",
        CompileOptions::default(),
    );
    assert!(scoped.has_errors());
    assert!(scoped.diagnostics[0].message.contains("instance-scoped"));
    let recursive = compile_source(
        "module M(p1,p2) {\nuse M X\n}\nuse M X\n",
        CompileOptions::default(),
    );
    assert!(recursive.has_errors());
    assert!(
        recursive.diagnostics[0]
            .message
            .contains("Recursive module")
    );
    let duplicated = compile_source(
        "module M() {\n}\nmodule M() {\n}\n",
        CompileOptions::default(),
    );
    assert!(duplicated.has_errors());
    assert!(
        duplicated.diagnostics[0]
            .message
            .contains("Duplicate module")
    );
}

#[test]
fn independent_requirements_stay_assertion_only_and_current_sources_accept_quantities() {
    assert!(
        kessetsu_core::requirements::compile_requirements(
            b"param limit: V = 2V\nassert value(V(OUT)) < 2V\n"
        )
        .is_err()
    );
    assert!(
        kessetsu_core::requirements::compile_requirements(b"assert value(V(OUT)) < {supply}\n")
            .is_err()
    );
    let source = "param current: A = 1mA\nnet GND\nnet OUT\ncurrent_source I1 {current}\nresistor R1 1k\nconnect I1.plus, R1.p1 to OUT\nconnect I1.minus, R1.p2 to GND\nsimulate op\n";
    let report = compile_source(source, CompileOptions::default());
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    assert!(matches!(&report.ir.unwrap().components[0].parameters,
        ComponentParams::CurrentSource { value: SourceValue::Dc(quantity) } if quantity.value == 0.001));
}
