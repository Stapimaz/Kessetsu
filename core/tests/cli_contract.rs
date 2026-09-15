mod common;

use common::{TestWorkspace, read_fixture};
use kessetsu_core::compiler::{COMPILE_SCHEMA_VERSION, CompileOptions, compile_source};
use serde_json::Value;
use std::fs;
use std::path::Path;

const CLI_SCHEMA_VERSION: &str = "kessetsu.cli.v1";

fn path_argument(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

#[test]
fn help_exposes_project_license_source_and_warranty_notice() {
    let workspace = TestWorkspace::new("help-license");
    let output = workspace.run_cli(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("AGPL-3.0-only"));
    assert!(stdout.contains("No warranty"));
    assert!(stdout.contains("https://github.com/Stapimaz/Kessetsu"));
}

#[test]
fn check_supports_human_and_machine_readable_success() {
    let workspace = TestWorkspace::new("check-success");
    let source = workspace.write("valid.kess", &read_fixture("valid/minimal.kess"));
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
    assert_eq!(value["schema_version"], CLI_SCHEMA_VERSION);
    assert_eq!(value["domain_versions"]["compile"], COMPILE_SCHEMA_VERSION);
    assert_eq!(value["diagnostics"], serde_json::json!([]));
    assert!(value.get("debug").is_none());
}

#[test]
fn io_parse_and_erc_failures_use_documented_exit_codes() {
    let workspace = TestWorkspace::new("failure-codes");

    let missing = workspace.path().join("missing.kess");
    let missing_arg = path_argument(&missing);
    let io_error = workspace.run_cli(&["check", &missing_arg]);
    assert_eq!(io_error.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&io_error.stderr).contains("Could not read file"));

    let parse_source = workspace.write(
        "parse-error.kess",
        &read_fixture("invalid/parser/missing_value.kess"),
    );
    let parse_arg = path_argument(&parse_source);
    let parse_error = workspace.run_cli(&["check", &parse_arg]);
    assert_eq!(parse_error.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&parse_error.stderr).contains("KES-P001"));

    let erc_source = workspace.write(
        "erc-error.kess",
        &read_fixture("invalid/semantic/floating_pin.kess"),
    );
    let erc_arg = path_argument(&erc_source);
    let erc_error = workspace.run_cli(&["check", &erc_arg]);
    assert_eq!(erc_error.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&erc_error.stderr).contains("KES-E003"));
}

#[test]
fn compile_writes_only_inside_its_isolated_workspace() {
    let workspace = TestWorkspace::new("compile-output");
    let source = workspace.write("circuit.kess", &read_fixture("valid/minimal.kess"));
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
    let input = format!("{native_root}\\file.kess\r\n{json_root}\\\\file.spice\r\n");

    assert_eq!(
        workspace.normalize_cli_text(input.as_bytes()),
        "<TEMP>/file.kess\n<TEMP>/file.spice\n"
    );
}

#[test]
fn format_is_a_true_global_option_before_or_after_the_subcommand() {
    let workspace = TestWorkspace::new("format-placement");
    let source = workspace.write("valid.kess", &read_fixture("valid/minimal.kess"));
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
        "invalid.kess",
        &read_fixture("invalid/parser/missing_parenthesis.kess"),
    );
    let source_arg = path_argument(&source);

    let output = workspace.run_cli(&["--format", "json", "check", &source_arg]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value: Value =
        serde_json::from_slice(&output.stdout).expect("stdout must contain only valid JSON");
    assert_eq!(value["status"], "error");
    assert_eq!(value["schema_version"], CLI_SCHEMA_VERSION);
    assert_eq!(value["domain_versions"]["compile"], COMPILE_SCHEMA_VERSION);
    assert_eq!(value["diagnostics"][0]["code"], "KES-P001");
    assert_eq!(value["diagnostics"][0]["stage"], "parse");
}

