//! Minimal Model Context Protocol (MCP) server over stdio.
//!
//! This is a thin adapter: every tool re-invokes this same `kess` executable with the
//! versioned `kessetsu.cli.v1` JSON contract and returns that JSON to the agent. It owns no
//! circuit semantics, so MCP results are byte-for-byte the CLI results.

use serde_json::{Value, json};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

const PROTOCOL_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18"];
const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";
const GUIDE: &str = include_str!("../../../../docs/guides/agent-workflow.md");
const INSTRUCTIONS: &str = "Kessetsu verifies analog circuits. Available tools: kessetsu_guide, kessetsu_capabilities, kessetsu_tool (calculators returning components, editable .kess, and share_url), kessetsu_import_spice, kessetsu_check, kessetsu_simulate (supports measure expressions), kessetsu_test, kessetsu_render_schematic, kessetsu_export, kessetsu_study_plan, kessetsu_study_run, and kessetsu_share_link. Workflow: (1) write measurable requirements as .kessreq assertion lines before designing; (2) write readable .kess source (or import SPICE via kessetsu_import_spice, or start from kessetsu_tool) with named nets; (3) kessetsu_check and fix ERC errors; (4) kessetsu_test with the unchanged requirements (and optional kessetsu_study_plan / kessetsu_study_run sweeps or tolerances) iterating on the design only; (5) kessetsu_render_schematic for human review; (6) kessetsu_export for handoff and call kessetsu_share_link so you include the clickable https://kessetsu.com/#kessetsu=1... link when presenting a verified circuit; (7) report the PASS table, share URL, and assumptions honestly. Call kessetsu_guide once for syntax and metrics.";

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn serve() -> i32 {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(message) => handle_message(&message),
            Err(error) => Some(error_response(
                Value::Null,
                -32700,
                &format!("parse error: {error}"),
            )),
        };
        if let Some(response) = response
            && (writeln!(stdout, "{response}").is_err() || stdout.flush().is_err())
        {
            break;
        }
    }
    0
}

fn handle_message(message: &Value) -> Option<Value> {
    let id = message.get("id").cloned();
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    // Notifications (no id) never receive a response.
    let id = id?;
    let result = match method {
        "initialize" => Ok(initialize(&params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => call_tool(&params),
        "prompts/list" => Ok(json!({ "prompts": [design_prompt_definition()] })),
        "prompts/get" => get_prompt(&params),
        "resources/list" => Ok(json!({ "resources": [] })),
        _ => Err((-32601, format!("method '{method}' is not supported"))),
    };
    Some(match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => error_response(id, code, &message),
    })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn initialize(params: &Value) -> Value {
    let requested = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL_VERSION);
    let version = if PROTOCOL_VERSIONS.contains(&requested) {
        requested
    } else {
        DEFAULT_PROTOCOL_VERSION
    };
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {}, "prompts": {} },
        "serverInfo": { "name": "kessetsu", "version": env!("CARGO_PKG_VERSION") },
        "instructions": INSTRUCTIONS,
    })
}

fn source_properties() -> Value {
    json!({
        "source": {
            "type": "string",
            "description": "Complete circuit text: Kessetsu .kess source or a supported SPICE netlist. Provide this or 'path'."
        },
        "path": {
            "type": "string",
            "description": "Path to a .kess or SPICE file to read instead of 'source' (relative to the server working directory)."
        },
        "input": {
            "type": "string",
            "enum": ["auto", "kess", "spice"],
            "description": "Source language; 'auto' detects SPICE by extension or dot directives."
        },
        "parameters": {
            "type": "object",
            "additionalProperties": { "type": "string" },
            "description": "Optional root parameter overrides, e.g. {\"supply\": \"15V\"}."
        }
    })
}

