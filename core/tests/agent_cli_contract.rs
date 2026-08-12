mod common;

use common::TestWorkspace;
use serde_json::Value;

const CLI_SCHEMA_VERSION: &str = "netlang.cli.v1";

fn candidate(resistance: &str) -> String {
    format!(
        "source V1 5V\nresistor R1 {resistance}\nconnect V1.plus to R1.p1\nconnect V1.minus to R1.p2\nsimulate op\nassert peak(I(V1)) < 100mA\n"
    )
}

fn json(output: &std::process::Output) -> Value {
    assert!(
        output.stderr.is_empty(),
        "stderr must stay empty in JSON mode"
    );
    serde_json::from_slice(&output.stdout).expect("CLI stdout should contain one JSON object")
}

#[test]
fn stdin_source_is_in_memory_by_default_and_supports_explicit_output() {
    let workspace = TestWorkspace::new("stdin-output");
    let source = candidate("100");

    let in_memory = workspace.run_cli_with_stdin(
        &["compile", "-", "--format", "json", "--include", "spice"],
        &source,
    );
    assert_eq!(in_memory.status.code(), Some(0));
    let in_memory_json = json(&in_memory);
    assert_eq!(in_memory_json["schema_version"], CLI_SCHEMA_VERSION);
    assert_eq!(in_memory_json["command"], "compile");
    assert_eq!(in_memory_json["artifacts"], serde_json::json!([]));
    assert!(
        in_memory_json["debug"]["spice_netlist"]
            .as_str()
            .is_some_and(|netlist| netlist.contains("R_R1") && netlist.contains(" 100\n"))
    );
    assert_eq!(
        std::fs::read_dir(workspace.path())
            .expect("workspace should be readable")
            .count(),
        0,
        "stdin compile must not invent an output filename"
    );

    let output_path = workspace.path().join("explicit.spice");
    let output_arg = output_path.to_string_lossy().into_owned();
    let explicit = workspace.run_cli_with_stdin(
        &["compile", "-", "--format", "json", "--output", &output_arg],
        &source,
    );
    assert_eq!(explicit.status.code(), Some(0));
    assert!(output_path.is_file());
    assert_eq!(json(&explicit)["artifacts"][0]["kind"], "spice_netlist");
}

#[test]
fn versioned_stdin_requests_are_byte_stable_for_agent_retries() {
    let workspace = TestWorkspace::new("stdin-determinism");
    let source = candidate("100");
    let simulator = workspace.write_agent_loop_simulator("deterministic-simulator");

    for arguments in [
        vec!["check", "-", "--format", "json"],
        vec!["compile", "-", "--format", "json", "--include", "spice"],
    ] {
        let first = workspace.run_cli_with_stdin(&arguments, &source);
        let second = workspace.run_cli_with_stdin(&arguments, &source);
        assert_eq!(first.status.code(), Some(0));
        assert_eq!(second.status.code(), Some(0));
        assert_eq!(first.stdout, second.stdout, "request was not byte-stable");
    }

    for command in ["simulate", "test"] {
        let arguments = [command, "-", "--format", "json"];
        let first = workspace.run_cli_with_stdin_and_env(
            &arguments,
            &source,
            "NETLANG_NGSPICE",
            &simulator,
        );
        let second = workspace.run_cli_with_stdin_and_env(
            &arguments,
            &source,
            "NETLANG_NGSPICE",
            &simulator,
        );
        assert_eq!(first.status.code(), Some(0));
        assert_eq!(second.status.code(), Some(0));
        assert_eq!(first.stdout, second.stdout, "request was not byte-stable");
    }
}

#[test]
fn external_agent_loop_compiles_measures_and_revises_without_parsing_human_text() {
    let workspace = TestWorkspace::new("agent-revision-loop");
    let simulator = workspace.write_agent_loop_simulator("agent-loop-simulator");
    let initial = candidate("10");

    let compile = workspace.run_cli_with_stdin(
        &["compile", "-", "--format", "json", "--include", "spice"],
        &initial,
    );
    assert_eq!(compile.status.code(), Some(0));
    let compile_json = json(&compile);
    assert_eq!(compile_json["status"], "success");
    assert_eq!(
        compile_json["domain_versions"]["compile"],
        "netlang.compile.v3"
    );

    let simulate = workspace.run_cli_with_stdin_and_env(
        &["simulate", "-", "--format", "json"],
        &initial,
        "NETLANG_NGSPICE",
        &simulator,
    );
    assert_eq!(simulate.status.code(), Some(0));
    let simulate_json = json(&simulate);
    assert_eq!(simulate_json["status"], "success");
    assert_eq!(simulate_json["measurements"]["observed_current"], 0.2);
    assert_eq!(
        simulate_json["domain_versions"]["simulation"],
        "netlang.simulation.v1"
    );

    let failing_test = workspace.run_cli_with_stdin_and_env(
        &["test", "-", "--format", "json"],
        &initial,
        "NETLANG_NGSPICE",
        &simulator,
    );
    assert_eq!(failing_test.status.code(), Some(4));
    let failing_json = json(&failing_test);
    assert_eq!(failing_json["status"], "test_failed");
    assert_eq!(
        failing_json["assertions"]["assertions"][0]["status"],
        "FAIL"
    );
    assert_eq!(failing_json["assertions"]["assertions"][0]["actual"], 0.2);

    let revised = initial.replace("resistor R1 10", "resistor R1 100");
    let passing_test = workspace.run_cli_with_stdin_and_env(
        &["test", "-", "--format", "json"],
        &revised,
        "NETLANG_NGSPICE",
        &simulator,
    );
    assert_eq!(passing_test.status.code(), Some(0));
    let passing_json = json(&passing_test);
    assert_eq!(passing_json["status"], "success");
    assert_eq!(
        passing_json["assertions"]["assertions"][0]["status"],
        "PASS"
    );
    assert_eq!(passing_json["assertions"]["assertions"][0]["actual"], 0.02);
    assert_eq!(passing_json["summary"]["assertions"]["passed"], 1);
}
