mod common;

use common::TestWorkspace;
use kessetsu_core::capabilities::CAPABILITIES_SCHEMA_VERSION;
use kessetsu_core::compiler::COMPILE_SCHEMA_VERSION;
use serde_json::Value;

const CLI_SCHEMA_VERSION: &str = "kessetsu.cli.v1";

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
fn capabilities_are_versioned_deterministic_and_side_effect_free() {
    let workspace = TestWorkspace::new("capabilities");
    let first = workspace.run_cli(&["capabilities", "--format", "json"]);
    let second = workspace.run_cli(&["capabilities", "--format", "json"]);

    assert_eq!(first.status.code(), Some(0));
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(first.stdout, second.stdout);
    let report = json(&first);
    assert_eq!(report["schema_version"], CLI_SCHEMA_VERSION);
    assert_eq!(report["command"], "capabilities");
    assert_eq!(report["status"], "success");
    assert_eq!(
        report["domain_versions"]["capabilities"],
        CAPABILITIES_SCHEMA_VERSION
    );
    assert_eq!(
        report["capabilities"]["schema_version"],
        CAPABILITIES_SCHEMA_VERSION
    );
    assert!(
        report["capabilities"]["language"]["measurement_metrics"]
            .as_array()
            .is_some_and(|metrics| metrics.iter().any(|metric| metric == "gain_at"))
    );
    assert!(
        report["capabilities"]["exports"]
            .as_array()
            .is_some_and(|exports| exports.iter().any(|export| export["format"] == "kicad"))
    );
    assert_eq!(
        std::fs::read_dir(workspace.path())
            .expect("workspace should be readable")
            .count(),
        0,
        "capability discovery must not create files"
    );
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
            "KESSETSU_NGSPICE",
            &simulator,
        );
        let second = workspace.run_cli_with_stdin_and_env(
            &arguments,
            &source,
            "KESSETSU_NGSPICE",
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
        COMPILE_SCHEMA_VERSION
    );

    let simulate = workspace.run_cli_with_stdin_and_env(
        &["simulate", "-", "--format", "json"],
        &initial,
        "KESSETSU_NGSPICE",
        &simulator,
    );
    assert_eq!(simulate.status.code(), Some(0));
    let simulate_json = json(&simulate);
    assert_eq!(simulate_json["status"], "success");
    assert_eq!(simulate_json["measurements"]["observed_current"], 0.2);
    assert_eq!(
        simulate_json["domain_versions"]["simulation"],
        "kessetsu.simulation.v1"
    );

    let failing_test = workspace.run_cli_with_stdin_and_env(
        &["test", "-", "--format", "json"],
        &initial,
        "KESSETSU_NGSPICE",
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
        "KESSETSU_NGSPICE",
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

#[test]
fn mcp_stdio_serves_initialize_tools_and_circuit_verification() {
    let workspace = TestWorkspace::new("mcp-stdio");
    let requests = [
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "protocolVersion": "2025-06-18" }
        })
        .to_string(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        })
        .to_string(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        })
        .to_string(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "kessetsu_check",
                "arguments": {
                    "source": "V1 IN 0 5\nR1 IN OUT 1k\nR2 OUT 0 1k\n.op\n.end\n"
                }
            }
        })
        .to_string(),
    ]
    .join("\n")
        + "\n";

    let output = workspace.run_cli_with_stdin(&["mcp"], &requests);
    assert_eq!(output.status.code(), Some(0));
    let lines: Vec<Value> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("each MCP stdout line must be JSON"))
        .collect();
    assert_eq!(lines.len(), 3, "notification must not emit a response");
    assert_eq!(lines[0]["id"], 1);
    assert_eq!(lines[0]["result"]["serverInfo"]["name"], "kessetsu");
    assert_eq!(lines[1]["id"], 2);
    assert!(lines[1]["result"]["tools"].as_array().is_some_and(|tools| {
        tools.iter().any(|t| t["name"] == "kessetsu_test")
            && tools.iter().any(|t| t["name"] == "kessetsu_tool")
            && tools.iter().any(|t| t["name"] == "kessetsu_study_plan")
            && tools.iter().any(|t| t["name"] == "kessetsu_study_run")
    }));
    assert_eq!(lines[2]["id"], 3);
    assert_eq!(lines[2]["result"]["isError"], false);
    assert_eq!(lines[2]["result"]["structuredContent"]["status"], "success");
}

