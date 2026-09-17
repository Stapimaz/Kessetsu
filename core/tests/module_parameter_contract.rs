use kessetsu_core::ir::{CircuitIR, ComponentParams, ast_to_ir};
use kessetsu_core::{CompileOptions, compile_source, parse_program};

fn ir(source: &str) -> Result<CircuitIR, String> {
    let parsed = parse_program(source).map_err(|error| error.to_string())?;
    let flat = parsed.flatten()?;
    ast_to_ir(&flat).map_err(|error| error.message)
}

fn value(ir: &CircuitIR, id: &str) -> f64 {
    match &ir
        .components
        .iter()
        .find(|component| component.id == id)
        .unwrap()
        .parameters
    {
        ComponentParams::TwoPinPassive { value } => value.value,
        _ => panic!("not passive"),
    }
}

#[test]
fn two_rc_instances_recompute_defaults_and_keep_stable_identity() {
    let source = include_str!("fixtures/parameters/rc_instances.kess");
    let report = compile_source(source, CompileOptions::all_outputs());
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let original = report.ir.unwrap();
    let schematic = report.schematic.unwrap();
    assert_eq!(schematic.components.len(), 5);
    assert_eq!(schematic.connectivity.expected_connected_pins, 10);
    assert!((value(&original, "SLOW_C1") - 318.3098861837907e-9).abs() < 1e-20);
    assert!((value(&original, "FAST_C1") - 79.57747154594768e-9).abs() < 1e-20);
    let edited = ir(&source.replace("base_cutoff * 4", "base_cutoff * 2")).unwrap();
    assert_eq!(value(&original, "SLOW_C1"), value(&edited, "SLOW_C1"));
    assert_eq!(
        original
            .components
            .iter()
            .map(|c| &c.id)
            .collect::<Vec<_>>(),
        edited.components.iter().map(|c| &c.id).collect::<Vec<_>>()
    );
    let record = original
        .parameter_manifest
        .parameters
        .iter()
        .find(|p| p.instance_path == ["FAST"] && p.name == "cutoff")
        .unwrap();
    assert_eq!(record.expression, "1k");
    assert_eq!(
        record.effective_override.as_deref(),
        Some("base_cutoff * 4")
    );
    assert_eq!(record.dependencies, ["base_cutoff"]);
    assert_eq!(
        original.parameter_manifest.parameters[0].name,
        "base_cutoff"
    );
}

#[test]
fn nested_overrides_use_the_caller_not_the_child_scope() {
    let source = "param r: Ohm = 10k\nmodule Child(a,b) {\nparam r: Ohm = 1k\nresistor R {r}\nconnect a to R.p1\nconnect R.p2 to b\n}\nmodule Parent(a,b) {\nparam r: Ohm = 2k\nuse Child C(r={r * 3})\nconnect a to C.a\nconnect C.b to b\n}\nuse Parent P\nuse Child C(r={r * 2})\n";
    let result = ir(source).unwrap();
    assert_eq!(value(&result, "P_C_R"), 6000.0);
    assert_eq!(value(&result, "C_R"), 20_000.0);
    let nested = result
        .parameter_manifest
        .parameters
        .iter()
        .find(|p| p.instance_path == ["P", "C"])
        .unwrap();
    assert_eq!(nested.dependencies, ["P.r"]);
}

#[test]
fn override_breaks_effective_cycle_but_cannot_hide_bad_names_or_types() {
    let cyclic = "module M(a,b) {\nparam r: Ohm = s\nparam s: Ohm = r\nresistor R {s}\n}\n";
    assert!(
        ir(&format!("{cyclic}use M X\n"))
            .unwrap_err()
            .contains("cycle")
    );
    let result = ir(&format!("{cyclic}use M X(r=2k)\n")).unwrap();
    assert_eq!(value(&result, "X_R"), 2000.0);
    for default in ["unknown", "1V", "missing * 1Ohm"] {
        let source = format!("module M() {{\nparam r: Ohm = {default}\n}}\nuse M X(r=1k)\n");
        assert!(
            ir(&source).is_err(),
            "accepted hidden invalid default {default}"
        );
    }
}

