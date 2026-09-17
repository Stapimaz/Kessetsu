mod common;
use common::TestWorkspace;
use kessetsu_core::ir::ComponentParams;
use kessetsu_core::{CompileOptions, compile_source};

const FILTERS: &str = include_str!("../../examples/reusable_filters.kess");
const AMPLIFIERS: &str = include_str!("../../examples/reusable_amplifiers.kess");

#[test]
fn structured_module_identity_survives_without_parameters_or_extra_symbols() {
    let source = "module Child(a,b) {\nresistor R 1k\nconnect a to R.p1\nconnect R.p2 to b\n}\nmodule Parent(a,b) {\nuse Child C\nconnect a to C.a\nconnect C.b to b\n}\nsource V 1V\nuse Parent P\nconnect V.plus to P.a\nconnect V.minus to P.b\n";
    let report = compile_source(source, CompileOptions::all_outputs());
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let ir = report.ir.unwrap();
    assert!(ir.parameter_manifest.is_empty());
    for (id, definition, path) in [("P", "Parent", vec!["P"]), ("P_C", "Child", vec!["P", "C"])] {
        let component = ir.components.iter().find(|c| c.id == id).unwrap();
        let ComponentParams::ModulePort {
            module_name,
            pins,
            instance_path,
        } = &component.parameters
        else {
            panic!("not module interface")
        };
        assert_eq!(module_name, definition);
        assert_eq!(pins, &["a", "b"]);
        assert_eq!(instance_path, &path);
    }
    assert_eq!(report.schematic.unwrap().components.len(), 2);
    let serialized = serde_json::to_value(&ir).unwrap();
    assert!(serialized.to_string().contains("instance_path"));
    let legacy: ComponentParams =
        serde_json::from_str(r#"{"ModulePort":{"module_name":"Child","pins":["a","b"]}}"#).unwrap();
    assert!(
        matches!(legacy, ComponentParams::ModulePort { instance_path, .. } if instance_path.is_empty())
    );
}

#[test]
fn public_block_examples_compile_all_outputs_and_keep_identity_after_parameter_edits() {
    for (source, edited, expected_symbols) in [
        (FILTERS, FILTERS.replace("Hz = 2kHz", "Hz = 1kHz"), 5),
        (
            AMPLIFIERS,
            AMPLIFIERS.replace("ratio = 10", "ratio = 8"),
            11,
        ),
    ] {
        let original = compile_source(source, CompileOptions::all_outputs());
        let changed = compile_source(&edited, CompileOptions::all_outputs());
        assert!(!original.has_errors(), "{:?}", original.diagnostics);
        assert!(!changed.has_errors(), "{:?}", changed.diagnostics);
        let original_ir = original.ir.unwrap();
        let changed_ir = changed.ir.unwrap();
        assert_eq!(
            original_ir
                .components
                .iter()
                .map(|c| &c.id)
                .collect::<Vec<_>>(),
            changed_ir
                .components
                .iter()
                .map(|c| &c.id)
                .collect::<Vec<_>>()
        );
        assert_eq!(original.graph, changed.graph);
        assert_eq!(
            original_ir.assertions, changed_ir.assertions,
            "changing a design setting must not retune acceptance limits"
        );
        assert_eq!(
            original.schematic.unwrap().components.len(),
            expected_symbols
        );
        assert!(original.kicad_sch.is_some());
        assert_eq!(
            original_ir
                .components
                .iter()
                .filter(|c| matches!(c.parameters, ComponentParams::ModulePort { .. }))
                .count(),
            2
        );
    }
}

#[test]
fn cli_block_examples_simulate_and_wrong_gain_fails_fixed_limits() {
    let workspace = TestWorkspace::new("reusable-blocks");
    for (source, count) in [(FILTERS, 4), (AMPLIFIERS, 6)] {
        let result = workspace.run_cli_with_stdin(&["test", "-", "--format", "json"], source);
        assert_eq!(
            result.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        let assertions = report["assertions"]["assertions"].as_array().unwrap();
        assert_eq!(assertions.len(), count);
        assert!(assertions.iter().all(|a| a["status"] == "PASS"));
    }
    let result = workspace.run_cli_with_stdin(
        &["test", "-", "--param", "second_gain=8", "--format", "json"],
        AMPLIFIERS,
    );
    assert_eq!(result.status.code(), Some(4));
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    let assertions = report["assertions"]["assertions"].as_array().unwrap();
    assert_eq!(assertions[0]["status"], "PASS");
    assert_eq!(assertions[1]["status"], "PASS");
    assert_eq!(assertions[2]["status"], "FAIL");
    assert_eq!(std::fs::read_dir(workspace.path()).unwrap().count(), 0);
}
