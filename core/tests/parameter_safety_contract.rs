mod common;
use common::TestWorkspace;
use kessetsu_core::compiler::{CompileOptions, DiagnosticStage, compile_source};
use kessetsu_core::expression::MAX_EXPRESSION_WORK;
use kessetsu_core::parser::{MAX_SOURCE_BYTES, MAX_SOURCE_STATEMENTS};

fn failure(source: &str) -> kessetsu_core::compiler::Diagnostic {
    let report = compile_source(source, CompileOptions::all_outputs());
    assert!(report.has_errors());
    assert!(report.ir.is_none());
    assert!(report.spice_netlist.is_none());
    assert!(report.schematic.is_none());
    report.diagnostics.into_iter().next().unwrap()
}

fn position(source: &str, needle: &str) -> (Option<usize>, Option<usize>) {
    let offset = source.find(needle).unwrap();
    let position = pest::Position::new(source, offset).unwrap().line_col();
    (Some(position.0), Some(position.1))
}

#[test]
fn module_defaults_use_actual_declarations_not_similarly_named_comments() {
    for source in [
        "// r is mentioned first\nmodule M() {\n  param r: Ohm = missing\n}\n",
        "// r is mentioned first\nmodule M() {\n  param r: Ohm = 1V\n}\n",
        "// r is mentioned first\nmodule M() {\n  param r: Ohm = 1k\n  param r: Ohm = 2k\n}\n",
    ] {
        let diagnostic = failure(source);
        let needle = if source.contains("= 2k") {
            "param r: Ohm = 2k"
        } else {
            "param r:"
        };
        assert_eq!(
            (diagnostic.line, diagnostic.column),
            position(source, needle)
        );
        assert_eq!(diagnostic.stage, DiagnosticStage::Flatten);
        assert_eq!(diagnostic.field.as_deref(), Some("r"));
    }
}

#[test]
fn module_override_errors_identify_the_exact_input_including_duplicates() {
    let module = "// r appears in a comment\nmodule M() {\n  param r: Ohm = 1k\n}\n";
    for (invocation, needle, field) in [
        ("use M X(missing=1k)\n", "missing=1k", "X.missing"),
        ("use M X(r=2k,r=3k)\n", "r=3k", "X.r"),
        ("use M X(r=1V)\n", "r=1V", "X.r"),
        ("use M X(r={1kOhm / 0})\n", "r={1kOhm / 0}", "X.r"),
    ] {
        let source = format!("{module}{invocation}");
        let diagnostic = failure(&source);
        assert_eq!(
            (diagnostic.line, diagnostic.column),
            position(&source, needle)
        );
        assert_eq!(diagnostic.field.as_deref(), Some(field));
    }
}

#[test]
fn nested_runtime_errors_keep_override_origins_and_default_provenance() {
    let source = "// r comes before the actual error\r\nmodule Child() {\r\n  param r: Ohm = 1k\r\n}\r\nmodule Parent() {\r\n  param denominator: ratio = 0\r\n  use Child Y(r={1kOhm / denominator})\r\n}\r\nuse Parent X\r\n";
    let diagnostic = failure(source);
    assert_eq!(diagnostic.stage, DiagnosticStage::Semantic);
    assert_eq!(diagnostic.field.as_deref(), Some("X.Y.r"));
    assert_eq!(
        (diagnostic.line, diagnostic.column),
        position(source, "r={1kOhm / denominator}")
    );
    let working = source.replace("ratio = 0", "ratio = 2");
    let report = compile_source(&working, CompileOptions::default());
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let parameter = report
        .ir
        .unwrap()
        .parameter_manifest
        .parameters
        .into_iter()
        .find(|parameter| parameter.instance_path == ["X", "Y"] && parameter.name == "r")
        .unwrap();
    assert_eq!(
        parameter.line, 3,
        "provenance still points to the declared default"
    );
    assert_eq!(parameter.expression, "1k");
    assert_eq!(parameter.resolved.value, 500.0);
}

#[test]
fn dependent_default_errors_point_to_the_formula_not_the_override() {
    let source = "// c is not this comment\nmodule RC() {\n  param r: Ohm = 1k\n  param c: F = 1 / (r * 1kHz)\n}\nuse RC X(r=0Ohm)\n";
    let diagnostic = failure(source);
    assert_eq!(diagnostic.field.as_deref(), Some("X.c"));
    assert_eq!(
        (diagnostic.line, diagnostic.column),
        position(source, "param c:")
    );
}

