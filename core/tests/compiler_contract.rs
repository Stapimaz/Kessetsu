use kessetsu_core::compiler::{
    COMPILE_SCHEMA_VERSION, CompileOptions, Diagnostic, DiagnosticSeverity, DiagnosticStage,
    compile_source,
};

const VALID_SOURCE: &str =
    "source V1 5V\nresistor R1 1k\nconnect V1.plus to R1.p1\nconnect V1.minus to R1.p2\n";

#[test]
fn successful_compile_returns_a_versioned_structured_report() {
    let report = compile_source(VALID_SOURCE, CompileOptions::all_outputs());

    assert_eq!(report.schema_version, COMPILE_SCHEMA_VERSION);
    assert!(!report.has_errors());
    assert!(report.diagnostics.is_empty());
    assert!(report.ast.is_some());
    assert!(report.ir.is_some());
    assert!(report.graph.is_some());
    assert!(report.spice_netlist.is_some());
    assert!(report.layout.is_some());
    assert!(report.schematic.is_some());
    assert!(report.schematic_svg.is_some());
    assert!(report.kicad_sch.is_some());
}

#[test]
fn default_options_keep_debug_and_schematic_outputs_opt_in() {
    let report = compile_source(VALID_SOURCE, CompileOptions::default());

    assert!(report.ast.is_none());
    assert!(report.spice_netlist.is_some());
    assert!(report.layout.is_none());
    assert!(report.schematic.is_none());
    assert!(report.schematic_svg.is_none());
    assert!(report.kicad_sch.is_none());
}

#[test]
fn parser_failure_is_structured_and_stops_downstream_stages() {
    let report = compile_source(
        "source V1 sine(0V, 1V, 1kHz\n",
        CompileOptions::all_outputs(),
    );

    assert!(report.has_errors());
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].code, "KES-P001");
    assert_eq!(report.diagnostics[0].stage, DiagnosticStage::Parse);
    assert_eq!(report.diagnostics[0].severity, DiagnosticSeverity::Error);
    assert!(report.diagnostics[0].line.is_some());
    assert!(report.ir.is_none());
    assert!(report.graph.is_none());
    assert!(report.spice_netlist.is_none());
    assert!(report.layout.is_none());
    assert!(report.schematic.is_none());
    assert!(report.schematic_svg.is_none());
    assert!(report.kicad_sch.is_none());
}

#[test]
fn flatten_failure_is_structured_and_stops_downstream_stages() {
    let report = compile_source("use Missing U1\n", CompileOptions::all_outputs());

    assert_eq!(report.diagnostics[0].code, "KES-C008");
    assert_eq!(report.diagnostics[0].stage, DiagnosticStage::Flatten);
    assert!(report.ir.is_none());
    assert!(report.spice_netlist.is_none());
}

#[test]
fn semantic_failure_preserves_ast_but_blocks_graph_and_backends() {
    let report = compile_source("resistor R1 nope\n", CompileOptions::all_outputs());

    assert_eq!(report.diagnostics[0].code, "KES-C001");
    assert_eq!(report.diagnostics[0].stage, DiagnosticStage::Semantic);
    assert_eq!(report.diagnostics[0].line, Some(1));
    assert!(report.diagnostics[0].column.is_some());
    assert!(report.ast.is_some());
    assert!(report.ir.is_none());
    assert!(report.graph.is_none());
    assert!(report.spice_netlist.is_none());
}

#[test]
fn erc_failure_preserves_ir_and_graph_but_blocks_backends() {
    let source = "resistor R1 1k\nconnect Missing.p1 to R1.p1\n";
    let report = compile_source(source, CompileOptions::all_outputs());

    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "KES-E002")
    );
    assert!(
        report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.stage == DiagnosticStage::Erc)
    );
    assert!(report.ir.is_some());
    assert!(
        report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.line.is_some())
    );
    assert!(report.graph.is_some());
    assert!(report.spice_netlist.is_none());
    assert!(report.layout.is_none());
    assert!(report.kicad_sch.is_none());
}

#[test]
fn graph_summary_is_stably_sorted() {
    let report = compile_source(VALID_SOURCE, CompileOptions::default());
    let graph = report
        .graph
        .expect("valid source should produce graph summary");

    assert!(graph.nets.windows(2).all(|nets| nets[0].id < nets[1].id));
    assert!(
        graph
            .nets
            .iter()
            .all(|net| net.pins.windows(2).all(|pins| pins[0] < pins[1]))
    );
}

#[test]
fn report_json_uses_stable_schema_and_lowercase_diagnostic_enums() {
    let report = compile_source("source V1 sine(0V, 1V, 1kHz\n", CompileOptions::default());
    let json = serde_json::to_value(report).expect("compile report should serialize");

    assert_eq!(json["schema_version"], COMPILE_SCHEMA_VERSION);
    assert_eq!(json["diagnostics"][0]["severity"], "error");
    assert_eq!(json["diagnostics"][0]["stage"], "parse");
}

#[test]
fn warning_diagnostics_do_not_turn_a_successful_report_into_an_error() {
    let mut report = compile_source(VALID_SOURCE, CompileOptions::default());
    report.diagnostics.push(Diagnostic {
        code: "KES-S004".to_string(),
        severity: DiagnosticSeverity::Warning,
        stage: DiagnosticStage::Erc,
        message: "Example non-blocking warning".to_string(),
        component: None,
        pin: None,
        field: None,
        line: None,
        column: None,
    });

    assert!(!report.has_errors());
    assert!(report.spice_netlist.is_some());
}

#[test]
fn shared_web_default_example_compiles_every_browser_output() {
    let source = include_str!("../../examples/demo_circuit.kess");
    assert!(!source.contains("battery"));

    let report = compile_source(source, CompileOptions::all_outputs());
    assert!(
        !report.has_errors(),
        "diagnostics: {:?}",
        report.diagnostics
    );
    assert!(report.spice_netlist.is_some());
    assert!(report.layout.is_some());
    assert!(report.kicad_sch.is_some());
}

#[test]
fn kicad_export_embeds_typed_source_symbols_and_pins() {
    let source = "source V1 5V\ncurrent_source I1 1A\nresistor R1 1k\nconnect V1.plus, I1.plus to R1.p1\nconnect V1.minus, I1.minus to R1.p2\n";
    let report = compile_source(source, CompileOptions::all_outputs());
    let kicad = report
        .kicad_sch
        .expect("valid source circuit should produce KiCad output");

    assert!(kicad.contains("Kessetsu:NL_V1"));
    assert!(kicad.contains("Kessetsu:NL_I1"));
    assert!(kicad.contains("(name \"plus\""));
    assert!(kicad.contains("(name \"minus\""));
}
