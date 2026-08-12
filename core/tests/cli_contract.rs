mod common;

use common::{TestWorkspace, read_fixture};
use netlang_core::compiler::{COMPILE_SCHEMA_VERSION, CompileOptions, compile_source};
use serde_json::Value;
use std::fs;
use std::path::Path;

fn path_argument(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

#[test]
fn check_supports_human_and_machine_readable_success() {
    let workspace = TestWorkspace::new("check-success");
    let source = workspace.write("valid.nl", &read_fixture("valid/minimal.nl"));
    let source_arg = path_argument(&source);

    let human = workspace.run_cli(&["check", &source_arg]);
    assert_eq!(human.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&human.stdout).contains("[SUCCESS]"));
    assert!(human.stderr.is_empty());

    let json = workspace.run_cli(&["--format", "json", "check", &source_arg]);
    assert_eq!(json.status.code(), Some(0));
    assert!(json.stderr.is_empty());
    let value: Value = serde_json::from_slice(&json.stdout).expect("stdout must be one JSON value");
    assert_eq!(value["status"], "success");
    assert_eq!(value["schema_version"], COMPILE_SCHEMA_VERSION);
    assert_eq!(value["diagnostics"], serde_json::json!([]));
}

#[test]
fn io_parse_and_erc_failures_use_documented_exit_codes() {
    let workspace = TestWorkspace::new("failure-codes");

    let missing = workspace.path().join("missing.nl");
    let missing_arg = path_argument(&missing);
    let io_error = workspace.run_cli(&["check", &missing_arg]);
    assert_eq!(io_error.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&io_error.stderr).contains("Could not read file"));

    let parse_source = workspace.write(
        "parse-error.nl",
        &read_fixture("invalid/parser/missing_value.nl"),
    );
    let parse_arg = path_argument(&parse_source);
    let parse_error = workspace.run_cli(&["check", &parse_arg]);
    assert_eq!(parse_error.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&parse_error.stderr).contains("NL-P001"));

    let erc_source = workspace.write(
        "erc-error.nl",
        &read_fixture("invalid/semantic/floating_pin.nl"),
    );
    let erc_arg = path_argument(&erc_source);
    let erc_error = workspace.run_cli(&["check", &erc_arg]);
    assert_eq!(erc_error.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&erc_error.stderr).contains("NL-E003"));
}

#[test]
fn compile_writes_only_inside_its_isolated_workspace() {
    let workspace = TestWorkspace::new("compile-output");
    let source = workspace.write("circuit.nl", &read_fixture("valid/minimal.nl"));
    let source_arg = path_argument(&source);

    let output = workspace.run_cli(&["compile", &source_arg]);
    assert_eq!(output.status.code(), Some(0));
    assert!(source.with_extension("spice").is_file());
    assert_eq!(
        fs::read_dir(workspace.path())
            .expect("workspace should exist")
            .count(),
        2
    );

    let normalized = workspace.normalize_cli_text(&output.stdout);
    assert!(normalized.contains("<TEMP>/circuit.spice"), "{normalized}");
}

#[test]
fn cli_text_normalization_handles_crlf_native_and_json_paths() {
    let workspace = TestWorkspace::new("normalization");
    let native_root = workspace.path().to_string_lossy();
    let json_root = native_root.replace('\\', "\\\\");
    let input = format!("{native_root}\\file.nl\r\n{json_root}\\\\file.spice\r\n");

    assert_eq!(
        workspace.normalize_cli_text(input.as_bytes()),
        "<TEMP>/file.nl\n<TEMP>/file.spice\n"
    );
}

#[test]
fn format_is_a_true_global_option_before_or_after_the_subcommand() {
    let workspace = TestWorkspace::new("format-placement");
    let source = workspace.write("valid.nl", &read_fixture("valid/minimal.nl"));
    let source_arg = path_argument(&source);

    let supported = workspace.run_cli(&["--format", "json", "check", &source_arg]);
    assert_eq!(supported.status.code(), Some(0));

    let supported_after = workspace.run_cli(&["check", &source_arg, "--format", "json"]);
    assert_eq!(supported_after.status.code(), Some(0));
    assert!(supported_after.stderr.is_empty());
    serde_json::from_slice::<Value>(&supported_after.stdout)
        .expect("global JSON format should work after the subcommand");
}