#[test]
fn unknown_duplicate_and_wrong_unit_overrides_fail_closed() {
    let module = "module M(a,b) {\nparam r: Ohm = 1k\nresistor R {r}\n}\n";
    for arguments in [
        "unknown=1k",
        "r=1k,r=2k",
        "r=1V",
        "r={1k * 2}",
        "r={global}",
    ] {
        let source = format!("{module}use M X({arguments})\n");
        let report = compile_source(&source, CompileOptions::all_outputs());
        assert!(report.has_errors(), "accepted {arguments}");
        assert!(report.spice_netlist.is_none());
        assert!(report.schematic.is_none());
        assert!(report.diagnostics[0].message.contains("X"));
    }
    assert!(parse_program(&format!("{module}use M X(r={{1kOhm +}})\n")).is_err());
    assert!(parse_program(&format!("{module}use M X(r=global)\n")).is_err());
    assert!(parse_program(&format!("{module}use M X(r={{1kOhm\n}})\n")).is_err());
    let result = ir(&format!("{module}use M X(r=2k)\n")).unwrap();
    assert_eq!(value(&result, "X_R"), 2000.0);
}

#[test]
fn module_ports_local_nets_and_flattened_path_collisions_are_explicit() {
    let module = "module M(a,b) {\nnet MID\nresistor R1 1k\nresistor R2 1k\nconnect a to R1.p1\nconnect R1.p2, R2.p1 to MID\nconnect R2.p2 to b\n}\n";
    let source = format!(
        "{module}net GND\nnet IN\nsource V1 1V\nuse M X\nconnect V1.plus, X.a to IN\nconnect V1.minus, X.b to GND\n"
    );
    let report = compile_source(&source, CompileOptions::default());
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    assert!(report.spice_netlist.unwrap().contains("X_MID"));
    let invalid = compile_source(&source.replace("X.a", "X.typo"), CompileOptions::default());
    assert!(invalid.diagnostics.iter().any(|d| d.code == "KES-E005"));
    let collision = "module Leaf() {\nresistor R 1k\n}\nmodule Outer() {\nuse Leaf B\n}\nuse Leaf A_B\nuse Outer A\n";
    assert!(ir(collision).unwrap_err().contains("identifier collision"));
    assert!(
        ir("module M(a,a) {\n}\n")
            .unwrap_err()
            .contains("duplicate interface")
    );
}

#[test]
fn a_default_module_matches_its_literal_ir_and_all_backends() {
    let capacitance = 1.0 / (2.0 * std::f64::consts::PI * 1000.0 * 1000.0);
    let parametric = "module RC(a,b,gnd) {\nparam r: Ohm = 1k\nparam c: F = 1 / (2 * pi * r * 1kHz)\nresistor R {r}\ncapacitor C {c}\nconnect a to R.p1\nconnect R.p2, C.p1 to b\nconnect C.p2 to gnd\n}\nnet GND\nnet IN\nnet OUT\nsource VIN ac(1V)\nuse RC X\nconnect VIN.plus, X.a to IN\nconnect VIN.minus, X.gnd to GND\nconnect X.b to OUT\nsimulate ac dec 20 1Hz 100kHz\n";
    let literal = parametric
        .replace("param r: Ohm = 1k\n", "")
        .replace("param c: F = 1 / (2 * pi * r * 1kHz)\n", "")
        .replace("resistor R {r}", "resistor R 1k")
        .replace("capacitor C {c}", &format!("capacitor C {capacitance}F"));
    let mut parameter_report = compile_source(parametric, CompileOptions::all_outputs());
    let literal_report = compile_source(&literal, CompileOptions::all_outputs());
    assert!(
        !parameter_report.has_errors(),
        "{:?}",
        parameter_report.diagnostics
    );
    assert!(!literal_report.has_errors());
    parameter_report.ir.as_mut().unwrap().parameter_manifest = Default::default();
    assert_eq!(parameter_report.ir, literal_report.ir);
    assert_eq!(parameter_report.spice_netlist, literal_report.spice_netlist);
    assert_eq!(parameter_report.schematic, literal_report.schematic);
    assert_eq!(parameter_report.schematic_svg, literal_report.schematic_svg);
    assert_eq!(parameter_report.kicad_sch, literal_report.kicad_sch);
}

#[test]
fn scoped_context_limits_and_unsupported_module_analyses_fail_explicitly() {
    let source = "module M() {\nparam r: Ohm = 1k\nresistor R {r}\nsimulate op\n}\nuse M X\n";
    assert!(ir(source).unwrap_err().contains("circuit root"));
    let mut chain = String::new();
    for index in 0..65 {
        chain.push_str(&format!("module M{index}() {{\nuse M{} X\n}}\n", index + 1));
    }
    chain.push_str("module M65() {\nresistor R 1k\n}\nuse M0 ROOT\n");
    assert!(ir(&chain).unwrap_err().contains("nesting limit"));
}
