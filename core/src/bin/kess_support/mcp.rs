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

const PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];
const GUIDE: &str = include_str!("../../../../docs/guides/agent-workflow.md");
const INSTRUCTIONS: &str = "Kessetsu verifies analog circuits. Workflow: (1) write measurable requirements as .kessreq assertion lines before designing; (2) write readable .kess source (or a supported SPICE netlist) with named nets; (3) kessetsu_check and fix ERC errors; (4) kessetsu_test with the unchanged requirements and iterate on the design only; (5) kessetsu_render_schematic for human review; (6) kessetsu_export for handoff; (7) report the PASS table and assumptions honestly. Call kessetsu_guide once for syntax and metrics.";

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
        .unwrap_or(PROTOCOL_VERSIONS[0]);
    let version = if PROTOCOL_VERSIONS.contains(&requested) {
        requested
    } else {
        PROTOCOL_VERSIONS[0]
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
        "inputSchema": { "type": "object", "properties": properties, "required": required },
    })
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "kessetsu_guide",
            "title": "Kessetsu design workflow and syntax",
            "description": "Returns the disciplined agent workflow, .kess syntax, .kessreq requirement syntax and metric list. Call once before designing.",
            "inputSchema": { "type": "object", "properties": {} },
        }),
        json!({
            "name": "kessetsu_capabilities",
            "title": "Installed Kessetsu capabilities",
            "description": "Versioned manifest of commands, contracts, metrics, exports and limits of this installed build.",
            "inputSchema": { "type": "object", "properties": {} },
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
            "Runs the declared analyses (op/tran/ac/dc) through Ngspice and returns typed results and any inline assertion outcomes.",
            json!({
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
                }
            },
        }),
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
        "Design this circuit with Kessetsu: {goal}\n\nFollow the Kessetsu workflow exactly:\n1. Call kessetsu_guide.\n2. Write measurable requirements (.kessreq) first and show them to me before designing; save them as requirements.kessreq.\n3. Write design.kess with named nets and a target comment.\n4. kessetsu_check, then kessetsu_test with the unchanged requirements; iterate on the design only.\n5. Render and save schematic.svg; export anything needed for handoff.\n6. Finish with NOTES.md: requirements, final PASS table, assumptions and open risks."
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
        "kessetsu_import_spice" => import_command(arguments),
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
    fn new() -> Result<Self, String> {
        let path = std::env::temp_dir().join(format!(
            "kessetsu-mcp-{}-{}",
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

fn test_command(arguments: &Value) -> Result<CliOutput, String> {
    let temp = TempDir::new()?;
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
    if let Some(path) = requirements_path {
        extra.push("--requirements".to_string());
        extra.push(path.display().to_string());
        if let Some(hash) = arguments.get("requirements_sha256").and_then(Value::as_str) {
            extra.push("--requirements-sha256".to_string());
            extra.push(hash.to_string());
        }
    }
    circuit_command("test", arguments, &extra)
}

type FileCommandOutput = (CliOutput, Option<Vec<u8>>, Option<String>);

fn file_command(
    command: &str,
    arguments: &Value,
    extension: &str,
    mut extra: Vec<String>,
) -> Result<FileCommandOutput, String> {
    let (source, input) = circuit_source(arguments)?;
    let temp = TempDir::new()?;
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

fn import_command(arguments: &Value) -> Result<CliOutput, String> {
    let netlist = if let Some(text) = arguments.get("netlist").and_then(Value::as_str) {
        text.to_string()
    } else if let Some(path) = arguments.get("path").and_then(Value::as_str) {
        fs::read_to_string(path).map_err(|error| format!("cannot read '{path}': {error}"))?
    } else {
        return Err("provide 'netlist' or 'path'".into());
    };
    // `check --include effective-source` returns the canonical .kess text without writing files.
    run_cli(
        &[
            "check",
            "-",
            "--input",
            "spice",
            "--include",
            "effective-source",
        ],
        Some(&netlist),
    )
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
            "params": { "protocolVersion": "2025-03-26" }
        }))
        .unwrap();
        assert_eq!(response["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(response["result"]["serverInfo"]["name"], "kessetsu");
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
        ] {
            assert!(names.iter().any(|name| name == expected), "{expected}");
        }
    }
}