#[test]
fn json_parse_errors_do_not_mix_logs_into_stdout() {
    let workspace = TestWorkspace::new("json-error");
    let source = workspace.write(
        "invalid.nl",
        &read_fixture("invalid/parser/missing_parenthesis.nl"),
    );
    let source_arg = path_argument(&source);

    let output = workspace.run_cli(&["--format", "json", "check", &source_arg]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value: Value =
        serde_json::from_slice(&output.stdout).expect("stdout must contain only valid JSON");
    assert_eq!(value["status"], "error");
    assert_eq!(value["schema_version"], COMPILE_SCHEMA_VERSION);
    assert_eq!(value["diagnostics"][0]["code"], "NL-P001");
    assert_eq!(value["diagnostics"][0]["stage"], "parse");
}

#[test]
fn semantic_errors_are_structured_and_use_the_semantic_exit_code() {
    let workspace = TestWorkspace::new("semantic-error");
    let source = workspace.write(
        "invalid.nl",
        &read_fixture("invalid/semantic/invalid_value.nl"),
    );
    let source_arg = path_argument(&source);

    let output = workspace.run_cli(&["--format", "json", "check", &source_arg]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let value: Value =
        serde_json::from_slice(&output.stdout).expect("stdout must contain only valid JSON");
    assert_eq!(value["status"], "error");
    assert_eq!(value["diagnostics"][0]["code"], "NL-C001");
    assert_eq!(value["diagnostics"][0]["component"], "R1");
    assert_eq!(value["diagnostics"][0]["pin"], Value::Null);
    assert_eq!(value["diagnostics"][0]["field"], "value");
}

#[test]
fn invalid_analysis_fails_before_simulator_launch() {
    let workspace = TestWorkspace::new("analysis-error");
    let source_text = read_fixture("valid/minimal.nl").replace("simulate op", "simulate noise");
    let source = workspace.write("invalid-analysis.nl", &source_text);
    let source_arg = path_argument(&source);

    let output = workspace.run_cli(&["simulate", &source_arg, "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let value: Value =
        serde_json::from_slice(&output.stdout).expect("analysis error should be structured JSON");
    assert_eq!(value["status"], "error");
    assert_eq!(value["diagnostics"][0]["code"], "NL-C009");
    assert_eq!(value["diagnostics"][0]["stage"], "semantic");
    assert!(!source.with_extension("spice").exists());
}

#[test]
fn compile_json_matches_the_canonical_library_report() {
    let workspace = TestWorkspace::new("canonical-report");
    let source_text = read_fixture("valid/minimal.nl");
    let source = workspace.write("circuit.nl", &source_text);
    let source_arg = path_argument(&source);

    let output = workspace.run_cli(&["compile", &source_arg, "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let cli_json: Value =
        serde_json::from_slice(&output.stdout).expect("CLI stdout should be one JSON report");

    let library_report = compile_source(
        &source_text,
        CompileOptions {
            include_ast: true,
            generate_spice: true,
            generate_layout: false,
            generate_kicad: false,
        },
    );
    let library_json =
        serde_json::to_value(library_report).expect("library report should serialize");

    for field in [
        "schema_version",
        "ast",
        "ir",
        "diagnostics",
        "graph",
        "spice_netlist",
    ] {
        assert_eq!(cli_json[field], library_json[field], "field {field}");
    }
}

#[test]
fn output_policy_requires_force_and_never_overwrites_the_source() {
    let workspace = TestWorkspace::new("overwrite-policy");
    let source = workspace.write("circuit.nl", &read_fixture("valid/minimal.nl"));
    let source_arg = path_argument(&source);

    let first = workspace.run_cli(&["compile", &source_arg]);
    assert_eq!(first.status.code(), Some(0));

    let refused = workspace.run_cli(&["compile", &source_arg, "--format", "json"]);
    assert_eq!(refused.status.code(), Some(2));
    let refused_json: Value =
        serde_json::from_slice(&refused.stdout).expect("refusal should be structured JSON");
    assert_eq!(refused_json["diagnostics"][0]["code"], "NL-I003");
    assert_eq!(refused_json["spice_netlist"], Value::Null);

    let forced = workspace.run_cli(&["compile", &source_arg, "--force"]);
    assert_eq!(forced.status.code(), Some(0));

    let source_before = fs::read_to_string(&source).expect("source should remain readable");
    let protected =
        workspace.run_cli(&["compile", &source_arg, "--output", &source_arg, "--force"]);
    assert_eq!(protected.status.code(), Some(2));
    assert_eq!(
        fs::read_to_string(&source).expect("source must not be overwritten"),
        source_before
    );
}

#[test]
fn render_stub_is_fail_closed_in_human_and_json_modes() {
    let workspace = TestWorkspace::new("render-stub");
    let source = workspace.write("circuit.nl", &read_fixture("valid/minimal.nl"));
    let source_arg = path_argument(&source);

    let human = workspace.run_cli(&["render", &source_arg]);
    assert_eq!(human.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&human.stderr).contains("NL-F001"));

    let json = workspace.run_cli(&["render", &source_arg, "--format", "json"]);
    assert_eq!(json.status.code(), Some(2));
    assert!(json.stderr.is_empty());
    let value: Value = serde_json::from_slice(&json.stdout).expect("stdout should be JSON only");
    assert_eq!(value["status"], "error");
    assert_eq!(value["diagnostics"][0]["code"], "NL-F001");
}

#[test]
fn simulator_process_status_and_json_status_cannot_disagree() {
    let workspace = TestWorkspace::new("simulator-status");
    let source = workspace.write("circuit.nl", &read_fixture("valid/minimal.nl"));
    let source_arg = path_argument(&source);
    let success_simulator =
        workspace.write_fake_simulator("success-simulator", "No. of Data Rows : 1", "", 0);

    let success = workspace.run_cli_with_env(
        &["simulate", &source_arg, "--format", "json"],
        "NETLANG_NGSPICE",
        &success_simulator,
    );
    assert_eq!(
        success.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&success.stdout)
    );
    assert!(success.stderr.is_empty());
    let success_json: Value =
        serde_json::from_slice(&success.stdout).expect("success stdout should be JSON only");
    assert_eq!(success_json["status"], "success");

    let failing_simulator =
        workspace.write_fake_simulator("failing-simulator", "", "Fatal error: singular matrix", 9);
    let failure = workspace.run_cli_with_env(
        &["simulate", &source_arg, "--format", "json", "--force"],
        "NETLANG_NGSPICE",
        &failing_simulator,
    );
    assert_eq!(failure.status.code(), Some(3));
    assert!(failure.stderr.is_empty());
    let failure_json: Value =
        serde_json::from_slice(&failure.stdout).expect("failure stdout should be JSON only");
    assert_eq!(failure_json["status"], "simulation_error");
    assert_eq!(failure_json["diagnostics"][0]["code"], "NL-S002");
}

#[test]
fn failed_assertion_has_structured_result_and_exit_code_four() {
    let workspace = TestWorkspace::new("assertion-status");
    let source_text = format!(
        "{}assert peak(I(V1)) < 100mA\n",
        read_fixture("valid/minimal.nl")
    );
    let source = workspace.write("circuit.nl", &source_text);
    let source_arg = path_argument(&source);
    let simulator = workspace.write_fake_simulator(
        "measurement-simulator",
        "peak_i_v1 = 2.0e-1\nNo. of Data Rows : 1",
        "",
        0,
    );

    let output = workspace.run_cli_with_env(
        &["test", &source_arg, "--format", "json"],
        "NETLANG_NGSPICE",
        &simulator,
    );
    assert_eq!(
        output.status.code(),
        Some(4),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stderr.is_empty());
    let value: Value =
        serde_json::from_slice(&output.stdout).expect("assertion stdout should be JSON only");
    assert_eq!(value["status"], "test_failed");
    assert_eq!(value["tests"][0]["pass"], false);
    assert_eq!(value["tests"][0]["actual"], 0.2);
}

#[test]
fn every_repository_example_has_an_explicit_cli_check_and_compile_outcome() {
    let workspace = TestWorkspace::new("example-matrix");
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");

    for name in [
        "demo_circuit.nl",
        "test_features.nl",
        "test_nc.nl",
        "wheatstone.nl",
    ] {
        let source = examples.join(name);
        let source_arg = path_argument(&source);
        let check = workspace.run_cli(&["check", &source_arg, "--format", "json"]);
        assert_eq!(check.status.code(), Some(0), "check failed for {name}");

        let output_path = workspace.path().join(name).with_extension("spice");
        let output_arg = path_argument(&output_path);
        let compile = workspace.run_cli(&[
            "compile",
            &source_arg,
            "--output",
            &output_arg,
            "--format",
            "json",
        ]);
        assert_eq!(compile.status.code(), Some(0), "compile failed for {name}");
        assert!(output_path.is_file(), "missing SPICE output for {name}");
    }

    let intentionally_invalid = examples.join("test_amp.nl");
    let invalid_arg = path_argument(&intentionally_invalid);
    let output = workspace.run_cli(&["check", &invalid_arg, "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    let value: Value =
        serde_json::from_slice(&output.stdout).expect("invalid example should return JSON");
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics should be an array")
            .iter()
            .any(|diagnostic| diagnostic["code"] == "NL-E003")
    );
}
