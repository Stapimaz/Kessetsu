mod common;

use common::TestWorkspace;
use netlang_core::compiler::{CompileOptions, compile_source};
use netlang_core::graph::{NetlistGraph, generate_spice};
use netlang_core::ir::{ComponentKind, FETPolarity, ModelSource, ast_to_ir};
use netlang_core::models::{MODEL_LOCK_SCHEMA_VERSION, lockfile_json};
use netlang_core::parse_program;
use netlang_core::simulation::{
    CancellationToken, NgspiceRunner, SimulationRequest, SimulationRunner,
};

fn user_model_source() -> &'static str {
    "model diode SafeD version=1.2.0 license=MIT source=user Is=2e-9 Rs=0.5\n\
     model mosfet SafeP pmos version=2.0.0 license=Apache-2.0 Vto=-2 Kp=4\n\
     subcircuit opamp SafeOp (in_p,in_n,vcc,vee,out) version=1.0.0 license=MIT gain=100k bandwidth=2MHz\n\
     diode D1 SafeD\n\
     mosfet M1 SafeP\n\
     opamp U1 SafeOp\n"
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
            "NL-C012",
        ),
        ("model bjt Bad version=1 license=MIT Is=1e-12\n", "NL-C010"),
        (
            "model mosfet Bad pmos version=1 license=MIT unknown=2\n",
            "NL-C010",
        ),
        (
            "model diode IRF540 version=1 license=MIT Is=1e-9\n",
            "NL-C013",
        ),
        ("model_include netlang_analog 9.9.9\n", "NL-C011"),
    ] {
        let report = compile_source(source, CompileOptions::default());
        assert_eq!(report.diagnostics[0].code, code, "source: {source}");
        assert!(report.spice_netlist.is_none());
    }

    let mismatch =
        "model mosfet SafeP pmos version=1 license=MIT Vto=-2\ntransistor Q1 npn SafeP\n";
    let report = compile_source(mismatch, CompileOptions::default());
    assert_eq!(report.diagnostics[0].code, "NL-C004");
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
    let source = "model_include netlang_analog 1.0.0\nnet GND\nnet VDD\nnet OUT\nsource VS 5V\nopamp U1 NLANG_PACKAGE_OPAMP\nresistor R1 10k\nconnect VS.minus to GND\nconnect VS.plus to VDD\nconnect U1.in_p to GND\nconnect U1.in_n to OUT\nconnect U1.vcc to VDD\nconnect U1.vee to GND\nconnect U1.out to OUT\nconnect R1.p1 to OUT\nconnect R1.p2 to GND\n";
    let first = compile_source(source, CompileOptions::default());
    let second = compile_source(source, CompileOptions::default());
    assert!(!first.has_errors());
    assert_eq!(first.model_lock, second.model_lock);
    let lock = first
        .model_lock
        .expect("model use should produce a lockfile");
    let json: serde_json::Value = serde_json::from_str(&lock).expect("lock should be JSON");
    assert_eq!(json["schema_version"], MODEL_LOCK_SCHEMA_VERSION);
    assert_eq!(json["packages"][0]["name"], "netlang_analog");
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
fn cli_materializes_netlang_lock_and_reports_it_as_an_artifact() {
    let source = "net GND\nnet VDD\nnet OUT\nsource VS 5V\nopamp U1\nresistor R1 10k\nconnect VS.minus to GND\nconnect VS.plus to VDD\nconnect U1.in_p to GND\nconnect U1.in_n to OUT\nconnect U1.vcc to VDD\nconnect U1.vee to GND\nconnect U1.out to OUT\nconnect R1.p1 to OUT\nconnect R1.p2 to GND\n";
    let workspace = TestWorkspace::new("model-lock");
    let source_path = workspace.write("circuit.nl", source);
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
    assert!(workspace.path().join("netlang.lock").is_file());
    assert!(
        value["artifacts"]
            .as_array()
            .expect("artifacts should be an array")
            .iter()
            .any(|artifact| artifact["kind"] == "model_lock")
    );
    assert_eq!(
        value["debug"]["models"]["manifest"]["models"][0]["name"],
        "NLANG_OPAMP_V1"
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
source VS 12V\nresistor RB 10k\nresistor RQ 100\ntransistor Q1 npn NLANG_POWER_NPN_V1\n\
mosfet M1 NLANG_PMOS_V1\nresistor RM 100\nopamp U1\nresistor RO 10k\n\
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
    let request = SimulationRequest::new(
        report.spice_netlist.expect("SPICE should exist"),
        circuit.analyses,
    );
    let result = NgspiceRunner::discover()
        .run(&request, &CancellationToken::new())
        .expect("Ngspice should launch");
    assert!(result.succeeded(), "simulation errors: {:?}", result.errors);
    assert_eq!(result.datasets.len(), 1);
}
