mod common;

use common::TestWorkspace;
use kessetsu_core::compiler::{CompileOptions, compile_source, compile_source_with_resources};
use kessetsu_core::exporter::{ExportFormat, ExportOptions, export_report};
use kessetsu_core::graph::{NetlistGraph, generate_spice};
use kessetsu_core::ir::{ComponentKind, FETPolarity, ModelSource, ast_to_ir};
use kessetsu_core::models::{MODEL_LOCK_SCHEMA_VERSION, lockfile_json};
use kessetsu_core::parse_program;
use kessetsu_core::simulation::{
    CancellationToken, NgspiceRunner, SimulationRequest, SimulationRunner,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn user_model_source() -> &'static str {
    "model diode SafeD version=1.2.0 license=MIT source=user Is=2e-9 Rs=0.5\n\
     model mosfet SafeP pmos version=2.0.0 license=Apache-2.0 Vto=-2 Kp=4\n\
     subcircuit opamp SafeOp (in_p,in_n,vcc,vee,out) version=1.0.0 license=MIT gain=100k bandwidth=2MHz\n\
     diode D1 SafeD\n\
     mosfet M1 SafeP\n\
     opamp U1 SafeOp\n"
}

fn external_model_fixture() -> Vec<u8> {
    b"* synthetic contract fixture; never manufacturer content\n.SUBCKT OPA197 IN+ IN- VCC VEE OUT\nE1 OUT 0 IN+ IN- 100000\n.ENDS OPA197\n"
        .to_vec()
}

fn external_model_source(hash: &str, file: &str, alias: &str) -> String {
    format!(
        "external_subcircuit opamp {alias} (in_p,in_n,vcc,vee,out) file=\"{file}\" entry=OPA197 sha256={hash} version=\"Final 1.3\" license=\"TI terms\" source=\"https://www.ti.com/model\" simulator=ngspice_ps redistribution=prohibited\n\
         net GND\nnet VCC\nnet VEE\nnet OUT\nsource VP 6V\nsource VN 6V\nopamp U1 {alias}\nresistor RL 10k\n\
         connect VP.minus to GND\nconnect VP.plus to VCC\nconnect VN.plus to GND\nconnect VN.minus to VEE\n\
         connect U1.in_p to GND\nconnect U1.in_n to OUT\nconnect U1.vcc to VCC\nconnect U1.vee to VEE\nconnect U1.out to OUT\n\
         connect RL.p1 to OUT\nconnect RL.p2 to GND\nsimulate op\n"
    )
}

fn parameterized_external_source(hash: &str, declaration: &str, instance: &str) -> String {
    format!(
        "external_subcircuit two_terminal TUNABLE (p1,p2) file=\"models/tunable.lib\" entry=TUNABLE sha256={hash} version=1.0.0 license=MIT source=\"synthetic contract fixture\" simulator=ngspice redistribution=permitted {declaration}\n\
         param target: Ohm = 2k\nnet GND\nnet OUT\nsource VIN 1V\ndevice X1 TUNABLE {instance}\n\
         connect VIN.minus, X1.p2 to GND\nconnect VIN.plus, X1.p1 to OUT\nsimulate op\n"
    )
}

#[test]
fn external_instance_parameters_are_typed_resolved_and_preserved_by_exports() {
    let bytes = b".SUBCKT TUNABLE p n PARAMS: Rvalue=1k Vlimit=5\nR1 p n {Rvalue}\n.ENDS TUNABLE\n"
        .to_vec();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let source = parameterized_external_source(
        &hash,
        "instance_parameters=\"Rvalue:Ohm,Vlimit:V\"",
        "(rvalue={target}, Vlimit=3V)",
    );
    let resources = BTreeMap::from([("models/tunable.lib".to_string(), bytes)]);

    let report = compile_source_with_resources(&source, CompileOptions::all_outputs(), &resources);
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    let circuit = report.ir.as_ref().expect("IR should exist");
    let component = circuit
        .components
        .iter()
        .find(|component| component.id == "X1")
        .expect("external device should exist");
    assert_eq!(component.instance_parameters["Rvalue"].value, 2_000.0);
    assert_eq!(component.instance_parameters["Vlimit"].value, 3.0);
    assert!(circuit.parameter_manifest.bindings.iter().any(|binding| {
        binding.component == "X1"
            && binding.field == "instance_parameter.Rvalue"
            && binding.dependencies == vec!["target"]
    }));
    let spice = report.spice_netlist.as_deref().expect("SPICE should exist");
    assert!(spice.contains("TUNABLE params: Rvalue=2000 Vlimit=3"));
    let kicad = report.kicad_sch.as_deref().expect("KiCad should exist");
    assert!(kicad.contains("Kessetsu_Instance_Parameters"));
    assert!(kicad.contains("Rvalue=2000; Vlimit=3"));
    let ltspice = export_report(&report, ExportFormat::Ltspice, ExportOptions::default())
        .expect("LTspice export should exist");
    let ltspice = std::str::from_utf8(&ltspice.bytes).expect("LTspice should be text");
    assert!(ltspice.contains("SYMATTR SpiceLine Rvalue=2000 Vlimit=3"));
}

