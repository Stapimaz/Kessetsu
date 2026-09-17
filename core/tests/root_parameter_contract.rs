mod common;
use common::{TestWorkspace, read_fixture};
use kessetsu_core::models::ExternalModelResources;
use kessetsu_core::{
    CompileInputs, CompileOptions, ParameterInput, compile_source, compile_source_with_inputs,
};
use serde_json::Value;

fn inputs(values: &[(&str, &str)]) -> CompileInputs {
    CompileInputs {
        parameters: values
            .iter()
            .map(|(name, value)| ParameterInput {
                name: (*name).into(),
                value: (*value).into(),
            })
            .collect(),
        ..CompileInputs::default()
    }
}

fn compile(source: &str, values: &[(&str, &str)]) -> kessetsu_core::CompileReport {
    compile_source_with_inputs(
        source,
        CompileOptions::all_outputs(),
        &inputs(values),
        &ExternalModelResources::new(),
    )
}

#[test]
fn root_inputs_match_materialized_source_and_preserve_original_provenance() {
    let source = read_fixture("parameters/assertions.kess");
    let report = compile(&source, &[("frequency", "2kHz"), ("amplitude", "2V")]);
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let effective = report.effective_source.as_ref().unwrap();
    assert_eq!(
        *effective,
        source
            .replace("param frequency: Hz = 1kHz", "param frequency: Hz = 2kHz")
            .replace("param amplitude: V = 1V", "param amplitude: V = 2V")
    );
    let reference = compile_source(effective, CompileOptions::all_outputs());
    assert!(!reference.has_errors(), "{:?}", reference.diagnostics);
    assert_eq!(report.spice_netlist, reference.spice_netlist);
    assert_eq!(report.graph, reference.graph);
    assert_eq!(report.schematic, reference.schematic);
    assert_eq!(report.kicad_sch, reference.kicad_sch);
    let actual = report.ir.unwrap();
    let expected = reference.ir.unwrap();
    assert_eq!(actual.components, expected.components);
    assert_eq!(actual.analyses, expected.analyses);
    assert_eq!(actual.assertions, expected.assertions);
    let frequency = actual
        .parameter_manifest
        .parameters
        .iter()
        .find(|p| p.name == "frequency")
        .unwrap();
    assert_eq!(frequency.expression, "1kHz");
    assert_eq!(frequency.effective_override.as_deref(), Some("2kHz"));
    assert_eq!(frequency.resolved.value, 2000.0);
    let duration = actual
        .parameter_manifest
        .parameters
        .iter()
        .find(|p| p.name == "duration")
        .unwrap();
    assert_eq!(duration.resolved.value, 0.005);
    assert_eq!(
        compile(&source, &[("amplitude", "2V"), ("frequency", "2kHz")])
            .effective_source
            .as_ref(),
        Some(effective)
    );
}

#[test]
fn materialization_preserves_comments_unicode_crlf_modules_and_formulas() {
    let source = "// µF unchanged\r\nparam resistance: Ohm = 1kOhm  // root\r\nparam capacitance: F = 1µF\r\nparam calculated: Ohm = resistance * 2\r\nmodule Load(a,b) {\r\n  param resistance: Ohm = 3kOhm // module default\r\n  resistor R {resistance}\r\n  connect a to R.p1\r\n  connect R.p2 to b\r\n}\r\nuse Load X(resistance={calculated})\r\nsource V 1V\r\nconnect V.plus to X.a\r\nconnect V.minus to X.b\r\n";
    let report = compile(source, &[("resistance", "2kΩ"), ("capacitance", "2µF")]);
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    assert_eq!(
        report.effective_source.unwrap(),
        source
            .replace("= 1kOhm  // root", "= 2kΩ  // root")
            .replace("= 1µF", "= 2µF")
    );
    let ir = report.ir.unwrap();
    let child = ir
        .parameter_manifest
        .parameters
        .iter()
        .find(|p| p.instance_path == ["X"] && p.name == "resistance")
        .unwrap();
    assert_eq!(child.resolved.value, 4000.0);
}

#[test]
fn root_override_can_break_effective_cycle_but_cannot_hide_bad_defaults() {
    let cycle = "param a: Ohm = b\nparam b: Ohm = a\n";
    let report = compile(cycle, &[("a", "1k")]);
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    assert!(
        report
            .ir
            .unwrap()
            .parameter_manifest
            .parameters
            .iter()
            .all(|p| p.resolved.value == 1000.0)
    );
    for source in [
        "param a: Ohm = missing\n",
        "param a: Ohm = 1V\n",
        "param a: Ohm = 1k\nparam a: Ohm = 2k\n",
    ] {
        let invalid = compile(source, &[("a", "1k")]);
        assert!(invalid.has_errors());
        assert!(invalid.spice_netlist.is_none());
        assert!(invalid.effective_source.is_none());
    }
}

#[test]
fn unknown_duplicate_nonliteral_and_invalid_units_fail_closed() {
    let source = read_fixture("parameters/assertions.kess");
    for values in [
        vec![("missing", "1V")],
        vec![("amplitude", "2V"), ("amplitude", "3V")],
        vec![("amplitude", "1A")],
        vec![("amplitude", "1V+1V")],
        vec![("amplitude", "{1V}")],
        vec![("amplitude", "frequency")],
        vec![("amplitude", "1e999V")],
        vec![("amplitude", "1V\nsource Evil 1V")],
        vec![("X.amplitude", "1V")],
        vec![("Amplitude", "1V")],
    ] {
        let report = compile(&source, &values);
        assert!(report.has_errors(), "{values:?}");
        assert!(report.ir.is_none());
        assert!(report.spice_netlist.is_none());
        assert!(report.schematic.is_none());
        assert!(report.effective_source.is_none());
    }
    let mut wrong_version = inputs(&[]);
    wrong_version.schema_version = "kessetsu.inputs.v99".into();
    let report = compile_source_with_inputs(
        "not source",
        CompileOptions::default(),
        &wrong_version,
        &ExternalModelResources::new(),
    );
    assert_eq!(
        report.diagnostics[0].stage,
        kessetsu_core::compiler::DiagnosticStage::Semantic
    );
    let report = compile(&source, &[("amplitude", "1A")]);
    assert_eq!(report.diagnostics[0].line, Some(2));
}