#[test]
fn semantic_errors_are_structured_and_use_the_semantic_exit_code() {
    let workspace = TestWorkspace::new("semantic-error");
    let source = workspace.write(
        "invalid.kess",
        &read_fixture("invalid/semantic/invalid_value.kess"),
    );
    let source_arg = path_argument(&source);

    let output = workspace.run_cli(&["--format", "json", "check", &source_arg]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let value: Value =
        serde_json::from_slice(&output.stdout).expect("stdout must contain only valid JSON");
    assert_eq!(value["status"], "error");
    assert_eq!(value["diagnostics"][0]["code"], "KES-C001");
    assert_eq!(value["diagnostics"][0]["component"], "R1");
    assert_eq!(value["diagnostics"][0]["pin"], Value::Null);
    assert_eq!(value["diagnostics"][0]["field"], "value");
}

#[test]
fn invalid_analysis_fails_before_simulator_launch() {
    let workspace = TestWorkspace::new("analysis-error");
    let source_text = read_fixture("valid/minimal.kess").replace("simulate op", "simulate noise");
    let source = workspace.write("invalid-analysis.kess", &source_text);
    let source_arg = path_argument(&source);

    let output = workspace.run_cli(&["simulate", &source_arg, "--format", "json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let value: Value =
        serde_json::from_slice(&output.stdout).expect("analysis error should be structured JSON");
    assert_eq!(value["status"], "error");
    assert_eq!(value["diagnostics"][0]["code"], "KES-C009");
    assert_eq!(value["diagnostics"][0]["stage"], "semantic");
    assert!(!source.with_extension("spice").exists());
}

#[test]
fn compile_debug_json_matches_the_canonical_library_report() {
    let workspace = TestWorkspace::new("canonical-report");
    let source_text = read_fixture("valid/minimal.kess");
    let source = workspace.write("circuit.kess", &source_text);
    let source_arg = path_argument(&source);

    let output = workspace.run_cli(&[
        "compile",
        &source_arg,
        "--format",
        "json",
        "--include",
        "ast,ir,graph,spice",
    ]);
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

    assert_eq!(cli_json["schema_version"], CLI_SCHEMA_VERSION);
    assert_eq!(
        cli_json["domain_versions"]["compile"],
        library_json["schema_version"]
    );
    for field in ["ast", "ir", "graph", "spice_netlist"] {
        assert_eq!(
            cli_json["debug"][field], library_json[field],
            "field {field}"
        );
    }
}

#[test]
fn default_json_is_compact_and_verbose_fields_are_explicitly_opt_in() {
    let workspace = TestWorkspace::new("compact-json");
    let source = workspace.write("circuit.kess", &read_fixture("valid/minimal.kess"));
    let source_arg = path_argument(&source);

    let output = workspace.run_cli(&["compile", &source_arg, "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    let value: Value = serde_json::from_slice(&output.stdout).expect("compact output should parse");
    assert!(value.get("debug").is_none());
    assert!(value.get("ast").is_none());
    assert!(value.get("ir").is_none());
    assert!(value.get("graph").is_none());
    assert!(value.get("spice_netlist").is_none());
    assert_eq!(value["summary"]["errors"], 0);
    assert_eq!(value["artifacts"][0]["kind"], "spice_netlist");
}

#[test]
fn unknown_cli_schema_fails_closed_before_creating_output() {
    let workspace = TestWorkspace::new("unknown-schema");
    let source = workspace.write("circuit.kess", &read_fixture("valid/minimal.kess"));
    let source_arg = path_argument(&source);

    let output = workspace.run_cli(&[
        "compile",
        &source_arg,
        "--format",
        "json",
        "--schema-version",
        "kessetsu.cli.v999",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    assert!(!source.with_extension("spice").exists());
    let value: Value = serde_json::from_slice(&output.stdout).expect("schema error should parse");
    assert_eq!(value["schema_version"], CLI_SCHEMA_VERSION);
    assert_eq!(value["diagnostics"][0]["code"], "KES-F002");
}

#[test]
fn output_policy_requires_force_and_never_overwrites_the_source() {
    let workspace = TestWorkspace::new("overwrite-policy");
    let source = workspace.write("circuit.kess", &read_fixture("valid/minimal.kess"));
    let source_arg = path_argument(&source);

    let first = workspace.run_cli(&["compile", &source_arg]);
    assert_eq!(first.status.code(), Some(0));

    let refused = workspace.run_cli(&["compile", &source_arg, "--format", "json"]);
    assert_eq!(refused.status.code(), Some(2));
    let refused_json: Value =
        serde_json::from_slice(&refused.stdout).expect("refusal should be structured JSON");
    assert_eq!(refused_json["diagnostics"][0]["code"], "KES-I003");
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
fn render_and_export_emit_versioned_artifacts_with_safe_overwrite() {
    let workspace = TestWorkspace::new("render-export");
    let source = workspace.write("circuit.kess", &read_fixture("valid/minimal.kess"));
    let source_arg = path_argument(&source);
    let png = workspace.path().join("circuit.png");
    let png_arg = path_argument(&png);

    let human = workspace.run_cli(&["render", &source_arg, "-o", &png_arg]);
    assert_eq!(human.status.code(), Some(0));
    assert!(png.is_file());
    assert_eq!(&fs::read(&png).unwrap()[..8], b"\x89PNG\r\n\x1a\n");

    let refused = workspace.run_cli(&["render", &source_arg, "-o", &png_arg, "--format", "json"]);
    assert_eq!(refused.status.code(), Some(2));
    let refused: Value = serde_json::from_slice(&refused.stdout).unwrap();
    assert_eq!(refused["diagnostics"][0]["code"], "KES-I003");

    let kicad = workspace.path().join("circuit.kicad_sch");
    let kicad_arg = path_argument(&kicad);
    let json = workspace.run_cli(&[
        "export",
        &source_arg,
        "--target",
        "kicad",
        "-o",
        &kicad_arg,
        "--format",
        "json",
    ]);
    assert_eq!(json.status.code(), Some(0));
    assert!(json.stderr.is_empty());
    let value: Value = serde_json::from_slice(&json.stdout).expect("stdout should be JSON only");
    assert_eq!(value["domain_versions"]["export"], "kessetsu.export.v1");
    assert_eq!(
        value["artifacts"][0]["schema_version"],
        "kessetsu.export.v1"
    );
    assert_eq!(value["artifacts"][0]["connectivity_verified"], true);
    assert_eq!(value["artifacts"][0]["sha256"].as_str().unwrap().len(), 64);
}

#[test]
fn simulator_process_status_and_json_status_cannot_disagree() {
    let workspace = TestWorkspace::new("simulator-status");
    let source = workspace.write("circuit.kess", &read_fixture("valid/minimal.kess"));
    let source_arg = path_argument(&source);
    let success_simulator =
        workspace.write_fake_simulator("success-simulator", "No. of Data Rows : 1", "", 0);

    let success = workspace.run_cli_with_env(
        &["simulate", &source_arg, "--format", "json"],
        "KESSETSU_NGSPICE",
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
    assert_eq!(
        success_json["domain_versions"]["simulation"],
        "kessetsu.simulation.v1"
    );
    assert_eq!(success_json["summary"]["analyses"], 1);
    assert!(success_json.get("debug").is_none());

    let verbose = workspace.run_cli_with_env(
        &[
            "simulate",
            &source_arg,
            "--format",
            "json",
            "--force",
            "--include",
            "datasets,raw-log",
        ],
        "KESSETSU_NGSPICE",
        &success_simulator,
    );
    assert_eq!(verbose.status.code(), Some(0));
    let verbose_json: Value =
        serde_json::from_slice(&verbose.stdout).expect("verbose result should be JSON");
    assert_eq!(verbose_json["debug"]["datasets"][0]["index"], 0);
    assert!(
        verbose_json["debug"]["raw_log"]["stdout"]
            .as_str()
            .is_some_and(|log| log.contains("No. of Data Rows"))
    );

    let human = workspace.run_cli_with_env(
        &["simulate", &source_arg, "--force"],
        "KESSETSU_NGSPICE",
        &success_simulator,
    );
    assert_eq!(human.status.code(), Some(0));
    assert!(human.stderr.is_empty());
    let human_stdout = String::from_utf8_lossy(&human.stdout);
    assert!(human_stdout.contains("1 analysis dataset(s), 0 measurement(s)"));
    assert!(human_stdout.contains("[SUCCESS] Simulation completed."));

    let failing_simulator =
        workspace.write_fake_simulator("failing-simulator", "", "Fatal error: singular matrix", 9);
    let failure = workspace.run_cli_with_env(
        &["simulate", &source_arg, "--format", "json", "--force"],
        "KESSETSU_NGSPICE",
        &failing_simulator,
    );
    assert_eq!(failure.status.code(), Some(3));
    assert!(failure.stderr.is_empty());
    let failure_json: Value =
        serde_json::from_slice(&failure.stdout).expect("failure stdout should be JSON only");
    assert_eq!(failure_json["status"], "simulation_error");
    assert_eq!(failure_json["diagnostics"][0]["code"], "KES-S002");
}

#[test]
fn failed_assertion_has_structured_result_and_exit_code_four() {
    let workspace = TestWorkspace::new("assertion-status");
    let source_text = format!(
        "{}assert peak(I(V1)) < 100mA\n",
        read_fixture("valid/minimal.kess")
    );
    let source = workspace.write("circuit.kess", &source_text);
    let source_arg = path_argument(&source);
    let simulator = workspace.write_fake_simulator(
        "measurement-simulator",
        "peak_i_v1 = 2.0e-1\nNo. of Data Rows : 1",
        "",
        0,
    );

    let output = workspace.run_cli_with_env(
        &["test", &source_arg, "--format", "json"],
        "KESSETSU_NGSPICE",
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
    assert_eq!(
        value["assertions"]["schema_version"],
        "kessetsu.assertion.v1"
    );
    assert_eq!(value["assertions"]["assertions"][0]["code"], "KES-T001");
    assert_eq!(value["assertions"]["assertions"][0]["status"], "FAIL");
    assert_eq!(value["assertions"]["assertions"][0]["actual"], 0.2);
    assert_eq!(value["assertions"]["summary"]["failed"], 1);
}

#[test]
fn test_without_assertions_fails_before_launching_the_simulator() {
    let workspace = TestWorkspace::new("zero-assertions");
    let source = workspace.write("circuit.kess", &read_fixture("valid/minimal.kess"));
    let source_arg = path_argument(&source);
    let missing_simulator = workspace.path().join("must-not-run-ngspice");

    let human = workspace.run_cli_with_env(
        &["test", &source_arg, "--force"],
        "KESSETSU_NGSPICE",
        &missing_simulator,
    );
    assert_eq!(human.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&human.stderr).contains("KES-T000"));
    assert!(String::from_utf8_lossy(&human.stderr).contains("no assertions"));

    let json = workspace.run_cli_with_env(
        &["test", &source_arg, "--format", "json", "--force"],
        "KESSETSU_NGSPICE",
        &missing_simulator,
    );
    assert_eq!(json.status.code(), Some(4));
    assert!(json.stderr.is_empty());
    let value: Value = serde_json::from_slice(&json.stdout).expect("failure should be JSON only");
    assert_eq!(value["status"], "test_failed");
    assert_eq!(value["diagnostics"][0]["code"], "KES-T000");
    assert_eq!(value["diagnostics"][0]["stage"], "assertion");
    assert_eq!(value["assertions"]["summary"]["total"], 0);
    assert_eq!(value["summary"]["assertions"]["total"], 0);
}

#[test]
fn every_repository_example_has_an_explicit_cli_check_and_compile_outcome() {
    let workspace = TestWorkspace::new("example-matrix");
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");

    for name in [
        "demo_circuit.kess",
        "test_features.kess",
        "test_nc.kess",
        "wheatstone.kess",
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

    let intentionally_invalid = examples.join("test_amp.kess");
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
            .any(|diagnostic| diagnostic["code"] == "KES-E003")
    );
}