#[test]
fn external_instance_parameters_fail_closed_on_unknown_names_units_and_library_contracts() {
    let bytes = b".SUBCKT TUNABLE p n PARAMS: Rvalue=1k\nR1 p n {Rvalue}\n.ENDS TUNABLE\n".to_vec();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let resources = BTreeMap::from([("models/tunable.lib".to_string(), bytes)]);

    let unknown =
        parameterized_external_source(&hash, "instance_parameters=\"Rvalue:Ohm\"", "(Missing=2k)");
    let report = compile_source_with_resources(&unknown, CompileOptions::default(), &resources);
    assert_eq!(report.diagnostics[0].code, "KES-C022");

    let wrong_unit =
        parameterized_external_source(&hash, "instance_parameters=\"Rvalue:Ohm\"", "(Rvalue=2V)");
    let report = compile_source_with_resources(&wrong_unit, CompileOptions::default(), &resources);
    assert_eq!(report.diagnostics[0].code, "KES-C022");

    let absent_from_library =
        parameterized_external_source(&hash, "instance_parameters=\"Other:Ohm\"", "(Other=2k)");
    let report =
        compile_source_with_resources(&absent_from_library, CompileOptions::default(), &resources);
    assert_eq!(report.diagnostics[0].code, "KES-C014");
}

#[test]
fn external_subcircuit_is_hash_bound_without_serializing_its_body() {
    let bytes = external_model_fixture();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let source = external_model_source(&hash, "models/OPA197.LIB", "OPA_ALIAS");
    let mut resources = BTreeMap::new();
    resources.insert("models/OPA197.LIB".to_string(), bytes.clone());

    let report = compile_source_with_resources(&source, CompileOptions::all_outputs(), &resources);
    assert!(
        !report.has_errors(),
        "diagnostics: {:?}",
        report.diagnostics
    );
    let spice = report.spice_netlist.as_deref().expect("SPICE should exist");
    assert!(spice.contains("XU1 0 OUT VCC VEE OUT OPA197"));
    assert!(spice.contains(".include \"models/OPA197.LIB\""));
    assert!(!spice.contains("synthetic contract fixture"));

    let lock = report.model_lock.as_deref().expect("lock should exist");
    assert!(lock.contains("\"source\": \"External\""));
    assert!(lock.contains("\"resource\": \"models/OPA197.LIB\""));
    assert!(lock.contains("\"simulator\": \"ngspice_ps\""));
    assert!(lock.contains(&format!("sha256:{hash}")));
    assert!(!lock.contains("synthetic contract fixture"));
    let serialized = serde_json::to_string(&report).expect("report should serialize");
    assert!(!serialized.contains("synthetic contract fixture"));
    assert!(!serialized.contains("E1 OUT"));
    let schematic_component = report
        .schematic
        .as_ref()
        .expect("schematic should exist")
        .components
        .iter()
        .find(|component| component.id == "U1")
        .expect("external opamp should exist in schematic");
    let metadata = schematic_component
        .model_metadata
        .as_ref()
        .expect("schematic should preserve model metadata");
    assert_eq!(metadata.resource.as_deref(), Some("models/OPA197.LIB"));
    assert_eq!(metadata.content_hash, format!("sha256:{hash}"));
    let kicad = report.kicad_sch.as_deref().expect("KiCad should exist");
    assert!(kicad.contains("Kessetsu_Model_Resource"));
    assert!(kicad.contains("models/OPA197.LIB"));
    assert!(!kicad.contains("synthetic contract fixture"));
    let ltspice = export_report(&report, ExportFormat::Ltspice, ExportOptions::default())
        .expect("LTspice export should exist");
    let ltspice_text = std::str::from_utf8(&ltspice.bytes).expect("LTspice should be text");
    assert!(ltspice_text.contains(".include \"models/OPA197.LIB\""));
    assert!(ltspice_text.contains("SYMATTR Value OPA197"));
    assert!(!ltspice_text.contains("SYMATTR Value OPA_ALIAS"));
    assert!(!ltspice_text.contains("synthetic contract fixture"));
    assert!(
        ltspice
            .warnings
            .iter()
            .any(|warning| warning.contains("user-owned"))
    );
}