#[test]
fn duplicate_root_declarations_and_field_names_have_unambiguous_origins() {
    let source = "// r elsewhere\nparam r: Ohm = 1k\nparam r: Ohm = 2k\nparam r: Ohm = 3k\n";
    let diagnostic = failure(source);
    assert_eq!(diagnostic.code, "KES-C020");
    assert_eq!(
        (diagnostic.line, diagnostic.column),
        position(source, "param r: Ohm = 2k")
    );
    let diagnostic = failure("param value: ratio = 1\nsource VIN {value}\n");
    assert_eq!(diagnostic.component.as_deref(), Some("VIN"));
    assert_eq!(
        diagnostic.line,
        Some(2),
        "a parameter name must not steal a component field error"
    );
}

#[test]
fn unsupported_nested_waveforms_and_deep_arithmetic_fail_without_poisoning_compiler() {
    let nested = format!(
        "source VIN sine({}0V{},1V,1kHz)\n",
        "f(".repeat(4096),
        ")".repeat(4096)
    );
    assert_eq!(failure(&nested).stage, DiagnosticStage::Parse);
    for expression in [
        format!("{}1{}", "(".repeat(128), ")".repeat(128)),
        format!("{}1", "-".repeat(128)),
    ] {
        let diagnostic = failure(&format!("param x: ratio = {expression}\n"));
        assert_eq!(diagnostic.stage, DiagnosticStage::Parse);
        assert!(diagnostic.message.contains("nesting limit"));
    }
    let normal = "source V 1V\nresistor R 1k\nconnect V.plus to R.p1\nconnect V.minus to R.p2\n";
    assert!(!compile_source(normal, CompileOptions::default()).has_errors());
}

// A balanced 256-literal sum has 511 nodes; attaching an Ohm literal makes 513.
fn expression(depth: usize) -> String {
    if depth == 0 {
        "1".into()
    } else {
        let child = expression(depth - 1);
        format!("({child}+{child})")
    }
}

#[test]
fn aggregate_source_and_expanded_field_expressions_are_bounded() {
    let value = format!("{} * 1Ohm", expression(8));
    let count = MAX_EXPRESSION_WORK / 513 + 1;
    let source = (0..count)
        .map(|index| format!("resistor R{index} {{{value}}}\n"))
        .collect::<String>();
    let diagnostic = failure(&source);
    assert_eq!(diagnostic.stage, DiagnosticStage::Parse);
    assert!(diagnostic.message.contains("Source expression work limit"));
    drop(source);
    let mut source = format!("module M() {{\nresistor R {{{value}}}\n}}\n");
    for index in 0..count {
        source.push_str(&format!("use M X{index}\n"));
    }
    let diagnostic = failure(&source);
    assert_eq!(diagnostic.stage, DiagnosticStage::Flatten);
    assert!(diagnostic.message.contains("expression work limit"));
}

#[test]
fn oversized_source_and_statement_counts_fail_before_typed_ast_construction() {
    let diagnostic = failure(&"/".repeat(MAX_SOURCE_BYTES + 1));
    assert_eq!(diagnostic.stage, DiagnosticStage::Parse);
    assert!(diagnostic.message.contains("byte limit"));
    assert!(
        diagnostic.message.len() < 500,
        "diagnostics must not echo an oversized line"
    );
    let diagnostic = failure(&"net N\n".repeat(MAX_SOURCE_STATEMENTS + 1));
    assert_eq!(diagnostic.stage, DiagnosticStage::Parse);
    assert!(diagnostic.message.contains("statement limit"));
}

#[test]
fn cli_json_retains_precise_parameter_positions_and_no_backend_output() {
    let workspace = TestWorkspace::new("parameter-locations");
    let source =
        "// r misleading comment\nmodule M() {\nparam r: Ohm = 1k\n}\nuse M X(r={1kOhm/0})\n";
    let output = workspace.run_cli_with_stdin(
        &["check", "-", "--format", "json", "--include", "spice"],
        source,
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["diagnostics"][0]["line"], 5);
    assert_eq!(report["diagnostics"][0]["column"], 9);
    assert_eq!(report["diagnostics"][0]["field"], "X.r");
    assert!(report["debug"]["spice_netlist"].is_null());
    assert_eq!(std::fs::read_dir(workspace.path()).unwrap().count(), 0);
}