#[test]
fn mcp_stdio_server_exposes_and_executes_verified_core_tools() {
    let workspace = TestWorkspace::new("mcp-tools-and-studies");
    let study_spec = serde_json::json!({
        "schema_version": "kessetsu.experiment.v1",
        "name": "DividerStudy",
        "source": "param resistance: Ohm = 1kOhm\nnet GND\nnet IN\nnet OUT\nsource VIN 5V\nresistor R1 {resistance}\nresistor R2 10kOhm\nconnect VIN.plus, R1.p1 to IN\nconnect R1.p2, R2.p1 to OUT\nconnect VIN.minus, R2.p2 to GND\nsimulate op\nassert peak(V(OUT)) > 1V\n",
        "axes": [
            {
                "parameter": "resistance",
                "values": {
                    "kind": "list",
                    "values": ["1kOhm", "2kOhm"]
                }
            }
        ],
        "temperatures_c": [27.0]
    });

    let requests = [
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "protocolVersion": "2025-06-18" }
        })
        .to_string(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        })
        .to_string(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        })
        .to_string(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "kessetsu_tool",
                "arguments": {
                    "tool": "divider",
                    "vin": "12V",
                    "target": "3V",
                    "lower": "10k",
                    "values": "e24"
                }
            }
        })
        .to_string(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "kessetsu_study_plan",
                "arguments": {
                    "spec": study_spec
                }
            }
        })
        .to_string(),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "kessetsu_study_run",
                "arguments": {
                    "spec": study_spec
                }
            }
        })
        .to_string(),
    ]
    .join("\n")
        + "\n";

    let output = workspace.run_cli_with_stdin(&["mcp"], &requests);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let lines: Vec<Value> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("each MCP stdout line must be JSON"))
        .collect();
    assert_eq!(lines.len(), 5, "notification must not emit a response");

    // id: 1 initialize
    assert_eq!(lines[0]["id"], 1);
    assert_eq!(lines[0]["result"]["serverInfo"]["name"], "kessetsu");

    // id: 2 tools/list includes kessetsu_tool, kessetsu_study_plan, kessetsu_study_run
    assert_eq!(lines[1]["id"], 2);
    let tool_names = lines[1]["result"]["tools"]
        .as_array()
        .expect("tools list")
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect::<Vec<_>>();
    assert!(tool_names.contains(&"kessetsu_tool"));
    assert!(tool_names.contains(&"kessetsu_study_plan"));
    assert!(tool_names.contains(&"kessetsu_study_run"));

    // id: 3 kessetsu_tool succeeds and returns calculation.source
    assert_eq!(lines[2]["id"], 3);
    assert_eq!(lines[2]["result"]["isError"], false);
    assert_eq!(lines[2]["result"]["structuredContent"]["status"], "success");
    assert!(
        lines[2]["result"]["structuredContent"]["calculation"]["source"]
            .as_str()
            .is_some_and(|s| s.contains("resistor"))
    );

    // id: 4 kessetsu_study_plan succeeds and returns study cases
    assert_eq!(lines[3]["id"], 4);
    assert_eq!(lines[3]["result"]["isError"], false);
    assert_eq!(lines[3]["result"]["structuredContent"]["status"], "success");
    assert_eq!(
        lines[3]["result"]["structuredContent"]["study"]["cases"]
            .as_array()
            .map(|cases| cases.len()),
        Some(2)
    );

    // id: 5 kessetsu_study_run succeeds and returns report
    assert_eq!(lines[4]["id"], 5);
    assert_eq!(lines[4]["result"]["isError"], false);
    assert_eq!(lines[4]["result"]["structuredContent"]["status"], "success");
    assert_eq!(
        lines[4]["result"]["structuredContent"]["study"]["summary"]["passed"],
        2
    );
}