fn tool(name: &str, title: &str, description: &str, extra: Value, required: &[&str]) -> Value {
    let mut properties = source_properties();
    if let (Some(base), Some(extra)) = (properties.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            base.insert(key.clone(), value.clone());
        }
    }
    json!({
        "name": name,
        "title": title,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false
        },
    })
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "kessetsu_guide",
            "title": "Kessetsu design workflow and syntax",
            "description": "Returns the disciplined agent workflow, .kess syntax, .kessreq requirement syntax and metric list. Call once before designing.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
        }),
        json!({
            "name": "kessetsu_capabilities",
            "title": "Installed Kessetsu capabilities",
            "description": "Versioned manifest of commands, contracts, metrics, exports and limits of this installed build.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
        }),
        tool(
            "kessetsu_check",
            "Check circuit (parse + ERC)",
            "Parses the circuit and runs Electrical Rules Check (floating pins, shorted sources, ambiguous ground, naming conflicts) without simulating. Fix errors here before simulating.",
            json!({}),
            &[],
        ),
        tool(
            "kessetsu_simulate",
            "Simulate circuit",
            "Runs the declared analyses (op/tran/ac/dc) through Ngspice and returns typed results, requested measurement expressions, OP summaries, and any inline assertion outcomes.",
            json!({
                "measure": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional measurement metric expressions to evaluate after simulation (e.g. ['value(V(OUT))', 'max(P(R1))', 'bandwidth(V(OUT),V(IN))'])."
                },
                "include": {
                    "type": "array",
                    "items": { "type": "string", "enum": ["spice", "datasets", "simulation", "models", "raw-log", "effective-source", "ir", "graph"] },
                    "description": "Optional verbose fields. 'effective-source' returns the canonical .kess text (useful after SPICE input)."
                }
            }),
            &[],
        ),
        tool(
            "kessetsu_test",
            "Test circuit against requirements",
            "Simulates and evaluates requirements. Pass independent acceptance criteria in 'requirements' (.kessreq assertion lines) so the design cannot rewrite its own limits; omit it to use inline assert lines. Returns PASS/FAIL/ERROR/SKIPPED per assertion with measured values.",
            json!({
                "requirements": {
                    "type": "string",
                    "description": ".kessreq text: one 'assert metric(...) op limit' per line, literal limits only."
                },
                "requirements_path": {
                    "type": "string",
                    "description": "Path to a .kessreq file instead of 'requirements'."
                },
                "requirements_sha256": {
                    "type": "string",
                    "description": "Optional expected sha256 of the requirements; the run fails closed if they changed."
                }
            }),
            &[],
        ),
        tool(
            "kessetsu_render_schematic",
            "Render schematic",
            "Renders the verified canonical schematic. Returns a PNG image (or SVG text) so you and the human can review the circuit. Optionally saves it to 'output_path'.",
            json!({
                "format": { "type": "string", "enum": ["png", "svg"], "description": "Default png." },
                "output_path": { "type": "string", "description": "Optional file to save (.png/.svg/.pdf)." },
                "overwrite": { "type": "boolean", "description": "Allow replacing an existing output_path." }
            }),
            &[],
        ),
        tool(
            "kessetsu_export",
            "Export engineering artifact",
            "Exports KiCad schematic, LTspice schematic, BOM CSV, canonical SPICE, schematic JSON or a handoff manifest. Returns the text and optionally saves it.",
            json!({
                "target": {
                    "type": "string",
                    "enum": ["kicad", "ltspice", "bom-csv", "spice", "schematic-json", "handoff-json"]
                },
                "output_path": { "type": "string", "description": "Optional file to save." },
                "overwrite": { "type": "boolean", "description": "Allow replacing an existing output_path." }
            }),
            &["target"],
        ),
        json!({
            "name": "kessetsu_import_spice",
            "title": "Convert SPICE to Kessetsu source",
            "description": "Converts a supported Ngspice-style netlist into readable, canonically verified .kess source with a name mapping. Unsupported constructs fail with line numbers.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "netlist": { "type": "string", "description": "SPICE netlist text." },
                    "path": { "type": "string", "description": "Path to a SPICE file instead of 'netlist'." }
                },
                "additionalProperties": false
            },
        }),
        json!({
            "name": "kessetsu_tool",
            "title": "Nominal circuit calculator",
            "description": "Run Kessetsu's nominal analytical circuit calculators (divider for voltage dividers, rc_lowpass for passive RC low-pass filters, non_inverting_opamp for non-inverting op-amp gain stages, and zener_regulator for Zener shunt regulators) with exact, E12, or E24 preferred value selection, returning calculated component values and editable .kess source.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tool": {
                        "type": "string",
                        "enum": ["divider", "rc_lowpass", "non_inverting_opamp", "zener_regulator"],
                        "description": "Calculator to run."
                    },
                    "vin": {
                        "type": "string",
                        "description": "Input voltage for divider (e.g. '12V'), zener_regulator (e.g. '12V'), or optional input voltage for non_inverting_opamp (default '0.5V')."
                    },
                    "target": {
                        "type": "string",
                        "description": "Target output voltage for divider (e.g. '3.3V')."
                    },
                    "lower": {
                        "type": "string",
                        "description": "Lower resistor value for divider (e.g. '10k')."
                    },
                    "load": {
                        "type": "string",
                        "description": "Optional load resistance for divider (e.g. '100k')."
                    },
                    "cutoff": {
                        "type": "string",
                        "description": "Target -3 dB cutoff frequency for rc_lowpass (e.g. '1kHz')."
                    },
                    "resistance": {
                        "type": "string",
                        "description": "Series resistor value for rc_lowpass (e.g. '10k')."
                    },
                    "target_gain": {
                        "type": "string",
                        "description": "Target dimensionless voltage gain V/V (> 1) for non_inverting_opamp (e.g. '10')."
                    },
                    "input_resistance": {
                        "type": "string",
                        "description": "Grounded inverting-input resistor R1 for non_inverting_opamp (e.g. '10k')."
                    },
                    "input_voltage": {
                        "type": "string",
                        "description": "Input voltage for non_inverting_opamp (optional, default '0.5V') or zener_regulator (e.g. '12V')."
                    },
                    "supply_voltage": {
                        "type": "string",
                        "description": "Optional symmetric supply rail magnitude ±Vsupply (> 2V, default '15V') for non_inverting_opamp."
                    },
                    "zener_voltage": {
                        "type": "string",
                        "description": "Target Zener breakdown voltage for zener_regulator ('3.3V', '5.1V', or '12V')."
                    },
                    "load_current": {
                        "type": "string",
                        "description": "Target load current (> 0) for zener_regulator (e.g. '10mA')."
                    },
                    "zener_knee_current": {
                        "type": "string",
                        "description": "Optional minimum Zener bias current (> 0, default '5mA') for zener_regulator."
                    },
                    "values": {
                        "type": "string",
                        "enum": ["exact", "e12", "e24"],
                        "default": "e24",
                        "description": "Nominal standard value series."
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Optional .kess file path to write the generated circuit."
                    },
                    "overwrite": {
                        "type": "boolean",
                        "description": "Allow overwriting output_path if it already exists."
                    }
                },
                "required": ["tool"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "kessetsu_study_plan",
            "title": "Plan parameter or tolerance study",
            "description": "Create or validate a deterministic Kessetsu parameter/tolerance/temperature study plan (kessetsu.experiment.v1) and return its case matrix and identity without running simulations.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "spec": {
                        "type": ["object", "string"],
                        "description": "Inline kessetsu.experiment.v1 study specification (JSON object or JSON string). Mutually exclusive with spec_path, source, and path."
                    },
                    "spec_path": {
                        "type": "string",
                        "description": "Path to an existing .kessstudy.json file on disk. Mutually exclusive with spec, source, and path."
                    },
                    "source": {
                        "type": "string",
                        "description": "Inline .kess circuit source for quick sweep plan creation. Mutually exclusive with path, spec, and spec_path."
                    },
                    "path": {
                        "type": "string",
                        "description": "Path to a .kess circuit file for quick sweep plan creation. Mutually exclusive with source, spec, and spec_path."
                    },
                    "sweep": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Parameter sweeps when creating a plan from source or path (e.g. ['R1=9k,10k,11k'])."
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Optional path to save the generated .kessstudy.json specification."
                    },
                    "overwrite": {
                        "type": "boolean",
                        "description": "Allow overwriting output_path if it already exists."
                    }
                },
                "additionalProperties": false
            }
        }),
        json!({
            "name": "kessetsu_study_run",
            "title": "Run parameter or tolerance study",
            "description": "Execute a deterministic Kessetsu parameter, tolerance-corner, Monte Carlo, or temperature study (kessetsu.experiment.v1) in Ngspice and return the structured kessetsu.experiment-results.v1 verification report.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "spec": {
                        "type": ["object", "string"],
                        "description": "Inline kessetsu.experiment.v1 study specification (JSON object or JSON string)."
                    },
                    "spec_path": {
                        "type": "string",
                        "description": "Path to an existing .kessstudy.json file on disk."
                    },
                    "source": {
                        "type": "string",
                        "description": "Inline .kess circuit source for a quick sweep study run."
                    },
                    "path": {
                        "type": "string",
                        "description": "Path to a .kess circuit file for a quick sweep study run."
                    },
                    "sweep": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Parameter sweeps when running from source or path (e.g. ['R1=9k,10k,11k'])."
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Optional file path to save the study results JSON."
                    },
                    "resume": {
                        "type": "boolean",
                        "description": "Resume an interrupted study run from existing output_path."
                    },
                    "overwrite": {
                        "type": "boolean",
                        "description": "Allow overwriting output_path if it already exists."
                    },
                    "include": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["datasets"] },
                        "description": "Optional extra sections to include (only 'datasets' is supported)."
                    }
                },
                "additionalProperties": false
            }
        }),
        tool(
            "kessetsu_share_link",
            "Generate clickable share URL",
            "Compiles the circuit (converting SPICE if needed) to validate ERC and extract the model manifest, returning a kessetsu.share_link.v1 payload with a clickable https://kessetsu.com/#kessetsu=1... web workbench URL.",
            json!({
                "name": {
                    "type": "string",
                    "description": "Optional display name embedded in the share link (1..=80 characters, no control characters)."
                },
                "input_format": {
                    "type": "string",
                    "enum": ["auto", "kess", "spice"],
                    "description": "Alias for 'input' selecting circuit language ('auto', 'kess', or 'spice')."
                }
            }),
            &[],
        ),
    ]
}