#[test]
fn empty_inputs_preserve_existing_contract_and_scaled_quantities_resolve_once() {
    let source = read_fixture("parameters/assertions.kess");
    assert_eq!(
        serde_json::to_value(compile_source_with_inputs(
            &source,
            CompileOptions::default(),
            &inputs(&[]),
            &ExternalModelResources::new()
        ))
        .unwrap(),
        serde_json::to_value(compile_source(&source, CompileOptions::default())).unwrap()
    );
    let report = compile(
        "param fraction: ratio = 1\nparam percentage: percent = 1%\nparam angle: deg = 0deg\n",
        &[
            ("fraction", "5%"),
            ("percentage", "0.1"),
            ("angle", "-45deg"),
        ],
    );
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let records = report.ir.unwrap().parameter_manifest.parameters;
    assert_eq!(
        records
            .iter()
            .find(|p| p.name == "fraction")
            .unwrap()
            .resolved
            .value,
        0.05
    );
    assert_eq!(
        records
            .iter()
            .find(|p| p.name == "percentage")
            .unwrap()
            .resolved
            .value,
        0.1
    );
    assert_eq!(
        records
            .iter()
            .find(|p| p.name == "angle")
            .unwrap()
            .resolved
            .value,
        -45.0
    );
}

fn json(output: &std::process::Output) -> Value {
    assert!(output.stderr.is_empty(), "{:?}", output.stderr);
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn cli_inputs_are_compact_repeatable_source_safe_and_exportable() {
    let workspace = TestWorkspace::new("root-input-cli");
    let source = read_fixture("parameters/assertions.kess");
    let path = workspace.write("circuit.kess", &source);
    let compact = workspace.run_cli(&[
        "check",
        "circuit.kess",
        "--param",
        "amplitude=2V",
        "--format",
        "json",
    ]);
    assert_eq!(compact.status.code(), Some(0));
    assert!(json(&compact).get("debug").is_none());
    let args = [
        "compile",
        "-",
        "--param",
        "amplitude=2V",
        "--include",
        "effective-source,ir,spice",
        "--format",
        "json",
    ];
    let first = workspace.run_cli_with_stdin(&args, &source);
    let second = workspace.run_cli_with_stdin(&args, &source);
    assert_eq!(first.status.code(), Some(0));
    assert_eq!(first.stdout, second.stdout);
    let result = json(&first);
    let effective = result["debug"]["effective_source"].as_str().unwrap();
    assert!(effective.contains("param amplitude: V = 2V"));
    let replay = workspace.run_cli_with_stdin(
        &["compile", "-", "--format", "json", "--include", "spice"],
        effective,
    );
    assert_eq!(
        json(&replay)["debug"]["spice_netlist"],
        result["debug"]["spice_netlist"]
    );
    let original = workspace.run_cli_with_stdin(
        &[
            "check",
            "-",
            "--include",
            "effective-source",
            "--format",
            "json",
        ],
        &source,
    );
    assert_eq!(json(&original)["debug"]["effective_source"], source);
    let exported = workspace.run_cli(&[
        "export",
        "circuit.kess",
        "--param",
        "amplitude=2V",
        "--target",
        "spice",
        "--output",
        "effective.spice",
        "--format",
        "json",
    ]);
    assert_eq!(exported.status.code(), Some(0));
    assert_eq!(
        std::fs::read_to_string(workspace.path().join("effective.spice")).unwrap(),
        result["debug"]["spice_netlist"].as_str().unwrap()
    );
    assert_eq!(std::fs::read_to_string(path).unwrap(), source);
    assert!(!workspace.path().join("circuit.spice").exists());
}

#[test]
fn invalid_cli_inputs_do_not_write_outputs_or_launch_simulator() {
    let workspace = TestWorkspace::new("root-input-errors");
    let source = read_fixture("parameters/assertions.kess");
    workspace.write("circuit.kess", &source);
    let absent_simulator = workspace.path().join("nonexistent-simulator.exe");
    for args in [
        vec![
            "test",
            "circuit.kess",
            "--param",
            "missing=1V",
            "--format",
            "json",
        ],
        vec![
            "compile",
            "circuit.kess",
            "--param",
            "amplitude=1A",
            "--format",
            "json",
        ],
        vec![
            "test",
            "circuit.kess",
            "--param",
            "amplitude=1V",
            "--param",
            "amplitude=2V",
            "--format",
            "json",
        ],
        vec![
            "test",
            "circuit.kess",
            "--param",
            "malformed",
            "--format",
            "json",
        ],
    ] {
        let output = workspace.run_cli_with_env(&args, "KESSETSU_NGSPICE", &absent_simulator);
        assert!(matches!(output.status.code(), Some(1 | 2)));
        assert_eq!(json(&output)["status"], "error");
        assert!(!workspace.path().join("circuit.spice").exists());
        assert_eq!(std::fs::read_dir(workspace.path()).unwrap().count(), 1);
    }
    let output = workspace.run_cli(&[
        "tool",
        "rc-lowpass",
        "--cutoff",
        "1kHz",
        "--resistance",
        "1k",
        "--param",
        "cutoff=2kHz",
        "--format",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(json(&output)["diagnostics"][0]["code"], "KES-F002");
}
