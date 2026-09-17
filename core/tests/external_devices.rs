mod common;
use common::TestWorkspace;
use kessetsu_core::compiler::{CompileOptions, compile_source_with_resources};
use kessetsu_core::exporter::{ExportFormat, ExportOptions, export_report};
use kessetsu_core::graph::NetlistGraph;
use kessetsu_core::model_resources::{
    browser_analysis_with_resources, resource_requirements, validate_browser_library,
    validate_resource_bindings,
};
use kessetsu_core::models::ExternalModelResources;
use kessetsu_core::sim_result::evaluate_assertions;
use kessetsu_core::simulation::{
    CancellationToken, NativeSimulationContext, NgspiceRunner, SimulationRequest,
};
use sha2::{Digest, Sha256};
use std::path::Path;

fn fixture(name: &str) -> (String, ExternalModelResources) {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");
    let source = std::fs::read_to_string(examples.join(format!("external_{name}.kess"))).unwrap();
    let requirements = resource_requirements(&source).unwrap();
    let resources = requirements
        .into_iter()
        .map(|r| {
            let bytes = std::fs::read(examples.join(&r.resource)).unwrap();
            assert_eq!(format!("{:x}", Sha256::digest(&bytes)), r.sha256);
            (r.resource, bytes)
        })
        .collect();
    (source, resources)
}

#[test]
fn device_descriptors_preserve_pins_metadata_and_all_exports_without_model_body_leaks() {
    for name in ["comparator", "memristor"] {
        let (source, resources) = fixture(name);
        let report =
            compile_source_with_resources(&source, CompileOptions::all_outputs(), &resources);
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let circuit = report.ir.as_ref().unwrap();
        let device = circuit
            .components
            .iter()
            .find(|c| matches!(c.kind, kessetsu_core::ir::ComponentKind::ExternalDevice(_)))
            .unwrap();
        assert_eq!(
            kessetsu_core::component::component_definition(&device.kind)
                .pins
                .len(),
            if name == "comparator" { 5 } else { 2 }
        );
        let serialized = serde_json::to_string(&report).unwrap();
        assert!(!serialized.contains("Bdrive"));
        assert!(!serialized.contains("Bx 0 x"));
        for format in [
            ExportFormat::Svg,
            ExportFormat::Png,
            ExportFormat::Pdf,
            ExportFormat::SchematicJson,
            ExportFormat::Spice,
            ExportFormat::Kicad,
            ExportFormat::Ltspice,
        ] {
            let artifact = export_report(&report, format, ExportOptions::default()).unwrap();
            assert!(artifact.connectivity_verified);
        }
        let schematic = report.schematic.as_ref().unwrap();
        assert!(
            schematic.quality.passed,
            "{}: {:?}",
            name, schematic.quality
        );
        let browser = browser_analysis_with_resources(
            circuit,
            &NetlistGraph::build(circuit),
            &circuit.analyses[0],
            &resources,
        )
        .unwrap();
        assert!(!browser.contains(".include"));
        assert!(browser.contains(if name == "comparator" {
            "Bdrive"
        } else {
            "Bx 0 x"
        }));
    }
}

#[test]
fn native_bound_models_match_independently_written_reference_topologies() {
    let runner = NgspiceRunner::discover();
    for (name, topology) in [
        (
            "comparator",
            "V_VP VCC 0 5\nV_VIN IN 0 SINE(0 0.1 1000)\nXU1 IN 0 VCC 0 OUT KESSETSU_COMPARATOR_V1\nR_RL OUT 0 10000\n.include \"models/comparator.lib\"",
        ),
        (
            "memristor",
            "V_VIN IN 0 SINE(0 3 100000000)\nV_VSENSE IN TOP 0\nXXM TOP 0 memristor\n.include \"models/memristor.lib\"",
        ),
    ] {
        let (source, resources) = fixture(name);
        let report = compile_source_with_resources(&source, CompileOptions::default(), &resources);
        assert!(!report.has_errors(), "{:?}", report.diagnostics);
        let circuit = report.ir.as_ref().unwrap();
        let context = NativeSimulationContext {
            resources,
            ..Default::default()
        };
        let generated = report.spice_netlist.as_ref().unwrap();
        let controls = generated.split_once("\n.control\n").unwrap().1;
        let reference = format!("Independent {name} topology\n{topology}\n.control\n{controls}");
        let execute = |deck: &str| {
            runner
                .run_with_context(
                    &SimulationRequest::new(deck, circuit.analyses.clone()),
                    &context,
                    &CancellationToken::new(),
                )
                .unwrap()
        };
        let actual = execute(generated);
        let expected = execute(&reference);
        assert!(actual.errors.is_empty(), "{}: {:?}", name, actual.errors);
        assert!(
            expected.errors.is_empty(),
            "{}: {:?}",
            name,
            expected.errors
        );
        let actual_assertions = evaluate_assertions(circuit, &actual);
        let expected_assertions = evaluate_assertions(circuit, &expected);
        assert!(
            actual_assertions.all_passed(),
            "{name}: {actual_assertions:?}"
        );
        assert_eq!(
            serde_json::to_value(actual_assertions).unwrap(),
            serde_json::to_value(expected_assertions).unwrap()
        );
    }
}

