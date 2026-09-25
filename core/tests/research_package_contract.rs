mod common;

use common::TestWorkspace;
use kessetsu_core::experiment::{
    ExperimentResults, ExperimentSpec, compile_case, evaluate_case, hash_bytes, new_results,
    plan_experiment, summarize, temperature_netlist,
};
use kessetsu_core::models::ExternalModelResources;
use kessetsu_core::research_package::{RESEARCH_PACKAGE_SCHEMA, build_research_package};
use kessetsu_core::simulation::{
    CancellationToken, NativeSimulationContext, NgspiceRunner, SimulationRequest,
};
use std::fs;

const MODEL: &str = ".subckt LOAD p n\nR1 p n 1k\n.ends LOAD\n";

fn fixture(policy: &str) -> (ExperimentSpec, ExternalModelResources) {
    let digest = hash_bytes(MODEL.as_bytes());
    let source = format!(
        "external_subcircuit two_terminal LOAD (p1,p2) file=\"models/load.lib\" entry=LOAD sha256={} version=1.0.0 license=MIT source=\"synthetic package fixture\" simulator=ngspice redistribution={policy}\n\
         net GND\nnet TOP\nsource VIN sine(0V,1V,1kHz)\ndevice X1 LOAD\n\
         connect VIN.minus, X1.p2 to GND\nconnect VIN.plus, X1.p1 to TOP\nsimulate tran 10us 1ms\n",
        digest.strip_prefix("sha256:").unwrap()
    );
    (
        ExperimentSpec {
            schema_version: kessetsu_core::experiment::EXPERIMENT_SCHEMA.into(),
            name: format!("{policy} research package"),
            source,
            requirements: None,
            axes: Vec::new(),
            revisions: Vec::new(),
            tolerances: None,
            temperatures_c: vec![27.0],
            timeout_ms: 30_000,
            measurements: Vec::new(),
            objective: None,
        },
        ExternalModelResources::from([("models/load.lib".into(), MODEL.as_bytes().to_vec())]),
    )
}

fn run(spec: &ExperimentSpec, resources: &ExternalModelResources) -> ExperimentResults {
    let plan = plan_experiment(spec.clone(), resources).unwrap();
    let runner = NgspiceRunner::discover();
    let mut results = new_results(plan, runner.info().unwrap(), "package-test".into());
    let case = &results.plan.cases[0];
    let report = compile_case(&results.plan, case, resources).unwrap();
    let circuit = report.ir.as_ref().unwrap();
    let simulation = runner
        .run_with_context(
            &SimulationRequest::new(
                temperature_netlist(report.spice_netlist.as_deref().unwrap(), case.temperature_c)
                    .unwrap(),
                circuit.analyses.clone(),
            ),
            &NativeSimulationContext {
                resources: resources.clone(),
                ..Default::default()
            },
            &CancellationToken::new(),
        )
        .unwrap();
    results.cases[0] = evaluate_case(&results.plan, case, &report, simulation);
    summarize(&mut results);
    results
}

#[test]
fn package_is_deterministic_and_never_embeds_prohibited_models() {
    for policy in ["permitted", "prohibited"] {
        let (spec, resources) = fixture(policy);
        let results = run(&spec, &resources);
        let first = build_research_package(&results, &resources, 0, "V(TOP)", &[]).unwrap();
        let second = build_research_package(&results, &resources, 0, "V(TOP)", &[]).unwrap();
        assert_eq!(first.manifest, second.manifest);
        assert_eq!(first.files, second.files);
        assert_eq!(first.manifest.schema_version, RESEARCH_PACKAGE_SCHEMA);
        assert_eq!(first.manifest.files.len(), first.files.len());
        for entry in &first.manifest.files {
            let bytes = &first.files[&entry.path];
            assert_eq!(entry.sha256, hash_bytes(bytes));
            assert_eq!(entry.bytes, bytes.len());
        }
        assert_eq!(first.manifest.dependencies.len(), 1);
        assert_eq!(
            first.manifest.dependencies[0].bundled,
            policy == "permitted"
        );
        assert_eq!(
            first.files.contains_key("models/load.lib"),
            policy == "permitted"
        );
        assert!(first.files.contains_key("results.json"));
        assert!(first.files.contains_key("datasets.csv"));
        assert!(first.files.contains_key("plot.svg"));
        assert!(first.files.contains_key("report.html"));
    }

    let (mut colliding, _) = fixture("permitted");
    colliding.source = colliding.source.replace("models/load.lib", "results.json");
    let resources =
        ExternalModelResources::from([("results.json".into(), MODEL.as_bytes().to_vec())]);
    let results = run(&colliding, &resources);
    assert!(
        build_research_package(&results, &resources, 0, "V(TOP)", &[])
            .unwrap_err()
            .contains("collides with a reserved package file")
    );
}

#[test]
fn cli_writes_a_complete_new_directory_and_refuses_overwrite() {
    let workspace = TestWorkspace::new("research-package-cli");
    let (spec, _) = fixture("permitted");
    workspace.write(
        "study.kessstudy.json",
        &serde_json::to_string_pretty(&spec).unwrap(),
    );
    workspace.write("models/load.lib", MODEL);
    let run = workspace.run_cli(&[
        "study",
        "run",
        "study.kessstudy.json",
        "--output",
        "results.json",
    ]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let package = workspace.run_cli(&[
        "study",
        "package",
        "study.kessstudy.json",
        "--results",
        "results.json",
        "--output",
        "portable",
        "--signal",
        "V(TOP)",
    ]);
    assert!(
        package.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&package.stdout),
        String::from_utf8_lossy(&package.stderr)
    );
    for relative in [
        "manifest.json",
        "README.txt",
        "study.kessstudy.json",
        "circuit.kess",
        "results.json",
        "summary.csv",
        "datasets.csv",
        "plot.svg",
        "report.html",
        "models/load.lib",
    ] {
        assert!(
            workspace.path().join("portable").join(relative).is_file(),
            "{relative}"
        );
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(workspace.path().join("portable/manifest.json")).unwrap())
            .unwrap();
    assert_eq!(manifest["schema_version"], RESEARCH_PACKAGE_SCHEMA);
    let rerun = workspace.run_cli(&[
        "study",
        "run",
        "portable/study.kessstudy.json",
        "--output",
        "portable/rerun-results.json",
    ]);
    assert!(
        rerun.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&rerun.stdout),
        String::from_utf8_lossy(&rerun.stderr)
    );
    let original: serde_json::Value =
        serde_json::from_slice(&fs::read(workspace.path().join("results.json")).unwrap()).unwrap();
    let reproduced: serde_json::Value = serde_json::from_slice(
        &fs::read(workspace.path().join("portable/rerun-results.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(original["plan"]["identity"], reproduced["plan"]["identity"]);
    assert_eq!(original["cases"], reproduced["cases"]);
    let retry = workspace.run_cli(&[
        "study",
        "package",
        "study.kessstudy.json",
        "--results",
        "results.json",
        "--output",
        "portable",
        "--signal",
        "V(TOP)",
    ]);
    assert_eq!(retry.status.code(), Some(2));
    assert!(workspace.path().join("portable/manifest.json").is_file());
}