fn design_prompt_definition() -> Value {
    json!({
        "name": "design_circuit",
        "title": "Design a circuit with Kessetsu",
        "description": "Start a traceable, requirement-driven circuit design using Kessetsu tools.",
        "arguments": [
            { "name": "goal", "description": "What the circuit must do, with any known limits.", "required": true }
        ]
    })
}

fn get_prompt(params: &Value) -> Result<Value, (i64, String)> {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    if name != "design_circuit" {
        return Err((-32602, format!("unknown prompt '{name}'")));
    }
    let goal = params
        .pointer("/arguments/goal")
        .and_then(Value::as_str)
        .unwrap_or("(describe the circuit)");
    let text = format!(
        "Design this circuit with Kessetsu: {goal}\n\nAvailable MCP tools: kessetsu_guide, kessetsu_capabilities, kessetsu_tool, kessetsu_import_spice, kessetsu_check, kessetsu_simulate, kessetsu_test, kessetsu_render_schematic, kessetsu_export, kessetsu_study_plan, kessetsu_study_run, kessetsu_share_link.\n\nFollow the Kessetsu workflow exactly:\n1. Call kessetsu_guide.\n2. Write measurable requirements (.kessreq) first and show them to me before designing; save them as requirements.kessreq.\n3. Write design.kess with named nets and a target comment (you may seed components via kessetsu_tool or import SPICE via kessetsu_import_spice).\n4. kessetsu_check, then kessetsu_test with the unchanged requirements (and kessetsu_study_plan / kessetsu_study_run if sweeps/tolerances are needed); iterate on the design only.\n5. Render and save schematic.svg; export anything needed for handoff and call kessetsu_share_link so you include the clickable https://kessetsu.com/#kessetsu=1... link when presenting the verified circuit.\n6. Finish with NOTES.md: requirements, final PASS table, clickable share URL, assumptions and open risks."
    );
    Ok(json!({
        "description": "Kessetsu requirement-driven design",
        "messages": [ { "role": "user", "content": { "type": "text", "text": text } } ]
    }))
}

fn call_tool(params: &Value) -> Result<Value, (i64, String)> {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let empty = json!({});
    let arguments = params.get("arguments").unwrap_or(&empty);
    let outcome = match name {
        "kessetsu_guide" => return Ok(text_result(GUIDE.to_string(), false)),
        "kessetsu_capabilities" => run_cli(&["capabilities"], None),
        "kessetsu_check" => circuit_command("check", arguments, &[]),
        "kessetsu_simulate" => {
            let mut extra = Vec::new();
            if let Some(measures) = arguments.get("measure") {
                let Some(items) = measures.as_array() else {
                    return Ok(text_result(
                        "'measure' must be an array of strings".into(),
                        true,
                    ));
                };
                for item in items {
                    let Some(expr) = item.as_str() else {
                        return Ok(text_result(
                            "every item in 'measure' must be a string".into(),
                            true,
                        ));
                    };
                    extra.push("--measure".to_string());
                    extra.push(expr.to_string());
                }
            }
            if let Some(include) = arguments.get("include").and_then(Value::as_array) {
                let joined = include
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(",");
                if !joined.is_empty() {
                    extra.push("--include".to_string());
                    extra.push(joined);
                }
            }
            circuit_command("simulate", arguments, &extra)
        }
        "kessetsu_test" => test_command(arguments),
        "kessetsu_render_schematic" => return render_command(arguments),
        "kessetsu_export" => return export_command(arguments),
        "kessetsu_import_spice" => return Ok(import_command(arguments)),
        "kessetsu_tool" => tool_command(arguments),
        "kessetsu_study_plan" => study_plan_command(arguments),
        "kessetsu_study_run" => study_run_command(arguments),
        "kessetsu_share_link" => return Ok(share_link_command(arguments)),
        _ => return Err((-32602, format!("unknown tool '{name}'"))),
    };
    Ok(match outcome {
        Ok(output) => cli_result(output),
        Err(message) => text_result(message, true),
    })
}

struct CliOutput {
    exit_code: i32,
    stdout: String,
    stderr: String,
}

fn text_result(text: String, is_error: bool) -> Value {
    json!({ "content": [ { "type": "text", "text": text } ], "isError": is_error })
}

fn cli_result(output: CliOutput) -> Value {
    match serde_json::from_str::<Value>(&output.stdout) {
        Ok(parsed) => {
            let status = parsed
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let text = format!(
                "status: {status} (exit code {})\n{}",
                output.exit_code,
                serde_json::to_string_pretty(&parsed).unwrap_or(output.stdout)
            );
            json!({
                "content": [ { "type": "text", "text": text } ],
                "structuredContent": parsed,
                "isError": false
            })
        }
        Err(_) => text_result(
            format!(
                "kess exited with code {} without JSON output.\nstdout:\n{}\nstderr:\n{}",
                output.exit_code, output.stdout, output.stderr
            ),
            true,
        ),
    }
}