#[test]
fn external_subcircuit_resolution_fails_closed() {
    let bytes = external_model_fixture();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let valid = external_model_source(&hash, "models/OPA197.LIB", "OPA_ALIAS");

    let missing = compile_source(&valid, CompileOptions::default());
    assert_eq!(missing.diagnostics[0].code, "KES-C015");
    assert!(missing.spice_netlist.is_none());

    let mut changed = BTreeMap::new();
    changed.insert("models/OPA197.LIB".to_string(), b"changed".to_vec());
    let mismatch = compile_source_with_resources(&valid, CompileOptions::default(), &changed);
    assert_eq!(mismatch.diagnostics[0].code, "KES-C016");

    let traversal = external_model_source(&hash, "../OPA197.LIB", "OPA_ALIAS");
    let invalid = compile_source_with_resources(&traversal, CompileOptions::default(), &changed);
    assert_eq!(invalid.diagnostics[0].code, "KES-C014");

    let unsupported = valid.replace("simulator=ngspice_ps", "simulator=ngspice_raw_args");
    let invalid = compile_source_with_resources(&unsupported, CompileOptions::default(), &changed);
    assert_eq!(invalid.diagnostics[0].code, "KES-C014");

    let absolute = external_model_source(&hash, "C:/models/OPA197.LIB", "OPA_ALIAS");
    let invalid = compile_source_with_resources(&absolute, CompileOptions::default(), &changed);
    assert_eq!(invalid.diagnostics[0].code, "KES-C014");

    let path_injection = external_model_source(&hash, "models/OPA197.LIB$danger", "OPA_ALIAS");
    let invalid =
        compile_source_with_resources(&path_injection, CompileOptions::default(), &changed);
    assert_eq!(invalid.diagnostics[0].code, "KES-C014");

    let injected = valid.replace("license=\"TI terms\"", "license=\"TI terms\n.control\"");
    let invalid = compile_source_with_resources(&injected, CompileOptions::default(), &changed);
    assert_eq!(invalid.diagnostics[0].code, "KES-C014");

    let duplicate = valid.replace(
        "simulator=ngspice_ps",
        &format!("sha256={hash} simulator=ngspice_ps"),
    );
    let invalid = compile_source_with_resources(&duplicate, CompileOptions::default(), &changed);
    assert_eq!(invalid.diagnostics[0].code, "KES-C014");

    let mut malformed = BTreeMap::new();
    let malformed_bytes = b".SUBCKT OPA197 IN+ IN- OUT\n.ENDS OPA197\n".to_vec();
    let malformed_hash = format!("{:x}", Sha256::digest(&malformed_bytes));
    malformed.insert("models/OPA197.LIB".to_string(), malformed_bytes);
    let invalid = compile_source_with_resources(
        &external_model_source(&malformed_hash, "models/OPA197.LIB", "OPA_ALIAS"),
        CompileOptions::default(),
        &malformed,
    );
    assert_eq!(invalid.diagnostics[0].code, "KES-C012");

    let mut valid_resource = BTreeMap::new();
    valid_resource.insert("models/OPA197.LIB".to_string(), bytes);
    let collision = compile_source_with_resources(
        &external_model_source(&hash, "models/OPA197.LIB", "KESSETSU_OPAMP_V1"),
        CompileOptions::default(),
        &valid_resource,
    );
    assert_eq!(collision.diagnostics[0].code, "KES-C013");
}

