mod common;

use common::{TestWorkspace, read_fixture};
use kessetsu_core::compiler::{CompileOptions, compile_source};
use kessetsu_core::sim_result::evaluate_assertions;
use kessetsu_core::simulation::{
    CancellationToken, NgspiceRunner, SimulationRequest, SimulationRunner,
};
use serde_json::Value;

fn run_benchmark(relative: &str) {
    let source = read_fixture(relative);
    let compiled = compile_source(&source, CompileOptions::default());
    assert!(
        !compiled.has_errors(),
        "{relative} compile diagnostics: {:?}",
        compiled.diagnostics
    );
    let circuit = compiled.ir.expect("benchmark IR should exist");
    let request = SimulationRequest::new(
        compiled
            .spice_netlist
            .expect("benchmark SPICE should exist"),
        circuit.analyses.clone(),
    );
    let result = NgspiceRunner::discover()
        .run(&request, &CancellationToken::new())
        .expect("bundled/system Ngspice should launch");
    assert!(result.succeeded(), "{relative} errors: {:?}", result.errors);
    let assertions = evaluate_assertions(&circuit, &result);
    assert!(
        assertions.all_passed(),
        "{relative} assertion report: {assertions:#?}"
    );
}

fn json(output: &std::process::Output) -> Value {
    assert!(
        output.stderr.is_empty(),
        "JSON stderr must stay empty: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("CLI stdout should be versioned JSON")
}

#[test]
fn canonical_rc_gain_stage_and_power_amplifier_meet_real_ngspice_targets() {
    for fixture in [
        "benchmarks/rc_filter.kess",
        "benchmarks/gain_stage.kess",
        "benchmarks/power_amplifier.kess",
        "benchmarks/ac_coupled_amplifier.kess",
        "parameters/rc_instances.kess",
    ] {
        run_benchmark(fixture);
    }
}

#[test]
fn external_agent_revises_a_real_rc_design_from_structured_feedback() {
    let workspace = TestWorkspace::new("real-rc-agent-revision");
    let discovered = NgspiceRunner::discover();
    let executable = discovered.executable();
    assert!(
        executable.is_absolute() && executable.is_file(),
        "Ngspice discovery should resolve PATH/bundled executables to an existing absolute path: {}",
        executable.display()
    );
    let target = read_fixture("benchmarks/rc_filter.kess");
    let initial = target.replace("159.154943nF", "15.9154943nF");

    let failing = workspace.run_cli_with_stdin_and_env(
        &["test", "-", "--format", "json"],
        &initial,
        "KESSETSU_NGSPICE",
        executable,
    );
    assert_eq!(failing.status.code(), Some(4));
    let failing = json(&failing);
    assert_eq!(failing["schema_version"], "kessetsu.cli.v1");
    assert_eq!(
        failing["domain_versions"]["assertion"],
        "kessetsu.assertion.v1"
    );
    assert_eq!(
        failing["domain_versions"]["measurement"],
        "kessetsu.measurement.v2"
    );
    let cutoff_failure = failing["assertions"]["assertions"]
        .as_array()
        .expect("assertions should be an array")
        .iter()
        .find(|assertion| assertion["metric"] == "cutoff" && assertion["status"] == "FAIL")
        .expect("wrong capacitor should fail a typed cutoff assertion");
    assert!(
        cutoff_failure["actual"]
            .as_f64()
            .is_some_and(|value| value > 9_000.0)
    );

    let revised = workspace.run_cli_with_stdin_and_env(
        &["test", "-", "--format", "json"],
        &target,
        "KESSETSU_NGSPICE",
        executable,
    );
    assert_eq!(revised.status.code(), Some(0));
    let revised = json(&revised);
    assert_eq!(revised["status"], "success");
    assert_eq!(revised["summary"]["assertions"]["passed"], 5);
    assert!(
        revised["assertions"]["assertions"]
            .as_array()
            .expect("assertions should be an array")
            .iter()
            .all(|assertion| assertion["status"] == "PASS")
    );
}