fn run_cli(arguments: &[&str], stdin: Option<&str>) -> Result<CliOutput, String> {
    let executable =
        std::env::current_exe().map_err(|error| format!("cannot locate kess: {error}"))?;
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .args(["--format", "json"])
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot start kess: {error}"))?;
    if let Some(input) = stdin {
        let mut pipe = child.stdin.take().ok_or("cannot open kess stdin")?;
        pipe.write_all(input.as_bytes())
            .map_err(|error| format!("cannot write circuit to kess: {error}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("kess did not finish: {error}"))?;
    Ok(CliOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// Resolves `source`/`path` into circuit text plus the `--input` language argument.
fn circuit_source(arguments: &Value) -> Result<(String, String), String> {
    let mut input = arguments
        .get("input")
        .and_then(Value::as_str)
        .unwrap_or("auto")
        .to_string();
    if !matches!(input.as_str(), "auto" | "kess" | "spice") {
        return Err(format!("input must be auto, kess or spice, not '{input}'"));
    }
    if arguments.get("source").and_then(Value::as_str).is_some()
        && arguments.get("path").and_then(Value::as_str).is_some()
    {
        return Err("provide either 'source' or 'path', not both".into());
    }
    if let Some(source) = arguments.get("source").and_then(Value::as_str) {
        return Ok((source.to_string(), input));
    }
    if let Some(path) = arguments.get("path").and_then(Value::as_str) {
        let text =
            fs::read_to_string(path).map_err(|error| format!("cannot read '{path}': {error}"))?;
        if input == "auto" {
            let extension = Path::new(path)
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase);
            input = match extension.as_deref() {
                Some("kess") => "kess".into(),
                Some("cir" | "sp" | "spice" | "net" | "ckt" | "cki") => "spice".into(),
                _ => "auto".into(),
            };
        }
        return Ok((text, input));
    }
    Err("provide 'source' (circuit text) or 'path'".into())
}

fn base_arguments(command: &str, arguments: &Value, input: &str) -> Vec<String> {
    let mut args = vec![
        command.to_string(),
        "-".to_string(),
        "--input".to_string(),
        input.to_string(),
    ];
    if let Some(parameters) = arguments.get("parameters").and_then(Value::as_object) {
        for (name, value) in parameters {
            let value = value
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| value.to_string());
            args.push("--param".into());
            args.push(format!("{name}={value}"));
        }
    }
    args
}

fn circuit_command(
    command: &str,
    arguments: &Value,
    extra: &[String],
) -> Result<CliOutput, String> {
    let (source, input) = circuit_source(arguments)?;
    let mut args = base_arguments(command, arguments, &input);
    args.extend(extra.iter().cloned());
    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_cli(&refs, Some(&source))
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Result<Self, String> {
        let path = std::env::temp_dir().join(format!(
            "kessetsu-mcp-{tag}-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).map_err(|error| format!("cannot create temp dir: {error}"))?;
        Ok(Self(path))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn is_inline_assert_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed
        .strip_prefix("assert")
        .is_some_and(|rest| rest.chars().next().is_some_and(char::is_whitespace))
}

fn strip_inline_assert_lines(source: &str) -> String {
    let ends_with_newline = source.ends_with('\n');
    let kept: Vec<&str> = source
        .lines()
        .filter(|line| !is_inline_assert_line(line))
        .collect();
    let mut result = kept.join("\n");
    if ends_with_newline && !result.is_empty() {
        result.push('\n');
    }
    result
}

fn test_command(arguments: &Value) -> Result<CliOutput, String> {
    if arguments
        .get("requirements")
        .and_then(Value::as_str)
        .is_some()
        && arguments
            .get("requirements_path")
            .and_then(Value::as_str)
            .is_some()
    {
        return Err("provide either 'requirements' or 'requirements_path', not both".into());
    }
    let temp = TempDir::new("test")?;
    let mut extra = Vec::new();
    let requirements_path = if let Some(text) =
        arguments.get("requirements").and_then(Value::as_str)
    {
        let path = temp.0.join("requirements.kessreq");
        fs::write(&path, text).map_err(|error| format!("cannot stage requirements: {error}"))?;
        Some(path)
    } else {
        arguments
            .get("requirements_path")
            .and_then(Value::as_str)
            .map(PathBuf::from)
    };
    let has_external_requirements = requirements_path.is_some();
    if let Some(path) = requirements_path {
        extra.push("--requirements".to_string());
        extra.push(path.display().to_string());
        if let Some(hash) = arguments.get("requirements_sha256").and_then(Value::as_str) {
            extra.push("--requirements-sha256".to_string());
            extra.push(hash.to_string());
        }
    }
    let (mut source, input) = circuit_source(arguments)?;
    let looks_like_spice = input == "spice"
        || (input == "auto" && source.lines().any(|l| l.trim_start().starts_with('.')));
    if has_external_requirements && !looks_like_spice {
        source = strip_inline_assert_lines(&source);
    }
    let mut args = base_arguments("test", arguments, &input);
    args.extend(extra);
    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_cli(&refs, Some(&source))
}

type FileCommandOutput = (CliOutput, Option<Vec<u8>>, Option<String>);

fn file_command(
    command: &str,
    arguments: &Value,
    extension: &str,
    mut extra: Vec<String>,
) -> Result<FileCommandOutput, String> {
    let (source, input) = circuit_source(arguments)?;
    let temp = TempDir::new("file")?;
    let staged = temp.0.join(format!("output.{extension}"));
    let mut args = base_arguments(command, arguments, &input);
    args.push("--output".into());
    args.push(staged.display().to_string());
    args.append(&mut extra);
    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let output = run_cli(&refs, Some(&source))?;
    let bytes = fs::read(&staged).ok();
    let mut saved = None;
    if let (Some(bytes), Some(target)) = (
        bytes.as_ref(),
        arguments.get("output_path").and_then(Value::as_str),
    ) {
        let target = Path::new(target);
        let overwrite = arguments
            .get("overwrite")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if target.exists() && !overwrite {
            return Err(format!(
                "'{}' already exists; pass overwrite=true to replace it",
                target.display()
            ));
        }
        fs::write(target, bytes)
            .map_err(|error| format!("cannot save '{}': {error}", target.display()))?;
        saved = Some(target.display().to_string());
    }
    Ok((output, bytes, saved))
}

fn render_command(arguments: &Value) -> Result<Value, (i64, String)> {
    let format = arguments
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("png");
    if !matches!(format, "png" | "svg") {
        return Err((-32602, "format must be png or svg".into()));
    }
    let extension = arguments
        .get("output_path")
        .and_then(Value::as_str)
        .and_then(|path| Path::new(path).extension().and_then(|value| value.to_str()))
        .filter(|extension| matches!(*extension, "png" | "svg" | "pdf"))
        .unwrap_or(format)
        .to_string();
    let (output, bytes, saved) = match file_command("render", arguments, &extension, Vec::new()) {
        Ok(result) => result,
        Err(message) => return Ok(text_result(message, true)),
    };
    let Some(bytes) = bytes else {
        return Ok(cli_result(output));
    };
    let note = saved
        .map(|path| format!("Schematic saved to {path}."))
        .unwrap_or_else(|| "Schematic rendered.".into());
    let content = match extension.as_str() {
        "png" => json!([
            { "type": "text", "text": note },
            { "type": "image", "data": base64(&bytes), "mimeType": "image/png" }
        ]),
        "svg" => json!([
            { "type": "text", "text": note },
            { "type": "text", "text": String::from_utf8_lossy(&bytes) }
        ]),
        _ => json!([{ "type": "text", "text": note }]),
    };
    Ok(json!({ "content": content, "isError": false }))
}

fn export_command(arguments: &Value) -> Result<Value, (i64, String)> {
    let Some(target) = arguments.get("target").and_then(Value::as_str) else {
        return Err((-32602, "target is required".into()));
    };
    let extension = match target {
        "kicad" => "kicad_sch",
        "ltspice" => "asc",
        "bom-csv" => "csv",
        "spice" => "spice",
        "schematic-json" | "handoff-json" => "json",
        _ => return Err((-32602, format!("unsupported export target '{target}'"))),
    };
    let extra = vec!["--target".to_string(), target.to_string()];
    let (output, bytes, saved) = match file_command("export", arguments, extension, extra) {
        Ok(result) => result,
        Err(message) => return Ok(text_result(message, true)),
    };
    let Some(bytes) = bytes else {
        return Ok(cli_result(output));
    };
    let note = saved
        .map(|path| format!("Exported {target} to {path}."))
        .unwrap_or_else(|| format!("Exported {target}."));
    Ok(json!({
        "content": [
            { "type": "text", "text": note },
            { "type": "text", "text": String::from_utf8_lossy(&bytes) }
        ],
        "isError": false
    }))
}

fn import_command(arguments: &Value) -> Value {
    if arguments.get("netlist").and_then(Value::as_str).is_some()
        && arguments.get("path").and_then(Value::as_str).is_some()
    {
        return text_result("provide either 'netlist' or 'path', not both".into(), true);
    }
    let netlist = if let Some(text) = arguments.get("netlist").and_then(Value::as_str) {
        text.to_string()
    } else if let Some(path) = arguments.get("path").and_then(Value::as_str) {
        match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => return text_result(format!("cannot read '{path}': {error}"), true),
        }
    } else {
        return text_result("provide 'netlist' or 'path'".into(), true);
    };

    let report = kessetsu_core::spice_import::import_spice(&netlist);
    let is_error = report.has_errors();
    let structured = match serde_json::to_value(&report) {
        Ok(value) => value,
        Err(error) => {
            return text_result(
                format!("failed to serialize SPICE import report: {error}"),
                true,
            );
        }
    };
    let text = serde_json::to_string_pretty(&structured).unwrap_or_default();
    json!({
        "content": [ { "type": "text", "text": text } ],
        "structuredContent": structured,
        "isError": is_error
    })
}

fn share_link_command(arguments: &Value) -> Value {
    let mut normalized = arguments.clone();
    if let (Some(input), Some(input_format)) = (
        arguments.get("input").and_then(Value::as_str),
        arguments.get("input_format").and_then(Value::as_str),
    ) && input != input_format
    {
        return text_result(
            format!("conflicting 'input' ('{input}') and 'input_format' ('{input_format}')"),
            true,
        );
    }
    if arguments.get("input").is_none()
        && let Some(input_format) = arguments.get("input_format").and_then(Value::as_str)
        && let Some(obj) = normalized.as_object_mut()
    {
        obj.insert("input".to_string(), Value::String(input_format.to_string()));
    }
    if let Some(name_value) = arguments.get("name")
        && !name_value.is_null()
        && !name_value.is_string()
    {
        return text_result("'name' must be a string".into(), true);
    }
    let name = arguments.get("name").and_then(Value::as_str);

    let (source, input) = match circuit_source(&normalized) {
        Ok(pair) => pair,
        Err(message) => return text_result(message, true),
    };
    let looks_like_spice = input == "spice"
        || (input == "auto"
            && source
                .lines()
                .any(|line| line.trim_start().starts_with('.')));
    let kess_source = if looks_like_spice {
        let imported = kessetsu_core::spice_import::import_spice(&source);
        match imported.kess_source {
            Some(kess) if !imported.has_errors() => kess,
            _ => {
                let messages = imported
                    .diagnostics
                    .iter()
                    .map(|diag| format!("{}: {}", diag.code, diag.message))
                    .collect::<Vec<_>>()
                    .join("; ");
                let detail = if messages.is_empty() {
                    "SPICE input could not be converted into canonical Kessetsu source".to_string()
                } else {
                    format!("SPICE input failed conversion: {messages}")
                };
                return text_result(detail, true);
            }
        }
    } else {
        source
    };

    let mut inputs = kessetsu_core::compile_inputs::CompileInputs::default();
    if let Some(parameters) = arguments.get("parameters").and_then(Value::as_object) {
        for (param_name, param_value) in parameters {
            let value_str = param_value
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| param_value.to_string());
            inputs
                .parameters
                .push(kessetsu_core::compile_inputs::ParameterInput {
                    name: param_name.clone(),
                    value: value_str,
                });
        }
    }
    let (source_path, source_is_stdin) = match arguments.get("path").and_then(Value::as_str) {
        Some(path) => (Path::new(path), false),
        None => (Path::new("-"), true),
    };
    let external_resources =
        match crate::load_external_model_resources(source_path, source_is_stdin, &kess_source) {
            Ok(resources) => resources,
            Err(diag) => {
                return text_result(format!("{}: {}", diag.code, diag.message), true);
            }
        };
    let report = kessetsu_core::compiler::compile_source_with_inputs(
        &kess_source,
        kessetsu_core::compiler::CompileOptions::default(),
        &inputs,
        &external_resources,
    );
    if report.has_errors() {
        let messages = report
            .diagnostics
            .iter()
            .filter(|diag| {
                matches!(
                    diag.severity,
                    kessetsu_core::compiler::DiagnosticSeverity::Error
                )
            })
            .map(|diag| format!("{}: {}", diag.code, diag.message))
            .collect::<Vec<_>>()
            .join("; ");
        return text_result(format!("circuit failed validation: {messages}"), true);
    }

    let manifest = report.ir.as_ref().map(|ir| &ir.model_manifest);
    let share_report = match kessetsu_core::share::encode_share_url(
        &kess_source,
        kessetsu_core::compiler::COMPILE_SCHEMA_VERSION,
        manifest,
        name,
    ) {
        Ok(report) => report,
        Err(message) => return text_result(message, true),
    };
    let structured = match serde_json::to_value(&share_report) {
        Ok(value) => value,
        Err(error) => {
            return text_result(
                format!("failed to serialize share link report: {error}"),
                true,
            );
        }
    };
    let text = serde_json::to_string_pretty(&structured).unwrap_or_default();
    json!({
        "content": [ { "type": "text", "text": text } ],
        "structuredContent": structured,
        "isError": false
    })
}

fn tool_command(arguments: &Value) -> Result<CliOutput, String> {
    let tool = arguments
        .get("tool")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "tool is required ('divider', 'rc_lowpass', 'non_inverting_opamp', or 'zener_regulator')"
                .to_string()
        })?;

    let values = arguments
        .get("values")
        .and_then(Value::as_str)
        .unwrap_or("e24");
    if !matches!(values, "exact" | "e12" | "e24") {
        return Err(format!("values must be exact, e12, or e24, not '{values}'"));
    }

    let reject_keys = |tool_name: &str, forbidden: &[&str]| -> Result<(), String> {
        let present = forbidden
            .iter()
            .copied()
            .filter(|key| arguments.get(*key).is_some_and(|v| !v.is_null()))
            .collect::<Vec<_>>();
        if present.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{} is not valid parameter(s) for {tool_name}",
                present.join(", ")
            ))
        }
    };

    let mut args = vec!["tool".to_string()];
    match tool {
        "divider" => {
            reject_keys(
                "divider",
                &[
                    "cutoff",
                    "resistance",
                    "target_gain",
                    "input_resistance",
                    "input_voltage",
                    "supply_voltage",
                    "zener_voltage",
                    "load_current",
                    "zener_knee_current",
                ],
            )?;
            let vin = arguments
                .get("vin")
                .and_then(Value::as_str)
                .ok_or_else(|| "divider requires 'vin'".to_string())?;
            let target = arguments
                .get("target")
                .and_then(Value::as_str)
                .ok_or_else(|| "divider requires 'target'".to_string())?;
            let lower = arguments
                .get("lower")
                .and_then(Value::as_str)
                .ok_or_else(|| "divider requires 'lower'".to_string())?;

            args.push("divider".to_string());
            args.push("--vin".to_string());
            args.push(vin.to_string());
            args.push("--target".to_string());
            args.push(target.to_string());
            args.push("--lower".to_string());
            args.push(lower.to_string());

            if let Some(load) = arguments.get("load").and_then(Value::as_str) {
                args.push("--load".to_string());
                args.push(load.to_string());
            }
        }
        "rc_lowpass" | "rc-lowpass" => {
            reject_keys(
                "rc_lowpass",
                &[
                    "vin",
                    "target",
                    "lower",
                    "load",
                    "target_gain",
                    "input_resistance",
                    "input_voltage",
                    "supply_voltage",
                    "zener_voltage",
                    "load_current",
                    "zener_knee_current",
                ],
            )?;
            let cutoff = arguments
                .get("cutoff")
                .and_then(Value::as_str)
                .ok_or_else(|| "rc_lowpass requires 'cutoff'".to_string())?;
            let resistance = arguments
                .get("resistance")
                .and_then(Value::as_str)
                .ok_or_else(|| "rc_lowpass requires 'resistance'".to_string())?;

            args.push("rc-lowpass".to_string());
            args.push("--cutoff".to_string());
            args.push(cutoff.to_string());
            args.push("--resistance".to_string());
            args.push(resistance.to_string());
        }
        "non_inverting_opamp" | "non-inverting-opamp" | "opamp_gain" | "opamp-gain" => {
            reject_keys(
                "non_inverting_opamp",
                &[
                    "cutoff",
                    "target",
                    "load",
                    "zener_voltage",
                    "load_current",
                    "zener_knee_current",
                ],
            )?;
            if arguments.get("input_voltage").is_some_and(|v| !v.is_null())
                && arguments.get("vin").is_some_and(|v| !v.is_null())
            {
                return Err(
                    "provide either 'input_voltage' or 'vin' for non_inverting_opamp, not both"
                        .into(),
                );
            }
            let target_gain = arguments
                .get("target_gain")
                .and_then(Value::as_str)
                .ok_or_else(|| "non_inverting_opamp requires 'target_gain'".to_string())?;
            let input_resistance = arguments
                .get("input_resistance")
                .or_else(|| arguments.get("lower"))
                .or_else(|| arguments.get("resistance"))
                .and_then(Value::as_str)
                .ok_or_else(|| "non_inverting_opamp requires 'input_resistance'".to_string())?;

            args.push("non-inverting-opamp".to_string());
            args.push("--target-gain".to_string());
            args.push(target_gain.to_string());
            args.push("--input-resistance".to_string());
            args.push(input_resistance.to_string());

            if let Some(vin) = arguments
                .get("input_voltage")
                .or_else(|| arguments.get("vin"))
                .and_then(Value::as_str)
            {
                args.push("--input-voltage".to_string());
                args.push(vin.to_string());
            }
            if let Some(supply) = arguments.get("supply_voltage").and_then(Value::as_str) {
                args.push("--supply-voltage".to_string());
                args.push(supply.to_string());
            }
        }
        "zener_regulator" | "zener-regulator" => {
            reject_keys(
                "zener_regulator",
                &[
                    "cutoff",
                    "resistance",
                    "target",
                    "lower",
                    "load",
                    "target_gain",
                    "input_resistance",
                    "supply_voltage",
                ],
            )?;
            if arguments.get("input_voltage").is_some_and(|v| !v.is_null())
                && arguments.get("vin").is_some_and(|v| !v.is_null())
            {
                return Err(
                    "provide either 'input_voltage' or 'vin' for zener_regulator, not both".into(),
                );
            }
            let vin = arguments
                .get("input_voltage")
                .or_else(|| arguments.get("vin"))
                .and_then(Value::as_str)
                .ok_or_else(|| "zener_regulator requires 'input_voltage' (or 'vin')".to_string())?;
            let vz = arguments
                .get("zener_voltage")
                .and_then(Value::as_str)
                .ok_or_else(|| "zener_regulator requires 'zener_voltage'".to_string())?;
            let iload = arguments
                .get("load_current")
                .and_then(Value::as_str)
                .ok_or_else(|| "zener_regulator requires 'load_current'".to_string())?;

            args.push("zener-regulator".to_string());
            args.push("--input-voltage".to_string());
            args.push(vin.to_string());
            args.push("--zener-voltage".to_string());
            args.push(vz.to_string());
            args.push("--load-current".to_string());
            args.push(iload.to_string());

            if let Some(iz_min) = arguments.get("zener_knee_current").and_then(Value::as_str) {
                args.push("--zener-knee-current".to_string());
                args.push(iz_min.to_string());
            }
        }
        _ => {
            return Err(format!(
                "unknown tool '{tool}', expected divider, rc_lowpass, non_inverting_opamp, or zener_regulator"
            ));
        }
    }

    args.push("--values".to_string());
    args.push(values.to_string());

    if let Some(output_path) = arguments.get("output_path").and_then(Value::as_str) {
        args.push("--output".to_string());
        args.push(output_path.to_string());
    }

    if arguments
        .get("overwrite")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        args.push("--force".to_string());
    }

    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_cli(&refs, None)
}