#[test]
fn native_cli_resolves_and_simulates_a_source_relative_external_model() {
    let bytes = external_model_fixture();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let source = external_model_source(&hash, "models/OPA197.LIB", "OPA_ALIAS");
    let workspace = TestWorkspace::new("external-model-native");
    workspace.write(
        "models/OPA197.LIB",
        std::str::from_utf8(&bytes).expect("fixture should be UTF-8"),
    );
    let source_path = workspace.write("circuit.kess", &source);
    let source_arg = source_path.to_string_lossy().into_owned();
    let output = workspace.run_cli(&[
        "simulate",
        &source_arg,
        "--format",
        "json",
        "--include",
        "models",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("CLI output should be JSON");
    assert_eq!(value["status"], "success");
    assert_eq!(
        value["debug"]["models"]["manifest"]["models"][0]["external"]["resource"],
        "models/OPA197.LIB"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("synthetic contract fixture"));
    assert!(!stdout.contains("E1 OUT"));
}

#[test]
fn cli_keeps_relative_external_dependencies_beside_the_source() {
    let bytes = external_model_fixture();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let source = external_model_source(&hash, "models/OPA197.LIB", "OPA_ALIAS");
    let workspace = TestWorkspace::new("external-model-output-location");
    workspace.write(
        "models/OPA197.LIB",
        std::str::from_utf8(&bytes).expect("fixture should be UTF-8"),
    );
    let source_path = workspace.write("circuit.kess", &source);
    workspace.write("other/.keep", "");
    let output_path = workspace.path().join("other/circuit.spice");
    let source_arg = source_path.to_string_lossy().into_owned();
    let output_arg = output_path.to_string_lossy().into_owned();
    let output = workspace.run_cli(&[
        "compile",
        &source_arg,
        "--output",
        &output_arg,
        "--format",
        "json",
    ]);
    assert_eq!(output.status.code(), Some(2));
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("CLI output should be JSON");
    assert_eq!(value["diagnostics"][0]["code"], "KES-I007");
    assert!(!output_path.exists());

    // Moving the source and its sidecar together must preserve both export flows.
    workspace.write("relocated/circuit.kess", &source);
    workspace.write(
        "relocated/models/OPA197.LIB",
        std::str::from_utf8(&bytes).unwrap(),
    );
    for (target, extension) in [("spice", "spice"), ("ltspice", "asc")] {
        let relocated = workspace.path().join("relocated/circuit.kess");
        let destination = workspace
            .path()
            .join(format!("relocated/circuit.{extension}"));
        let rejected = workspace.path().join(format!("other/circuit.{extension}"));
        for (output_path, expected_exit) in [(&destination, 0), (&rejected, 2)] {
            let output = workspace.run_cli(&[
                "export",
                relocated.to_str().unwrap(),
                "--target",
                target,
                "--output",
                output_path.to_str().unwrap(),
                "--format",
                "json",
            ]);
            assert_eq!(output.status.code(), Some(expected_exit), "{output:?}");
        }
        let exported = std::fs::read_to_string(destination).unwrap();
        assert!(exported.contains(".include \"models/OPA197.LIB\""));
        assert!(!exported.contains("synthetic contract fixture"));
    }
}

#[test]
fn optional_official_opa197_runs_through_the_native_external_model_path() {
    let Some(model_path) = std::env::var_os("KESSETSU_U6_MODEL") else {
        return;
    };
    let bytes = std::fs::read(model_path).expect("configured OPA197 model should be readable");
    let hash = format!("{:x}", Sha256::digest(&bytes));
    assert_eq!(
        hash, "fc5b020e63346e511bd808bf41c856b0150b000bcf8a41fe00eeececb1f422a5",
        "optional integration must use the exact recorded TI OPAx197 Rev. D file"
    );
    let source = format!(
        "external_subcircuit opamp OPA197 (in_p,in_n,vcc,vee,out) file=\"models/OPAx197.LIB\" entry=OPAx197 sha256={hash} version=\"Final 1.3\" license=\"TI terms\" source=\"https://www.ti.com/lit/zip/SBOMA34\" simulator=ngspice_ps redistribution=prohibited\n\
         net GND\nnet VCC\nnet VEE\nnet IN\nnet FB\nnet OUT\nsource VP 6V\nsource VN 6V\nsource VIN sine_ac(0V,100mV,1kHz,1V)\n\
         opamp U1 OPA197\nresistor RF 40k\nresistor RG 10k\nresistor RL 10k\n\
         connect VP.minus to GND\nconnect VP.plus to VCC\nconnect VN.plus to GND\nconnect VN.minus to VEE\nconnect VIN.minus to GND\nconnect VIN.plus to IN\n\
         connect U1.in_p to IN\nconnect U1.in_n to FB\nconnect U1.vcc to VCC\nconnect U1.vee to VEE\nconnect U1.out to OUT\n\
         connect RF.p1 to OUT\nconnect RF.p2 to FB\nconnect RG.p1 to FB\nconnect RG.p2 to GND\nconnect RL.p1 to OUT\nconnect RL.p2 to GND\n\
         simulate op\nsimulate ac dec 100 10Hz 1MHz\nsimulate tran 2us 10ms\n"
    );
    let workspace = TestWorkspace::new("official-opa197-native");
    workspace.write(
        "models/OPAx197.LIB",
        std::str::from_utf8(&bytes).expect("official model should be UTF-8"),
    );
    let source_path = workspace.write("circuit.kess", &source);
    let source_arg = source_path.to_string_lossy().into_owned();
    let output = workspace.run_cli(&["simulate", &source_arg, "--format", "json"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("CLI output should be JSON");
    assert_eq!(value["status"], "success");
    assert_eq!(value["summary"]["analyses"], 3);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("Green-Williams-Lis"));
}

#[test]
fn typed_user_models_generate_only_canonical_safe_directives() {
    let program = parse_program(user_model_source()).expect("typed model syntax should parse");
    let circuit = ast_to_ir(&program).expect("typed declarations should reach IR");
    assert_eq!(circuit.model_manifest.models.len(), 3);
    assert!(circuit.model_manifest.models.iter().all(|entry| {
        entry.source == ModelSource::UserDefined
            && entry.provenance.content_hash.starts_with("sha256:")
            && entry.provenance.content_hash.len() == 71
            && entry.provenance.simulator == "ngspice-35+"
    }));
    assert!(matches!(
        circuit.components[1].kind,
        ComponentKind::MOSFET(FETPolarity::PMOS)
    ));

    let spice = generate_spice(&circuit, &NetlistGraph::build(&circuit));
    assert!(spice.contains(".model SafeD D (is=2e-9 rs=0.5)"));
    assert!(spice.contains(".model SafeP PMOS (kp=4 vto=-2)"));
    assert!(spice.contains(".subckt SafeOp in_p in_n vcc vee out"));
    assert!(!spice.contains("source=user"));
    assert!(!spice.contains("license=MIT"));
}

#[test]
fn subcircuit_pins_kind_polarity_capability_and_names_fail_closed() {
    for (source, code) in [
        (
            "subcircuit opamp Bad (out,in_p,in_n,vcc,vee) version=1 license=MIT gain=1k bandwidth=1MHz\n",
            "KES-C012",
        ),
        ("model bjt Bad version=1 license=MIT Is=1e-12\n", "KES-C010"),
        (
            "model mosfet Bad pmos version=1 license=MIT unknown=2\n",
            "KES-C010",
        ),
        (
            "model diode IRF540 version=1 license=MIT Is=1e-9\n",
            "KES-C013",
        ),
        ("model_include kessetsu_analog 9.9.9\n", "KES-C011"),
    ] {
        let report = compile_source(source, CompileOptions::default());
        assert_eq!(report.diagnostics[0].code, code, "source: {source}");
        assert!(report.spice_netlist.is_none());
    }

    let mismatch =
        "model mosfet SafeP pmos version=1 license=MIT Vto=-2\ntransistor Q1 npn SafeP\n";
    let report = compile_source(mismatch, CompileOptions::default());
    assert_eq!(report.diagnostics[0].code, "KES-C004");
}

#[test]
fn raw_directive_injection_never_crosses_the_typed_ir_boundary() {
    for payload in [
        "model diode Evil version=1 license=MIT Is=\"1e-9 .control\"\n",
        "model diode Evil version=1 license=MIT Is=\"1e-9\n.control\nshell calc\n.endc\"\n",
        "subcircuit opamp Evil (in_p,in_n,vcc,vee,out) version=1 license=MIT gain=\"1k .include secret\" bandwidth=1MHz\n",
    ] {
        let report = compile_source(payload, CompileOptions::default());
        assert!(
            report.has_errors(),
            "payload unexpectedly compiled: {payload:?}"
        );
        assert!(report.spice_netlist.is_none());
    }
}

#[test]
fn package_resolution_and_lockfile_are_exact_and_reproducible() {
    let source = "model_include kessetsu_analog 1.0.0\nnet GND\nnet VDD\nnet OUT\nsource VS 5V\nopamp U1 KESSETSU_PACKAGE_OPAMP\nresistor R1 10k\nconnect VS.minus to GND\nconnect VS.plus to VDD\nconnect U1.in_p to GND\nconnect U1.in_n to OUT\nconnect U1.vcc to VDD\nconnect U1.vee to GND\nconnect U1.out to OUT\nconnect R1.p1 to OUT\nconnect R1.p2 to GND\n";
    let first = compile_source(source, CompileOptions::default());
    let second = compile_source(source, CompileOptions::default());
    assert!(!first.has_errors());
    assert_eq!(first.model_lock, second.model_lock);
    let lock = first
        .model_lock
        .expect("model use should produce a lockfile");
    let json: serde_json::Value = serde_json::from_str(&lock).expect("lock should be JSON");
    assert_eq!(json["schema_version"], MODEL_LOCK_SCHEMA_VERSION);
    assert_eq!(json["packages"][0]["name"], "kessetsu_analog");
    assert_eq!(json["packages"][0]["version"], "1.0.0");
    assert!(
        json["packages"][0]["content_hash"]
            .as_str()
            .is_some_and(|hash| hash.starts_with("sha256:"))
    );

    let circuit = first.ir.expect("valid compile should preserve IR");
    assert_eq!(lockfile_json(&circuit.model_manifest), lock);
}

#[test]
fn cli_materializes_kessetsu_lock_and_reports_it_as_an_artifact() {
    let source = "net GND\nnet VDD\nnet OUT\nsource VS 5V\nopamp U1\nresistor R1 10k\nconnect VS.minus to GND\nconnect VS.plus to VDD\nconnect U1.in_p to GND\nconnect U1.in_n to OUT\nconnect U1.vcc to VDD\nconnect U1.vee to GND\nconnect U1.out to OUT\nconnect R1.p1 to OUT\nconnect R1.p2 to GND\n";
    let workspace = TestWorkspace::new("model-lock");
    let source_path = workspace.write("circuit.kess", source);
    let source_arg = source_path.to_string_lossy().into_owned();
    let output = workspace.run_cli(&[
        "compile",
        &source_arg,
        "--format",
        "json",
        "--include",
        "models",
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("CLI output should be JSON");
    assert!(workspace.path().join("kessetsu.lock").is_file());
    assert!(
        value["artifacts"]
            .as_array()
            .expect("artifacts should be an array")
            .iter()
            .any(|artifact| artifact["kind"] == "model_lock")
    );
    assert_eq!(
        value["debug"]["models"]["manifest"]["models"][0]["name"],
        "KESSETSU_OPAMP_V1"
    );
    assert!(
        value["debug"]["models"]["lock"]
            .as_str()
            .is_some_and(|lock| lock.contains(MODEL_LOCK_SCHEMA_VERSION))
    );
}

#[test]
fn opamp_pmos_and_power_transistor_paths_run_in_real_ngspice() {
    let source = "net GND\nnet VDD\nnet BIAS\nnet QOUT\nnet MOUT\nnet OOUT\n\
source VS 12V\nresistor RB 10k\nresistor RQ 100\ntransistor Q1 npn KESSETSU_POWER_NPN_V1\n\
mosfet M1 KESSETSU_PMOS_V1\nresistor RM 100\nopamp U1\nresistor RO 10k\n\
connect VS.minus to GND\nconnect VS.plus to VDD\nconnect RB.p1 to VDD\nconnect RB.p2 to BIAS\n\
connect Q1.b to BIAS\nconnect Q1.c to QOUT\nconnect Q1.e to GND\nconnect RQ.p1 to VDD\nconnect RQ.p2 to QOUT\n\
connect M1.s to VDD\nconnect M1.g to GND\nconnect M1.d to MOUT\nconnect RM.p1 to MOUT\nconnect RM.p2 to GND\n\
connect U1.in_p to GND\nconnect U1.in_n to OOUT\nconnect U1.vcc to VDD\nconnect U1.vee to GND\nconnect U1.out to OOUT\nconnect RO.p1 to OOUT\nconnect RO.p2 to GND\nsimulate op\n";
    let report = compile_source(source, CompileOptions::default());
    assert!(
        !report.has_errors(),
        "compile diagnostics: {:?}",
        report.diagnostics
    );
    let circuit = report.ir.expect("IR should exist");
    let pmos = circuit
        .model_manifest
        .models
        .iter()
        .find(|model| model.name == "KESSETSU_PMOS_V1")
        .expect("verified PMOS should be present in the model manifest");
    assert_eq!(pmos.provenance.version, "1.0.1");
    let netlist = report.spice_netlist.expect("SPICE should exist");
    assert!(netlist.contains(".model KESSETSU_PMOS_V1 PMOS (Level=1"));
    assert!(!netlist.contains(" Cgd=") && !netlist.contains(" Cgs="));
    let request = SimulationRequest::new(netlist, circuit.analyses);
    let result = NgspiceRunner::discover()
        .run(&request, &CancellationToken::new())
        .expect("Ngspice should launch");
    assert!(
        result.succeeded(),
        "simulator: {:?}; errors: {:?}; stdout: {}; stderr: {}",
        result.simulator,
        result.errors,
        result.raw_log.stdout,
        result.raw_log.stderr
    );
    assert_eq!(result.datasets.len(), 1);
}

#[test]
fn builtin_diodes_and_default_model_run_with_versioned_legacy_provenance() {
    let source = include_str!("fixtures/models/builtin_diodes.kess");
    let report = compile_source(source, CompileOptions::default());
    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    assert_eq!(
        report.model_lock,
        compile_source(source, CompileOptions::default()).model_lock
    );
    let circuit = report.ir.expect("valid diode circuit should reach IR");
    assert_eq!(circuit.model_manifest.models.len(), 2);
    for model in &circuit.model_manifest.models {
        assert_eq!(model.provenance.version, "1.0.1");
        assert_eq!(model.provenance.license, "legacy-provenance");
        assert_eq!(model.source, ModelSource::Builtin);
        assert!(model.provenance.content_hash.starts_with("sha256:"));
    }
    let netlist = report.spice_netlist.expect("SPICE should exist");
    for unsupported in ["mfg=", "type=silicon", "Iave=", "Vpk="] {
        assert!(!netlist.contains(unsupported));
    }
    let request = SimulationRequest::new(netlist, circuit.analyses.clone());
    let result = NgspiceRunner::discover()
        .run(&request, &CancellationToken::new())
        .expect("Ngspice should launch");
    assert!(
        result.succeeded(),
        "{:?}; {:?}",
        result.errors,
        result.raw_log
    );
    let assertions = kessetsu_core::sim_result::evaluate_assertions(&circuit, &result);
    assert_eq!(assertions.summary.total, 6);
    assert_eq!(assertions.summary.passed, 6, "{:?}", assertions);
}

#[test]
fn opamp_v1_clamps_output_to_rails_in_real_ngspice() {
    let source = "net GND\nnet VCC\nnet VEE\nnet IN\nnet FB\nnet OUT\n\
source VP 12V\nsource VN 12V\nsource VIN sine(0V, 2V, 1kHz)\n\
opamp U1 KESSETSU_OPAMP_V1\nresistor RF 90k\nresistor RG 10k\nresistor RL 10k\n\
connect VP.minus to GND\nconnect VP.plus to VCC\nconnect VN.plus to GND\nconnect VN.minus to VEE\n\
connect VIN.minus to GND\nconnect VIN.plus to IN\n\
connect U1.in_p to IN\nconnect U1.in_n to FB\nconnect U1.vcc to VCC\nconnect U1.vee to VEE\nconnect U1.out to OUT\n\
connect RF.p1 to OUT\nconnect RF.p2 to FB\nconnect RG.p1 to FB\nconnect RG.p2 to GND\nconnect RL.p1 to OUT\nconnect RL.p2 to GND\n\
simulate tran 5us 5ms\n\
assert peak(V(OUT)) < 13V\n\
assert peak(V(OUT)) > 11.5V\n\
assert clipping(V(OUT), -12V, 12V) > 20%\n";
    let report = compile_source(source, CompileOptions::default());
    assert!(
        !report.has_errors(),
        "compile diagnostics: {:?}",
        report.diagnostics
    );
    let circuit = report.ir.expect("IR should exist");
    let opamp = circuit
        .model_manifest
        .models
        .iter()
        .find(|model| model.name == "KESSETSU_OPAMP_V1")
        .expect("KESSETSU_OPAMP_V1 should be in model manifest");
    assert_eq!(opamp.provenance.version, "1.1.0");
    let netlist = report.spice_netlist.expect("SPICE netlist should exist");
    assert!(netlist.contains("DCLP_P out vcc_lim DCLAMP"));
    assert!(netlist.contains("BRAIL_P vcc_lim 0"));
    let request = SimulationRequest::new(netlist, circuit.analyses.clone());
    let result = NgspiceRunner::discover()
        .run(&request, &CancellationToken::new())
        .expect("Ngspice should launch");
    assert!(
        result.succeeded(),
        "simulation failed: {:?}; raw_log: {:?}",
        result.errors,
        result.raw_log
    );
    let assertions = kessetsu_core::sim_result::evaluate_assertions(&circuit, &result);
    assert_eq!(assertions.summary.total, 3);
    assert_eq!(
        assertions.summary.passed, 3,
        "assertions failed: {:?}",
        assertions
    );
}

#[test]
fn parameterized_opamp_subcircuit_validates_parameter_bounds() {
    let base = "subcircuit opamp Op (in_p,in_n,vcc,vee,out) version=1.0.0 license=MIT gain=100k bandwidth=1MHz";

    // Negative rail_headroom fails with KES-C012
    let report = compile_source(
        &format!("{base} rail_headroom=-0.5\nopamp U1 Op\n"),
        CompileOptions::default(),
    );
    assert_eq!(report.diagnostics[0].code, "KES-C012");
    assert_eq!(
        report.diagnostics[0].field.as_deref(),
        Some("rail_headroom")
    );
    assert!(
        report.diagnostics[0]
            .message
            .contains("opamp rail_headroom must be non-negative")
    );

    // Non-positive slew_rate fails with KES-C012
    let report = compile_source(
        &format!("{base} slew_rate=0\nopamp U1 Op\n"),
        CompileOptions::default(),
    );
    assert_eq!(report.diagnostics[0].code, "KES-C012");
    assert_eq!(report.diagnostics[0].field.as_deref(), Some("slew_rate"));
    assert!(
        report.diagnostics[0]
            .message
            .contains("opamp slew_rate must be positive")
    );

    let report = compile_source(
        &format!("{base} slew_rate=-100k\nopamp U1 Op\n"),
        CompileOptions::default(),
    );
    assert_eq!(report.diagnostics[0].code, "KES-C012");
    assert_eq!(report.diagnostics[0].field.as_deref(), Some("slew_rate"));
    assert!(
        report.diagnostics[0]
            .message
            .contains("opamp slew_rate must be positive")
    );

    // Non-positive rout fails with KES-C012
    let report = compile_source(
        &format!("{base} rout=0\nopamp U1 Op\n"),
        CompileOptions::default(),
    );
    assert_eq!(report.diagnostics[0].code, "KES-C012");
    assert_eq!(report.diagnostics[0].field.as_deref(), Some("rout"));
    assert!(
        report.diagnostics[0]
            .message
            .contains("opamp rout must be positive")
    );

    let report = compile_source(
        &format!("{base} rout=-50\nopamp U1 Op\n"),
        CompileOptions::default(),
    );
    assert_eq!(report.diagnostics[0].code, "KES-C012");
    assert_eq!(report.diagnostics[0].field.as_deref(), Some("rout"));
    assert!(
        report.diagnostics[0]
            .message
            .contains("opamp rout must be positive")
    );
}

#[test]
fn opamp_v2_and_parameterized_subcircuit_simulate_cleanly_in_real_ngspice() {
    // 1. KESSETSU_OPAMP_V2
    let v2_source = "net GND\nnet VCC\nnet VEE\nnet IN\nnet FB\nnet OUT\n\
source VP 12V\nsource VN 12V\nsource VIN sine(0V, 1V, 1kHz)\n\
opamp U1 KESSETSU_OPAMP_V2\nresistor RF 10k\nresistor RG 10k\nresistor RL 1k\n\
connect VP.minus to GND\nconnect VP.plus to VCC\nconnect VN.plus to GND\nconnect VN.minus to VEE\n\
connect VIN.minus to GND\nconnect VIN.plus to IN\n\
connect U1.in_p to IN\nconnect U1.in_n to FB\nconnect U1.vcc to VCC\nconnect U1.vee to VEE\nconnect U1.out to OUT\n\
connect RF.p1 to OUT\nconnect RF.p2 to FB\nconnect RG.p1 to FB\nconnect RG.p2 to GND\nconnect RL.p1 to OUT\nconnect RL.p2 to GND\n\
simulate tran 5us 5ms\n";

    let report = compile_source(v2_source, CompileOptions::default());
    assert!(
        !report.has_errors(),
        "compile diagnostics: {:?}",
        report.diagnostics
    );
    let circuit = report.ir.expect("IR should exist");
    let opamp = circuit
        .model_manifest
        .models
        .iter()
        .find(|model| model.name == "KESSETSU_OPAMP_V2")
        .expect("KESSETSU_OPAMP_V2 should be in model manifest");
    assert_eq!(opamp.provenance.version, "1.0.0");
    let netlist = report.spice_netlist.expect("SPICE netlist should exist");
    assert!(netlist.contains("BGAIN"));
    assert!(netlist.contains("ROUT out_int out 50"));
    assert!(netlist.contains("v(vcc)-v(vee)>2?v(vcc)-1:0.5*(v(vcc)+v(vee))"));
    let request = SimulationRequest::new(netlist, circuit.analyses.clone());
    let result = NgspiceRunner::discover()
        .run(&request, &CancellationToken::new())
        .expect("Ngspice should launch");
    assert!(
        result.succeeded(),
        "KESSETSU_OPAMP_V2 simulation failed: {:?}; raw_log: {:?}",
        result.errors,
        result.raw_log
    );

    // 2. Parameterized user subcircuit opamp
    let param_source = "subcircuit opamp PhysicalOp (in_p,in_n,vcc,vee,out) version=1.0.0 license=MIT gain=100k bandwidth=1MHz rail_headroom=1 slew_rate=500k rout=50\n\
net GND\nnet VCC\nnet VEE\nnet IN\nnet FB\nnet OUT\n\
source VP 12V\nsource VN 12V\nsource VIN sine(0V, 1V, 1kHz)\n\
opamp U1 PhysicalOp\nresistor RF 10k\nresistor RG 10k\nresistor RL 1k\n\
connect VP.minus to GND\nconnect VP.plus to VCC\nconnect VN.plus to GND\nconnect VN.minus to VEE\n\
connect VIN.minus to GND\nconnect VIN.plus to IN\n\
connect U1.in_p to IN\nconnect U1.in_n to FB\nconnect U1.vcc to VCC\nconnect U1.vee to VEE\nconnect U1.out to OUT\n\
connect RF.p1 to OUT\nconnect RF.p2 to FB\nconnect RG.p1 to FB\nconnect RG.p2 to GND\nconnect RL.p1 to OUT\nconnect RL.p2 to GND\n\
simulate tran 5us 5ms\n";

    let report = compile_source(param_source, CompileOptions::default());
    assert!(
        !report.has_errors(),
        "compile diagnostics: {:?}",
        report.diagnostics
    );
    let circuit = report.ir.expect("IR should exist");
    let netlist = report.spice_netlist.expect("SPICE netlist should exist");
    assert!(netlist.contains(".subckt PhysicalOp"));
    assert!(netlist.contains("BGAIN"));
    assert!(netlist.contains("ROUT out_int out 50"));
    let request = SimulationRequest::new(netlist, circuit.analyses.clone());
    let result = NgspiceRunner::discover()
        .run(&request, &CancellationToken::new())
        .expect("Ngspice should launch");
    assert!(
        result.succeeded(),
        "parameterized opamp simulation failed: {:?}; raw_log: {:?}",
        result.errors,
        result.raw_log
    );
}

#[test]
fn zener_breakdown_voltage_regulation_and_clipping_in_real_ngspice() {
    let op_source = "\
model diode CustomZener version=1.0.0 license=MIT is=1e-14 rs=1 bv=6.8 ibv=10m
net GND
net SUPPLY
net SUPPLY2
net OUT_1N4733A
net OUT_BZX3V3
net OUT_BZX5V1
net OUT_BZX12
net OUT_CUSTOM
source VS 12V
source VS2 24V
resistor R1 1k
resistor R2 1k
resistor R3 1k
resistor R4 1k
resistor R5 1k
diode DZ1 1N4733A
diode DZ2 BZX84C3V3
diode DZ3 BZX84C5V1
diode DZ4 BZX84C12
diode DZC CustomZener
connect VS.plus, R1.p1, R2.p1, R3.p1, R5.p1 to SUPPLY
connect VS2.plus, R4.p1 to SUPPLY2
connect VS.minus, VS2.minus to GND
connect DZ1.p1, DZ2.p1, DZ3.p1, DZ4.p1, DZC.p1 to GND
connect R1.p2, DZ1.p2 to OUT_1N4733A
connect R2.p2, DZ2.p2 to OUT_BZX3V3
connect R3.p2, DZ3.p2 to OUT_BZX5V1
connect R4.p2, DZ4.p2 to OUT_BZX12
connect R5.p2, DZC.p2 to OUT_CUSTOM
simulate op
assert value(V(OUT_1N4733A)) > 5.0V
assert value(V(OUT_1N4733A)) < 5.2V
assert value(V(OUT_BZX3V3)) > 3.2V
assert value(V(OUT_BZX3V3)) < 3.4V
assert value(V(OUT_BZX5V1)) > 5.0V
assert value(V(OUT_BZX5V1)) < 5.2V
assert value(V(OUT_BZX12)) > 11.8V
assert value(V(OUT_BZX12)) < 12.2V
assert value(V(OUT_CUSTOM)) > 6.7V
assert value(V(OUT_CUSTOM)) < 6.9V
";
    let report = compile_source(op_source, CompileOptions::default());
    assert!(
        !report.has_errors(),
        "compile diagnostics: {:?}",
        report.diagnostics
    );
    let circuit = report.ir.expect("IR should exist");
    assert_eq!(circuit.model_manifest.models.len(), 5);
    let netlist = report.spice_netlist.expect("SPICE netlist should exist");
    assert!(netlist.contains("Bv=5.1"));
    assert!(netlist.contains("Bv=3.3"));
    assert!(netlist.contains("Bv=12"));
    assert!(netlist.contains("bv=6.8"));
    let request = SimulationRequest::new(netlist, circuit.analyses.clone());
    let result = NgspiceRunner::discover()
        .run(&request, &CancellationToken::new())
        .expect("Ngspice should launch");
    assert!(
        result.succeeded(),
        "Zener .op simulation failed: {:?}; raw_log: {:?}",
        result.errors,
        result.raw_log
    );
    let assertions = kessetsu_core::sim_result::evaluate_assertions(&circuit, &result);
    assert_eq!(assertions.summary.total, 10);
    assert_eq!(
        assertions.summary.passed, assertions.summary.total,
        "{:?}",
        assertions
    );

    // 2. Transient bidirectional clipping (+5.1V in breakdown, -0.7V in forward conduction)
    let tran_source = "\
net GND
net IN
net OUT
source VIN sine(0V, 10V, 1kHz)
resistor R1 1k
diode DZ 1N4733A
connect VIN.plus, R1.p1 to IN
connect R1.p2, DZ.p2 to OUT
connect VIN.minus, DZ.p1 to GND
simulate tran 5us 5ms
assert max(V(OUT)) > 5.0V
assert max(V(OUT)) < 5.2V
assert min(V(OUT)) > -0.8V
assert min(V(OUT)) < -0.6V
";
    let report = compile_source(tran_source, CompileOptions::default());
    assert!(
        !report.has_errors(),
        "compile diagnostics: {:?}",
        report.diagnostics
    );
    let circuit = report.ir.expect("IR should exist");
    let netlist = report.spice_netlist.expect("SPICE netlist should exist");
    let request = SimulationRequest::new(netlist, circuit.analyses.clone());
    let result = NgspiceRunner::discover()
        .run(&request, &CancellationToken::new())
        .expect("Ngspice should launch");
    assert!(
        result.succeeded(),
        "Zener .tran simulation failed: {:?}; raw_log: {:?}",
        result.errors,
        result.raw_log
    );
    let assertions = kessetsu_core::sim_result::evaluate_assertions(&circuit, &result);
    assert_eq!(assertions.summary.total, 4);
    assert_eq!(
        assertions.summary.passed, assertions.summary.total,
        "{:?}",
        assertions
    );
}