#[test]
fn model_identity_pin_count_entry_modes_and_dependencies_fail_closed() {
    let (source, resources) = fixture("comparator");
    for (bad, code) in [
        (
            source.replace("entry=KESSETSU_COMPARATOR_V1", "entry=Missing"),
            "KES-C017",
        ),
        (
            source.replace("(in_p,in_n,vcc,vee,out)", "(in_n,in_p,vcc,vee,out)"),
            "KES-C012",
        ),
        (source.replace("sha256=00198", "sha256=10198"), "KES-C016"),
        (source.replace("device U1", "opamp U1"), "KES-C004"),
    ] {
        let report = compile_source_with_resources(&bad, CompileOptions::all_outputs(), &resources);
        assert!(
            report.diagnostics.iter().any(|d| d.code == code),
            "{:?}",
            report.diagnostics
        );
        assert!(report.spice_netlist.is_none());
    }
    let report = compile_source_with_resources(
        &source.replace("simulator=ngspice", "simulator=ngspice_ps"),
        CompileOptions::default(),
        &resources,
    );
    let circuit = report.ir.as_ref().unwrap();
    assert!(
        browser_analysis_with_resources(
            circuit,
            &NetlistGraph::build(circuit),
            &circuit.analyses[0],
            &resources
        )
        .unwrap_err()
        .contains("native-only")
    );
    assert!(
        validate_browser_library(".subckt root p n\nX1 p n missing\n.ends root")
            .unwrap_err()
            .contains("dependency")
    );
    for directive in [
        "X1 p n root",
        ".control",
        ".include evil.lib",
        ".lib evil.lib",
        ".tran 1n 1m",
        ".model x memristor(1)",
        "A1 p n compiled",
        "R1 p n file=data",
        ".end",
        ".save all",
        "stop",
    ] {
        assert!(
            validate_browser_library(&format!(".subckt root p n\n{directive}\n.ends root"))
                .is_err(),
            "{directive}"
        );
    }
    assert!(
        validate_resource_bindings(&[("../escape".into(), vec![])].into_iter().collect()).is_err()
    );
}

#[test]
fn native_cli_selects_two_new_interface_families_with_fixed_requirements() {
    for name in ["comparator", "memristor"] {
        let (source, resources) = fixture(name);
        let workspace = TestWorkspace::new(name);
        for (resource, bytes) in resources {
            workspace.write(resource, std::str::from_utf8(&bytes).unwrap());
        }
        let circuit = workspace.write("circuit.kess", &source);
        let result = workspace.run_cli(&["test", circuit.to_str().unwrap(), "--format", "json"]);
        assert_eq!(
            result.status.code(),
            Some(0),
            "{name}: {} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn uic_and_continued_default_parameters_preserve_legacy_analyses() {
    let (source, resources) = fixture("memristor");
    let report = compile_source_with_resources(&source, CompileOptions::default(), &resources);
    assert!(
        report
            .spice_netlist
            .unwrap()
            .contains("tran 1e-10 1e-8 uic")
    );
    let legacy = compile_source_with_resources(
        &source.replace(" uic\n", "\n"),
        CompileOptions::default(),
        &resources,
    );
    assert!(
        !serde_json::to_string(&legacy.ir.unwrap().analyses)
            .unwrap()
            .contains("use_initial_conditions")
    );
    let invalid = compile_source_with_resources(
        &source.replace(" uic\n", " arbitrary\n"),
        CompileOptions::default(),
        &resources,
    );
    assert!(invalid.diagnostics.iter().any(|d| d.code == "KES-C009"));
    let mut continued = resources.clone();
    let bytes = String::from_utf8(continued["models/memristor.lib"].clone())
        .unwrap()
        .replace("PARAMS: stime", "\n+ PARAMS: stime")
        .into_bytes();
    let changed_source = source.replace(
        "cd4bac38581fb00c1b5667e9d5ec440cf954efb5e26105ae2775853cc90301f6",
        &format!("{:x}", Sha256::digest(&bytes)),
    );
    continued.insert("models/memristor.lib".into(), bytes);
    assert!(
        !compile_source_with_resources(&changed_source, CompileOptions::default(), &continued)
            .has_errors()
    );
}