fn study_plan_command(arguments: &Value) -> Result<CliOutput, String> {
    let has_spec = arguments.get("spec").is_some() && !arguments["spec"].is_null();
    let has_spec_path = arguments.get("spec_path").and_then(Value::as_str).is_some();
    let has_source = arguments.get("source").and_then(Value::as_str).is_some();
    let has_path = arguments.get("path").and_then(Value::as_str).is_some();

    let count = [has_spec, has_spec_path, has_source, has_path]
        .iter()
        .filter(|&&b| b)
        .count();
    if count == 0 {
        return Err("provide one of 'spec', 'spec_path', 'source', or 'path'".into());
    }
    if count > 1 {
        return Err("provide only one of 'spec', 'spec_path', 'source', or 'path'".into());
    }

    if (has_spec || has_spec_path) && arguments.get("sweep").is_some() {
        return Err("sweep is only supported with 'source' or 'path'".into());
    }

    let output_path = arguments.get("output_path").and_then(Value::as_str);
    let overwrite = arguments
        .get("overwrite")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let temp = TempDir::new("study-plan")?;

    let spec_file = if has_source || has_path {
        let circuit_path = if has_source {
            let p = temp.0.join("circuit.kess");
            let source_str = arguments["source"].as_str().unwrap();
            fs::write(&p, source_str)
                .map_err(|error| format!("cannot stage circuit source: {error}"))?;
            p
        } else {
            PathBuf::from(arguments["path"].as_str().unwrap())
        };

        let spec_dest = if let Some(out) = output_path {
            let p = PathBuf::from(out);
            if p.exists() && !overwrite {
                return Err(format!(
                    "'{}' already exists; pass overwrite=true to replace it",
                    p.display()
                ));
            }
            p
        } else {
            temp.0.join("study.kessstudy.json")
        };

        let circuit_str = circuit_path.display().to_string();
        let spec_dest_str = spec_dest.display().to_string();
        let mut create_args = vec![
            "study".to_string(),
            "create".to_string(),
            circuit_str,
            "--output".to_string(),
            spec_dest_str,
        ];
        if overwrite || output_path.is_none() {
            create_args.push("--force".to_string());
        }
        if let Some(sweeps) = arguments.get("sweep").and_then(Value::as_array) {
            for sweep in sweeps {
                if let Some(s) = sweep.as_str() {
                    create_args.push("--sweep".to_string());
                    create_args.push(s.to_string());
                }
            }
        }
        let refs = create_args.iter().map(String::as_str).collect::<Vec<_>>();
        let create_out = run_cli(&refs, None)?;
        if create_out.exit_code != 0 {
            return Ok(create_out);
        }
        spec_dest
    } else if has_spec {
        let spec_val = &arguments["spec"];
        let spec_string = match spec_val {
            Value::String(s) => s.clone(),
            Value::Object(_) => serde_json::to_string_pretty(spec_val)
                .map_err(|error| format!("cannot serialize spec: {error}"))?,
            _ => return Err("spec must be a JSON object or JSON string".into()),
        };

        if let Some(out) = output_path {
            let p = PathBuf::from(out);
            if p.exists() && !overwrite {
                return Err(format!(
                    "'{}' already exists; pass overwrite=true to replace it",
                    p.display()
                ));
            }
            fs::write(&p, &spec_string)
                .map_err(|error| format!("cannot save spec to '{}': {error}", p.display()))?;
            p
        } else {
            let p = temp.0.join("study.kessstudy.json");
            fs::write(&p, &spec_string).map_err(|error| format!("cannot stage spec: {error}"))?;
            p
        }
    } else {
        let p = PathBuf::from(arguments["spec_path"].as_str().unwrap());
        if !p.exists() {
            return Err(format!("spec_path '{}' does not exist", p.display()));
        }
        if let Some(out) = output_path {
            let out_path = PathBuf::from(out);
            if out_path.exists() && !overwrite {
                return Err(format!(
                    "'{}' already exists; pass overwrite=true to replace it",
                    out_path.display()
                ));
            }
            fs::copy(&p, &out_path).map_err(|error| {
                format!("cannot copy spec to '{}': {error}", out_path.display())
            })?;
            out_path
        } else {
            p
        }
    };

    let spec_str = spec_file.display().to_string();
    let plan_args = ["study", "plan", &spec_str];
    run_cli(&plan_args, None)
}

fn study_run_command(arguments: &Value) -> Result<CliOutput, String> {
    let has_spec = arguments.get("spec").is_some() && !arguments["spec"].is_null();
    let has_spec_path = arguments.get("spec_path").and_then(Value::as_str).is_some();
    let has_source = arguments.get("source").and_then(Value::as_str).is_some();
    let has_path = arguments.get("path").and_then(Value::as_str).is_some();

    let count = [has_spec, has_spec_path, has_source, has_path]
        .iter()
        .filter(|&&b| b)
        .count();
    if count == 0 {
        return Err("provide one of 'spec', 'spec_path', 'source', or 'path'".into());
    }
    if count > 1 {
        return Err("provide only one of 'spec', 'spec_path', 'source', or 'path'".into());
    }

    if (has_spec || has_spec_path) && arguments.get("sweep").is_some() {
        return Err("sweep is only supported with 'source' or 'path'".into());
    }

    let resume = arguments
        .get("resume")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let overwrite = arguments
        .get("overwrite")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let output_path = arguments.get("output_path").and_then(Value::as_str);

    if resume && output_path.is_none() {
        return Err("resume requires output_path".into());
    }
    if resume && overwrite {
        return Err("choose resume or overwrite, not both".into());
    }
    if let Some(out) = output_path {
        let out_path = PathBuf::from(out);
        if out_path.exists() && !overwrite && !resume {
            return Err(format!(
                "'{}' already exists; pass overwrite=true to replace it or resume=true to resume",
                out_path.display()
            ));
        }
    }

    let mut include_datasets = false;
    if let Some(include) = arguments.get("include").and_then(Value::as_array) {
        for item in include {
            match item.as_str() {
                Some("datasets") => include_datasets = true,
                Some(other) => {
                    return Err(format!(
                        "unsupported include '{other}'; only 'datasets' is supported"
                    ));
                }
                None => return Err("include items must be strings".into()),
            }
        }
    }

    let temp = TempDir::new("study-run")?;

    let spec_file = if has_source || has_path {
        let circuit_path = if has_source {
            let p = temp.0.join("circuit.kess");
            let source_str = arguments["source"].as_str().unwrap();
            fs::write(&p, source_str)
                .map_err(|error| format!("cannot stage circuit source: {error}"))?;
            p
        } else {
            PathBuf::from(arguments["path"].as_str().unwrap())
        };

        let staged_spec = temp.0.join("study.kessstudy.json");
        let circuit_str = circuit_path.display().to_string();
        let staged_spec_str = staged_spec.display().to_string();
        let mut create_args = vec![
            "study".to_string(),
            "create".to_string(),
            circuit_str,
            "--output".to_string(),
            staged_spec_str,
            "--force".to_string(),
        ];
        if let Some(sweeps) = arguments.get("sweep").and_then(Value::as_array) {
            for sweep in sweeps {
                if let Some(s) = sweep.as_str() {
                    create_args.push("--sweep".to_string());
                    create_args.push(s.to_string());
                }
            }
        }
        let refs = create_args.iter().map(String::as_str).collect::<Vec<_>>();
        let create_out = run_cli(&refs, None)?;
        if create_out.exit_code != 0 {
            return Ok(create_out);
        }
        staged_spec
    } else if has_spec {
        let spec_val = &arguments["spec"];
        let spec_string = match spec_val {
            Value::String(s) => s.clone(),
            Value::Object(_) => serde_json::to_string_pretty(spec_val)
                .map_err(|error| format!("cannot serialize spec: {error}"))?,
            _ => return Err("spec must be a JSON object or JSON string".into()),
        };
        let staged_spec = temp.0.join("study.kessstudy.json");
        fs::write(&staged_spec, &spec_string)
            .map_err(|error| format!("cannot stage spec: {error}"))?;
        staged_spec
    } else {
        let p = PathBuf::from(arguments["spec_path"].as_str().unwrap());
        if !p.exists() {
            return Err(format!("spec_path '{}' does not exist", p.display()));
        }
        p
    };

    let results_path = if let Some(out) = output_path {
        PathBuf::from(out)
    } else {
        temp.0.join("results.json")
    };

    let spec_str = spec_file.display().to_string();
    let results_str = results_path.display().to_string();
    let mut run_args = vec![
        "study".to_string(),
        "run".to_string(),
        spec_str,
        "--output".to_string(),
        results_str,
    ];
    if resume {
        run_args.push("--resume".to_string());
    } else if overwrite || output_path.is_none() {
        run_args.push("--force".to_string());
    }
    if include_datasets {
        run_args.push("--include".to_string());
        run_args.push("datasets".to_string());
    }

    let refs = run_args.iter().map(String::as_str).collect::<Vec<_>>();
    run_cli(&refs, None)
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let triple = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for index in 0..4 {
            if index <= chunk.len() {
                encoded.push(ALPHABET[((triple >> (18 - 6 * index)) & 0x3f) as usize] as char);
            } else {
                encoded.push('=');
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn notifications_receive_no_response() {
        let message = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(handle_message(&message).is_none());
    }

    #[test]
    fn initialize_negotiates_known_protocol() {
        let response = handle_message(&json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": { "protocolVersion": "2025-06-18" }
        }))
        .unwrap();
        assert_eq!(response["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(response["result"]["serverInfo"]["name"], "kessetsu");

        let modern = handle_message(&json!({
            "jsonrpc": "2.0", "id": 2, "method": "initialize",
            "params": { "protocolVersion": "2025-11-25" }
        }))
        .unwrap();
        assert_eq!(modern["result"]["protocolVersion"], "2025-11-25");

        let legacy_fallback = handle_message(&json!({
            "jsonrpc": "2.0", "id": 3, "method": "initialize",
            "params": { "protocolVersion": "2025-03-26" }
        }))
        .unwrap();
        assert_eq!(legacy_fallback["result"]["protocolVersion"], "2025-06-18");
    }

    #[test]
    fn tool_list_exposes_the_design_loop() {
        let names = tool_definitions()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        for expected in [
            "kessetsu_guide",
            "kessetsu_check",
            "kessetsu_test",
            "kessetsu_render_schematic",
            "kessetsu_export",
            "kessetsu_tool",
            "kessetsu_study_plan",
            "kessetsu_study_run",
        ] {
            assert!(names.iter().any(|name| name == expected), "{expected}");
        }
        let tool_def = tool_definitions()
            .into_iter()
            .find(|tool| tool["name"] == "kessetsu_tool")
            .expect("kessetsu_tool must exist");
        let tool_enum = tool_def["inputSchema"]["properties"]["tool"]["enum"]
            .as_array()
            .expect("tool enum")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();
        assert_eq!(
            tool_enum,
            vec![
                "divider",
                "rc_lowpass",
                "non_inverting_opamp",
                "zener_regulator"
            ]
        );
        let props = tool_def["inputSchema"]["properties"]
            .as_object()
            .expect("properties");
        for prop in [
            "target_gain",
            "input_resistance",
            "input_voltage",
            "supply_voltage",
            "zener_voltage",
            "load_current",
            "zener_knee_current",
        ] {
            assert!(
                props.contains_key(prop),
                "missing kessetsu_tool prop {prop}"
            );
        }

        assert!(names.iter().any(|name| name == "kessetsu_share_link"));
        let sim_def = tool_definitions()
            .into_iter()
            .find(|tool| tool["name"] == "kessetsu_simulate")
            .expect("kessetsu_simulate must exist");
        assert!(
            sim_def["inputSchema"]["properties"]
                .as_object()
                .unwrap()
                .contains_key("measure")
        );
    }

    #[test]
    fn share_link_and_import_spice_return_structured_reports() {
        let kess_src = "\
net GND\n\
net IN\n\
net OUT\n\
source VIN 10V\n\
resistor R1 1k\n\
resistor R2 1k\n\
connect VIN.plus, R1.p1 to IN\n\
connect R1.p2, R2.p1 to OUT\n\
connect VIN.minus, R2.p2 to GND\n\
simulate op\n";
        let share_res = handle_message(&json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "kessetsu_share_link",
                "arguments": {
                    "source": kess_src,
                    "name": "Divider",
                    "input_format": "kess"
                }
            }
        }))
        .unwrap();
        let structured = &share_res["result"]["structuredContent"];
        assert_eq!(share_res["result"]["isError"], false);
        assert_eq!(structured["schema_version"], "kessetsu.share_link.v1");
        assert_eq!(structured["name"], "Divider");
        let url = structured["url"].as_str().unwrap();
        assert!(url.starts_with("https://kessetsu.com/#kessetsu=1."));

        let spice = "Simple divider\nV1 in 0 DC 5\nR1 in out 1k\nR2 out 0 1k\n.op\n.end\n";
        let import_res = handle_message(&json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": {
                "name": "kessetsu_import_spice",
                "arguments": { "netlist": spice }
            }
        }))
        .unwrap();
        let imported = &import_res["result"]["structuredContent"];
        assert_eq!(import_res["result"]["isError"], false);
        assert!(imported["names"].is_array());
        assert!(!imported["names"].as_array().unwrap().is_empty());
        assert!(imported["summary"].is_object());
        assert!(imported["diagnostics"].is_array());
        assert!(
            imported["kess_source"]
                .as_str()
                .unwrap()
                .contains("resistor")
        );

        let spice_share = handle_message(&json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "tools/call",
            "params": {
                "name": "kessetsu_share_link",
                "arguments": {
                    "source": spice,
                    "input_format": "spice"
                }
            }
        }))
        .unwrap();
        assert_eq!(spice_share["result"]["isError"], false);
        assert!(
            spice_share["result"]["structuredContent"]["url"]
                .as_str()
                .unwrap()
                .starts_with("https://kessetsu.com/#kessetsu=1.")
        );
    }

    #[test]
    fn strip_inline_assert_lines_removes_calculator_asserts_cleanly() {
        let calc_res = kessetsu_core::tools::calculate_tool(
            kessetsu_core::tools::ToolRequest::NonInvertingOpamp {
                target_gain: "5".into(),
                input_resistance: "10k".into(),
                input_voltage: None,
                supply_voltage: None,
                preferred_values: kessetsu_core::tools::PreferredValues::E24,
            },
        )
        .unwrap();
        assert!(calc_res.source.contains("assert value(V(OUT))"));
        let stripped = strip_inline_assert_lines(&calc_res.source);
        assert!(!stripped.contains("assert "));
        assert!(stripped.contains("simulate ac"));
        let compiled = kessetsu_core::compiler::compile_source(
            &stripped,
            kessetsu_core::compiler::CompileOptions::default(),
        );
        assert!(!compiled.has_errors(), "{:?}", compiled.diagnostics);
        assert!(compiled.ir.unwrap().assertions.is_empty());
    }

    #[test]
    fn instructions_and_prompt_document_share_links_and_tools() {
        assert!(INSTRUCTIONS.contains("kessetsu_share_link"));
        assert!(INSTRUCTIONS.contains("https://kessetsu.com/#kessetsu=1"));
        assert!(INSTRUCTIONS.contains("kessetsu_tool"));
        assert!(INSTRUCTIONS.contains("kessetsu_import_spice"));
        let prompt = get_prompt(&json!({
            "name": "design_circuit",
            "arguments": { "goal": "1kHz low-pass" }
        }))
        .unwrap();
        let prompt_text = prompt["messages"][0]["content"]["text"].as_str().unwrap();
        assert!(prompt_text.contains("kessetsu_share_link"));
        assert!(prompt_text.contains("https://kessetsu.com/#kessetsu=1"));
    }
}
